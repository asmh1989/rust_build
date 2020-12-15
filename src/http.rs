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
