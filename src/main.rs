#![allow(dead_code)]

use actix_web::{
    error::InternalError, error::JsonPayloadError, get, middleware::Logger, web, App, Error,
    HttpRequest, HttpResponse, HttpServer, Responder,
};
use http_response::*;
use log::info;
use serde_json::Value;

mod build_params;
mod config;
mod db;
mod framework;
mod http;
mod http_response;
mod shell;
mod utils;
mod weed;
mod work;

#[get("/")]
async fn hello() -> impl Responder {
    response_ok(Value::String("hello world".to_string()))
}

async fn manual_hello() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}

fn post_error(err: JsonPayloadError, _: &HttpRequest) -> Error {
    let res = format!("{}", err);
    InternalError::from_response(err, response_error(res)).into()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 修改config
    // config::Config::get_instance()
    // .lock()
    // .unwrap()
    // .set_cache_home("/tmp");

    config::Config::get_instance();

    info!("start ...");

    db::init_db("mongodb://192.168.2.36:27017").await;

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::new("%U %s %D"))
            .service(hello)
            .service(
                web::resource("/app/build")
                    .data(web::JsonConfig::default().error_handler(post_error))
                    .route(web::post().to(http::MyRoute::build)),
            )
            .route("/hey", web::get().to(manual_hello))
    })
    .bind("127.0.0.1:3771")?
    .run()
    .await
}
