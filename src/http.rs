use std::thread;

use actix_web::{web, HttpResponse, Responder};
use bson::Bson;
use build_params::CODE_ILLEGAL;
use log::info;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    build_params::{self, AppParams, BuildParams, MSG_ILLEGAL},
    config::Config,
    db::{Db, COLLECTION_BUILD},
    filter_build_id, get_upload_url,
    http_response::{response_error, response_ok},
    work,
};

#[derive(Debug, Serialize, Deserialize)]
struct QueryResponse {
    pub status: i32,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "downloadPath")]
    pub download_path: Option<String>,
}
pub struct MyRoute;
impl MyRoute {
    pub async fn build(params: web::Json<BuildParams>) -> impl Responder {
        let build_p = params.0;
        let app = AppParams::new(build_p, "");
        let id = app.build_id.clone();

        if let Err(e) = app.save_db().await {
            return response_error(e);
        }

        if !Config::is_building() {
            Config::change_building(true);

            thread::spawn(|| {
                let mut rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async move {
                    work::start_build(app).await;
                });
            });
        } else {
            info!("waiting ....")
        }

        response_ok(json!({ "id": id }))
    }

    pub async fn query(web::Path(id): web::Path<String>) -> impl Responder {
        info!("query id {} ... ", id);

        let mut res = QueryResponse {
            status: CODE_ILLEGAL,
            msg: MSG_ILLEGAL.to_string(),
            detail: None,
            download_path: None,
        };

        let doc = filter_build_id!(id);

        if Db::contians(COLLECTION_BUILD, doc.clone()).await {
            let result = Db::find_one(COLLECTION_BUILD, doc, None).await.unwrap();

            match result {
                Some(doc) => {
                    let result = bson::from_bson::<AppParams>(Bson::Document(doc));
                    match result {
                        Ok(app) => {
                            res.status = app.status.code;
                            res.msg = if app.status.is_success() {
                                let mut fid = app.fid.clone();
                                res.download_path = Some(
                                    get_upload_url!(fid.get_or_insert("".to_string())).to_string(),
                                );
                                "打包成功".to_string()
                            } else {
                                res.detail = Some(app.status.msg.clone());
                                "打包失败".to_string()
                            };
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
}
