#![allow(dead_code)]

use std::{
    fs::{metadata, read_dir, remove_dir_all},
    io,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, UNIX_EPOCH},
};

use crate::redis::{Redis, BUILD_CHANNEL};
use actix_web::{
    error::InternalError, error::JsonPayloadError, get, middleware::Logger, web, App, Error,
    HttpRequest, HttpServer, Responder,
};
use args::Opt;
use bson::doc;
use build_params::{AppParams, CODE_BUILDING, CODE_WAITING};
use chrono::{DateTime, NaiveDateTime, Utc};
use db::{Db, COLLECTION_BUILD};
use http_response::*;
use log::info;
use mongodb::options::FindOptions;
use serde_json::Value;
use tokio::time::interval;

use structopt::StructOpt;

mod args;
mod build_params;
mod config;
mod db;
mod framework;
mod http;
mod http_response;
mod mail;
mod redis;
mod shell;
mod utils;
mod weed;
mod work;

#[get("/")]
async fn hello() -> impl Responder {
    response_ok(Value::String("hello world".to_string()))
}

fn post_error(err: JsonPayloadError, _: &HttpRequest) -> Error {
    let res = format!("{}", err);
    InternalError::from_response(err, response_error(res)).into()
}

fn clear_cache() -> io::Result<()> {
    let path = config::Config::cache_home() + "/apps";

    for entry in read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        let time = metadata(path.clone())?.modified()?;
        let time = time.duration_since(UNIX_EPOCH).unwrap();

        let time = DateTime::<Utc>::from_utc(
            NaiveDateTime::from_timestamp(time.as_secs() as i64, time.subsec_nanos()),
            Utc,
        );

        let duration = time.signed_duration_since(Utc::now());
        if duration.num_days().abs() > 2 {
            info!(
                " file = {:?} duration = {}  delete !",
                path.clone(),
                duration.num_days().abs()
            );

            remove_dir_all(path)?;
        }
    }

    Ok(())
}

fn time_work() {
    thread::spawn(|| {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let mut interval = interval(Duration::from_millis(8000));
            loop {
                interval.tick().await;

                if let Err(err) = clear_cache() {
                    info!(" clear cache error = {}", err);
                }

                let filter = doc! {"code":{"$gt": 1}};

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
                        if app.status.code == CODE_WAITING {
                            info!("found waiting work id = {} ", app.build_id);

                            Redis::publish(BUILD_CHANNEL, &app.build_id.to_string()).await;

                            continue;
                        } else if app.status.code == CODE_BUILDING {
                            let time = if app.update_time.is_none() {
                                app.date
                            } else {
                                app.update_time.unwrap()
                            };

                            let duration = time.signed_duration_since(chrono::Utc::now());

                            if duration.num_minutes().abs() > 20 {
                                info!(" exception building dur = {} ", duration);
                                Redis::publish(BUILD_CHANNEL, &app.build_id.to_string()).await;
                                continue;
                            }
                        }
                    }
                }
            }
        });
    });
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mut opt: Opt = Opt::from_args();

    config::Config::get_instance();

    if opt.ip.is_empty() {
        opt.ip = whoami::hostname();
    } else {
        config::Config::get_instance()
            .lock()
            .unwrap()
            .set_ip(&opt.ip);
    }

    if !opt.cache_path.is_empty() {
        config::Config::get_instance()
            .lock()
            .unwrap()
            .set_cache_home(&opt.cache_path);
    }

    if !opt.android_home.is_empty() {
        config::Config::get_instance()
            .lock()
            .unwrap()
            .set_android_home(&opt.android_home);
    }

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // 打印版本
    if opt.version {
        info!("{}", VERSION);
        return Ok(());
    }

    info!("{:#?}", opt);

    info!("start ...");

    db::init_db(&format!("mongodb://{}", opt.sql)).await;
    redis::init_redis(format!("redis://{}", opt.redis), !opt.disable_manager_build).await;

    time_work();

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::new("%U %s %D"))
            .service(hello)
            .service(
                web::resource("/app/build")
                    .data(web::JsonConfig::default().error_handler(post_error))
                    .route(web::post().to(http::MyRoute::build)),
            )
            .route("/app/query/{id}", web::get().to(http::MyRoute::query))
    })
    .bind(format!("127.0.0.1:{}", opt.port))?
    .run()
    .await
}
