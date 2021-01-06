use std::{sync::Arc, thread, time::Duration};

use log::{info, warn};
use once_cell::sync::OnceCell;
use redis::{aio::ConnectionManager, Msg, RedisResult};
use tokio_stream::StreamExt;

#[derive(Clone)]
pub struct Redis {
    con: ConnectionManager,
    value: String,
}

static RM: OnceCell<Arc<Redis>> = OnceCell::new();
pub const BUILD_CHANNEL: &'static str = "build_work";
const EXPIRE_TIME: i32 = 60 * 12;

impl Redis {
    fn get_instance() -> Option<Arc<Redis>> {
        let result = RM.get();
        match result {
            Some(r) => Some(r.clone()),
            None => None,
        }
    }

    pub async fn publish(channel: &str, msg: &str) {
        let result = Redis::get_instance();

        match result {
            Some(res) => {
                let mut con = res.con.clone();

                let result: RedisResult<()> = redis::cmd("PUBLISH")
                    .arg(channel)
                    .arg(msg)
                    .query_async(&mut con)
                    .await;

                info!(
                    "publish channel = {}, msg = {} enquene work, result = {:?}",
                    channel, msg, result
                );
            }
            None => {
                info!("publish error, redis not ready...");
            }
        }
    }

    pub async fn lock(key: &str) -> bool {
        Redis::lock_with_time(key, EXPIRE_TIME).await
    }

    pub async fn lock_with_time(key: &str, time: i32) -> bool {
        let result = Redis::get_instance();

        match result {
            Some(res) => {
                let value = res.value.clone();
                let mut con = res.con.clone();

                let result: RedisResult<i8> = redis::cmd("setnx")
                    .arg(key)
                    .arg(value)
                    .query_async(&mut con.clone())
                    .await;
                if let Ok(i) = result {
                    info!("lock {} = {} ", key, (i == 1));

                    if i == 1 {
                        let result: RedisResult<()> = redis::cmd("expire")
                            .arg(key)
                            .arg(time)
                            .query_async(&mut con)
                            .await;

                        if result.is_err() {
                            info!("expire error = {:?}", result.err());
                            return false;
                        }
                        return true;
                    }
                } else {
                    info!("setnx error = {:?}", result.err());
                }
            }
            None => {
                info!("lock error, redis not ready...");
            }
        }
        false
    }

    pub async fn unlock(key: &str) -> bool {
        let result = Redis::get_instance();

        match result {
            Some(res) => {
                let value = res.value.clone();
                let mut con = res.con.clone();

                let result: RedisResult<String> = redis::cmd("get")
                    .arg(key)
                    .query_async(&mut con.clone())
                    .await;
                if let Ok(i) = result {
                    if i == value {
                        let result: RedisResult<()> =
                            redis::cmd("del").arg(key).query_async(&mut con).await;

                        if result.is_err() {
                            info!("del error = {:?}", result.err());
                            return false;
                        }
                        return true;
                    } else {
                        info!("unlock error, can not unlock other server lock...");
                    }
                } else {
                    info!("unlock get error {:?}", result.err());
                }
            }
            None => {
                info!("unlock error, redis not ready...");
            }
        }
        false
    }
}

pub async fn init_redis(url: String, pub_sub: bool) {
    let client = redis::Client::open(url.clone()).unwrap();
    let result = client.get_tokio_connection_manager().await;

    match result {
        Ok(con) => {
            info!("init redis success ...");

            let value = uuid::Uuid::new_v4().to_string();

            let _ = RM.set(Arc::new(Redis { con, value }));

            if !pub_sub {
                info!("close build work listener ....");
                return;
            }

            // 开启订阅
            thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();

                rt.block_on(async move {
                    info!("start listern redis channel to listener build work ....");

                    loop {
                        let pubsub = client.get_async_connection().await;

                        if let Ok(con) = pubsub {
                            let mut pubsub_conn = con.into_pubsub();
                            let _ = pubsub_conn.subscribe(BUILD_CHANNEL).await;
                            let mut pubsub_stream = pubsub_conn.into_on_message();

                            let data: Option<Msg> = pubsub_stream.next().await;

                            if let Some(msg) = data {
                                if msg.get_channel_name() == BUILD_CHANNEL {
                                    let result: RedisResult<String> = msg.get_payload();

                                    if let Ok(id) = result {
                                        info!(
                                            "found channel = {}, id = {}",
                                            msg.get_channel_name(),
                                            id
                                        );

                                        crate::work::start_build_by_id(id).await;
                                    }
                                }
                            }
                        } else {
                            tokio::time::sleep(Duration::from_millis(1000)).await;
                        }
                    }
                });
            });
        }
        Err(err) => {
            warn!("init redis error ...{}", err);
            thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async move {
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                    info!("restart init redis ...");
                    init_redis(url, false).await
                });
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use actix_rt::time::interval;
    use log::info;
    use redis::{Client, RedisResult};
    use tokio_stream::StreamExt;

    use crate::config;

    use super::Redis;

    async fn lll(client: Client) -> RedisResult<()> {
        config::get_runtime().spawn(async move {
            loop {
                let pubsub = client.get_async_connection().await;

                info!("pubsub thread = {}", thread_id::get());

                if let Ok(pubsub_conn2) = pubsub {
                    let mut pubsub_conn = pubsub_conn2.into_pubsub();
                    let _ = pubsub_conn.subscribe("wavephone").await;
                    let mut pubsub_stream = pubsub_conn.into_on_message();

                    let msg = pubsub_stream.next().await;
                    info!("receive msg = {:?}", msg);
                }
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        });

        config::get_runtime().spawn(async move {
            info!("start publish");

            loop {
                let result = Redis::get_instance();

                match result {
                    Some(res) => {
                        let mut con = res.con.clone();

                        info!(
                            "pub thread = {} {} ",
                            thread_id::get(),
                            Arc::strong_count(&res)
                        );

                        let result: RedisResult<()> = redis::cmd("PUBLISH")
                            .arg(&["wavephone", "bar"])
                            .query_async(&mut con)
                            .await;

                        info!("result = {:?}", result);
                    }
                    None => {}
                }

                tokio::time::sleep(Duration::from_millis(2000)).await;
            }
        });

        Ok(())
    }

    #[tokio::test]
    async fn test_redis_lock() {
        crate::config::Config::get_instance();
        super::init_redis("redis://192.168.2.36:6379".to_string(), false).await;

        let key = "123";
        assert!(super::Redis::lock(key).await);
        assert!(!super::Redis::lock(key).await);
        assert!(super::Redis::unlock(key).await);

        assert!(super::Redis::lock_with_time(key, 10).await);
        tokio::time::sleep(Duration::from_millis(1000)).await;
        assert!(!super::Redis::lock(key).await);
        tokio::time::sleep(Duration::from_millis(10000)).await;
        assert!(super::Redis::lock(key).await);
        assert!(super::Redis::unlock(key).await);
    }

    #[tokio::test]
    async fn test_redis() {
        crate::config::Config::get_instance();
        super::init_redis("redis://192.168.2.36:6379".to_string(), false).await;

        let r = redis::Client::open("redis://192.168.2.36:6379").unwrap();
        let _ = lll(r).await.is_ok();

        let mut interval = interval(Duration::from_millis(1000000));
        loop {
            interval.tick().await;
            info!("> PING");
        }
    }
}
