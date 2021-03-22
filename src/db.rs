use std::{sync::Arc, time::Duration};

use bson::{Bson, Document};
use log::info;
use mongodb::{
    error::Error,
    options::{ClientOptions, FindOneOptions, FindOptions},
    Client,
};
use once_cell::sync::OnceCell;
use serde::de::DeserializeOwned;
use tokio_stream::StreamExt;

use std::result::Result;

#[macro_export]
macro_rules! filter_build_id {
    ($e:expr) => {
        bson::doc! {"build_id" : $e}
    };
}

#[derive(Clone, Debug)]
pub struct Db;

const TABLE_NAME: &'static str = "build_data";
pub const COLLECTION_BUILD: &'static str = "build";
const KEY_UPDATE_TIME: &'static str = "update_time";

static INSTANCE: OnceCell<Arc<Client>> = OnceCell::new();

impl Db {
    pub fn get_instance() -> &'static Arc<Client> {
        INSTANCE.get().expect("db need init first")
    }

    pub async fn find<T>(
        table: &str,
        filter: impl Into<Option<Document>>,
        options: impl Into<Option<FindOptions>>,
        call_back: &dyn Fn(T),
    ) -> Result<(), Error>
    where
        T: DeserializeOwned,
    {
        let client = Db::get_instance();
        let db = client.database(TABLE_NAME);
        let collection = db.collection(table);

        let mut cursor = collection.find(filter, options).await?;

        // Iterate over the results of the cursor.
        while let Some(result) = cursor.next().await {
            match result {
                Ok(document) => {
                    let result = bson::from_bson::<T>(Bson::Document(document));
                    match result {
                        Ok(app) => call_back(app),
                        Err(err) => {
                            info!("err = {:?}", err);
                        }
                    }
                }
                Err(e) => {
                    info!("error = {:?}", e);
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    pub async fn find_one(
        table: &str,
        filter: impl Into<Option<Document>>,
        options: impl Into<Option<FindOneOptions>>,
    ) -> Result<Option<Document>, Error> {
        let client = Db::get_instance();
        let db = client.database(TABLE_NAME);
        let collection = db.collection(table);

        collection.find_one(filter, options).await
    }

    pub async fn save(table: &str, filter: Document, app: Document) -> Result<(), Error> {
        let client = Db::get_instance();
        let db = client.database(TABLE_NAME);
        let collection = db.collection(table);

        let mut update_doc = app;
        update_doc.insert(KEY_UPDATE_TIME, Bson::DateTime(chrono::Utc::now()));

        let result = collection.find_one(filter.clone(), None).await?;

        if let Some(_) = result {
            // info!("db update");
            collection
                .update_one(filter.clone(), update_doc, None)
                .await?;
        } else {
            let result = collection.insert_one(update_doc, None).await?;

            info!("db insert {:?}", result);
        }

        Ok(())
    }

    pub async fn delete(table: &str, filter: Document) -> Result<(), Error> {
        let client = Db::get_instance();
        let db = client.database(TABLE_NAME);
        let collection = db.collection(table);

        let result = collection.delete_one(filter, None).await?;

        info!("db delete {:?}", result);

        Ok(())
    }

    pub async fn contians(table: &str, filter: Document) -> bool {
        let client = Db::get_instance();
        let db = client.database(TABLE_NAME);
        let collection = db.collection(table);

        let result = collection.find_one(filter, None).await;

        match result {
            Ok(d) => d.is_some(),
            Err(_) => false,
        }
    }
}

/// 初始化 数据库
pub async fn init_db(url: &str) {
    let mut client_options = ClientOptions::parse(url).await.unwrap();
    client_options.connect_timeout = Some(Duration::new(4, 0));
    // 选择超时
    client_options.server_selection_timeout = Some(Duration::new(8, 0));

    INSTANCE
        .set(Arc::new(Client::with_options(client_options).unwrap()))
        .expect("db init error");

    // unsafe {
    //     CLIENT_OPTIONS = Some(client_options.clone());

    //     // Rust中使用可变静态变量都是unsafe的
    //     DB.get_or_insert_with(|| {
    //         // 初始化单例对象的代码
    //         Arc::new(Mutex::new(Db {
    //             client: Client::with_options(client_options).unwrap(),
    //         }))
    //     });
    // }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use bson::doc;
    use log::info;

    use crate::build_params::AppParams;

    #[actix_rt::test]
    async fn test_remove_fid() {
        crate::config::Config::get_instance();

        super::init_db("mongodb://192.168.2.36:27017").await;

        let filter = doc! {"code": 0, "fid": {"$gt":""}};

        let find_options = FindOptions::builder()
            .sort(doc! { "date": -1 })
            .limit(Some(20))
            .build();

        let vec: Arc<Mutex<Vec<AppParams>>> = Arc::new(Mutex::new(Vec::new()));

        let result = Db::find(COLLECTION_BUILD, filter, find_options, &|app| {
            vec.lock().unwrap().push(app)
        })
        .await;

        if result.is_err() {
            info!("find error : {:?}", result.err());
        } else {
            for app in vec.lock().unwrap().iter() {
                let fid = app.fid.clone().unwrap();
                info!("delete  fid = {} ", fid.clone());
                let _ = crate::weed::delete(&fid).await;
            }
        }

        assert!(true);
    }

    #[actix_rt::test]
    async fn test_mongdb_find() {
        crate::config::Config::get_instance();

        super::init_db("mongodb://192.168.2.36:27017").await;
        info!("start find...");

        let filter = doc! {};

        let find_options = FindOneOptions::builder().sort(doc! { "date": -1 }).build();

        let result = Db::find_one(COLLECTION_BUILD, filter, find_options).await;
        let doc = match result {
            Ok(d) => d.unwrap(),
            Err(e) => {
                info!("err = {}", e);
                return assert!(false);
            }
        };

        let app = bson::from_bson::<AppParams>(Bson::Document(doc)).unwrap();

        info!("start delete...");
        Db::delete(
            super::COLLECTION_BUILD,
            filter_build_id!(app.build_id.to_string()),
        )
        .await
        .expect("delete error");

        assert!(!Db::contians(COLLECTION_BUILD, filter_build_id!(app.build_id.to_string())).await);

        let doc = match bson::to_bson(&app) {
            Ok(d) => d.as_document().unwrap().clone(),
            Err(e) => {
                info!("to_bson err {}", e);
                bson::doc! {}
            }
        };

        info!("start save1...");

        Db::save(
            COLLECTION_BUILD,
            filter_build_id!(app.build_id.to_string()),
            doc.clone(),
        )
        .await
        .expect("save error");

        assert!(Db::contians(COLLECTION_BUILD, filter_build_id!(app.build_id.to_string())).await);

        info!("start save2...");
        Db::save(
            COLLECTION_BUILD,
            filter_build_id!(app.build_id.to_string()),
            doc,
        )
        .await
        .expect("save error");

        let result = Db::find_one(
            COLLECTION_BUILD,
            filter_build_id!(app.build_id.to_string()),
            None,
        )
        .await;

        let doc2 = match result {
            Ok(d) => d.unwrap(),
            Err(e) => {
                info!("err = {}", e);
                return assert!(false);
            }
        };

        let app2 = bson::from_bson::<AppParams>(Bson::Document(doc2)).unwrap();

        assert_eq!(app.date, app2.date);
    }
}
