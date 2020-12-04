use std::thread;

use actix_web::{web, Responder};
use log::info;
use serde_json::json;

use crate::{
    build_params::{AppParams, BuildParams},
    config::Config,
    http_response::{response_error, response_ok},
    work,
};

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
}
