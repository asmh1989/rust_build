use std::thread;

use actix_web::{web, Responder};
use log::{info, warn};
use serde_json::json;

use crate::{
    build_params::{AppParams, BuildParams},
    config::Config,
    http_response::response_ok,
    work,
};

pub struct MyRoute;
impl MyRoute {
    pub async fn build(params: web::Json<BuildParams>) -> impl Responder {
        let build_p = params.0;
        let app = AppParams::new(build_p, "");
        let id = app.build_id.clone();

        if !Config::is_building() {
            thread::spawn(move || {
                info!("start build {} ... ", app.build_id);
                Config::get_instance().lock().unwrap().set_building(true);
                match work::start(&app) {
                    Ok(_) => {
                        info!("{}  build finish ....", app.build_id)
                    }
                    Err(e) => {
                        warn!(
                            "{} error \n------------------------\n{}\n------------------------",
                            app.build_id, e
                        )
                    }
                }
                Config::get_instance().lock().unwrap().set_building(false);
            });
        } else {
            info!("waiting ....")
        }

        response_ok(json!({ "id": id }))
    }
}
