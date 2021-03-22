use std::sync::{Arc, Mutex};

use actix_web::{
    web::{self},
    HttpResponse, Responder,
};
use bson::{doc, Bson};
use build_params::{AppParams2, CODE_ILLEGAL};
use log::info;
use mongodb::options::FindOptions;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    build_params::{self, AppParams, BuildParams, MSG_ILLEGAL},
    config::Config,
    db::{Db, COLLECTION_BUILD},
    filter_build_id, get_upload_url,
    http_response::{response_error, response_ok},
    redis::BUILD_CHANNEL,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResponse {
    pub status: i32,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "downloadPath")]
    pub download_path: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct QueryInfo {
    pub status: Option<i64>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

impl QueryResponse {
    pub fn new() -> Self {
        QueryResponse {
            status: CODE_ILLEGAL,
            msg: MSG_ILLEGAL.to_string(),
            detail: None,
            download_path: None,
        }
    }

    pub fn to_response(&mut self, app: &AppParams) {
        self.status = app.status.code;
        self.msg = if app.status.is_success() {
            self.download_path = Some(format!("/app/package/{}.apk", app.build_id.clone()));
            "打包成功".to_string()
        } else {
            self.detail = Some(app.status.msg.clone());
            "打包失败".to_string()
        };
    }
}

pub struct MyRoute;
impl MyRoute {
    pub async fn build(params: web::Json<BuildParams>) -> impl Responder {
        let build_p = params.0;
        let email = build_p.email.clone();
        let app = AppParams::new(build_p, &Config::ip(), email);
        let id = app.build_id.clone();

        if let Err(e) = app.save_db().await {
            return response_error(e);
        }

        crate::redis::Redis::publish(BUILD_CHANNEL, &id.to_string()).await;

        response_ok(json!({ "id": id }))
    }

    pub async fn querys(info: web::Query<QueryInfo>) -> impl Responder {
        info!("querys info {:?} ... ", info);
        let page = info.page.unwrap_or(0);
        let page_size = info.page_size.unwrap_or(20);
        let status = info.status;

        let find_options = FindOptions::builder()
            .sort(doc! { "date": -1 })
            .limit(page_size)
            .skip(page * page_size)
            .build();

        let vec: Arc<Mutex<Vec<AppParams2>>> = Arc::new(Mutex::new(Vec::new()));

        let result = Db::find(
            COLLECTION_BUILD,
            status.map(|f| doc! {"code":{"$eq": f}}),
            find_options,
            &|app| vec.lock().unwrap().push(app),
        )
        .await;

        if result.is_err() {
            response_error(format!("{:?}", result.err()))
        } else {
            let mut data = Vec::new();
            for app in vec.lock().unwrap().iter() {
                data.push(serde_json::to_value(app).unwrap())
            }
            let v = serde_json::to_value(data);

            response_ok(v.unwrap())
        }
    }

    pub async fn query(web::Path(id): web::Path<String>) -> impl Responder {
        info!("query id {} ... ", id);

        let mut res = QueryResponse::new();

        let doc = filter_build_id!(id.clone());

        if Db::contians(COLLECTION_BUILD, doc.clone()).await {
            let result = Db::find_one(COLLECTION_BUILD, doc, None).await.unwrap();

            match result {
                Some(doc) => {
                    let result = bson::from_bson::<AppParams>(Bson::Document(doc));
                    match result {
                        Ok(app) => {
                            res.to_response(&app);
                        }
                        Err(err) => {
                            info!("{}", err);
                            res.msg = format!("{:?}", err);
                        }
                    }
                }
                None => {
                    res.msg = "no document".to_string();
                }
            }
        }

        HttpResponse::Ok()
            .content_type("application/json")
            .body(serde_json::to_string(&res).unwrap())
    }

    pub async fn package(web::Path(id): web::Path<String>) -> impl Responder {
        info!("package id {} ... ", id);

        let doc = filter_build_id!(id.clone());

        let result = Db::find_one(COLLECTION_BUILD, doc, None).await;
        match result {
            Ok(oo) => match oo {
                Some(doc) => {
                    let result = bson::from_bson::<AppParams>(Bson::Document(doc));
                    match result {
                        Ok(app) => {
                            return HttpResponse::PermanentRedirect()
                                .header("Location", get_upload_url!(&app.fid.unwrap()))
                                .finish()
                        }
                        Err(_) => {}
                    }
                }
                None => {}
            },
            Err(err) => {
                info!("{}", err);
            }
        }

        HttpResponse::Ok()
            .content_type("application/json")
            .body(serde_json::to_string(&QueryResponse::new()).unwrap())
    }
}
