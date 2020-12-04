use actix_rt::spawn;
use actix_web::{web, Responder};
use log::{info, warn};
use serde_json::json;

use crate::{
    build_params,
    build_params::{AppParams, BuildParams},
    config::Config,
    http_response::{response_error, response_ok},
    work,
};

pub struct MyRoute;
impl MyRoute {
    pub async fn build(params: web::Json<BuildParams>) -> impl Responder {
        let build_p = params.0;
        let mut app = AppParams::new(build_p, "");
        let id = app.build_id.clone();

        if let Err(e) = app.save_db().await {
            return response_error(e);
        }

        if !Config::is_building() {
            spawn(async move {
                info!("start build {} ... ", app.build_id);
                Config::get_instance().lock().unwrap().set_building(true);
                app.status = build_params::BuildStatus::building();
                if let Err(e) = app.save_db().await {
                    info!("{}", e);
                }
                match work::start(&app).await {
                    Ok(_) => {
                        info!("{}  build finish ....", app.build_id);

                        app.status = build_params::BuildStatus::success();
                        if let Err(e) = app.save_db().await {
                            info!("{}", e);
                        }
                    }
                    Err(e) => {
                        warn!(
                            "{} error \n------------------------\n{}\n------------------------",
                            app.build_id, e
                        );

                        app.status = build_params::BuildStatus::failed(e);
                        if let Err(err) = app.save_db().await {
                            info!("{}", err);
                        }
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
