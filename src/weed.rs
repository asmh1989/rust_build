use log::info;
use reqwest::{multipart::Form, multipart::Part, Body, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::result_err;

const ASSIGN_URL: &'static str = "http://gitlab.justsafe.com:9333/dir/assign";
const LOOKUP_URL: &'static str = "http://gitlab.justsafe.com:9333/dir/lookup?fileId=";
const AUTH_KEY: &'static str = "Authorization";

#[derive(Debug, Serialize, Deserialize)]
struct Assign {
    pub fid: String,
    pub url: String,
}

#[macro_export]
macro_rules! get_upload_url {
    ($e:expr) => {
        format!("http://gitlab.justsafe.com:8080/{}", $e).as_str()
    };
}

pub async fn upload(path: &str, file_name: String) -> Result<String, String> {
    info!("upload file {} ...", path);

    if !crate::utils::file_exist(path) {
        return Err(format!("{} not exist!!", path));
    }

    match reqwest::get(ASSIGN_URL).await {
        Ok(res) => {
            let header = res.headers().clone();

            let json = res.json::<Assign>().await.map_err(result_err!())?;

            info!("res = {:?}", json);

            let fid = json.fid;
            let url = json.url;

            let client = reqwest::Client::new();

            let mut form = Form::new();
            form = {
                let file = File::open(path).await.map_err(result_err!())?;

                let reader = Body::wrap_stream(FramedRead::new(file, BytesCodec::new()));
                form.part("file", Part::stream(reader).file_name(file_name))
            };

            let result = client
                .post(format!("http://{}/{}", url, fid).as_str())
                .header(AUTH_KEY, header.get(AUTH_KEY).unwrap())
                .multipart(form)
                .send()
                .await
                .map_err(result_err!())?;

            let code = { result.status() };
            let s = { result.text().await.unwrap() };
            info!("upload {}, url = {} ", s, get_upload_url!(fid));

            if code == StatusCode::CREATED {
                Ok(fid)
            } else {
                Err(s)
            }
        }
        Err(err) => return Err(err.to_string()),
    }
}

pub async fn delete(fid: &str) -> Result<(), String> {
    let res = reqwest::get(format!("{}{}", LOOKUP_URL, fid).as_str())
        .await
        .map_err(result_err!())?;

    let auth = { res.headers().get(AUTH_KEY) };

    let client = reqwest::Client::new();

    let result = client
        .delete(get_upload_url!(fid))
        .header(AUTH_KEY, auth.unwrap())
        .send()
        .await
        .map_err(result_err!())?;

    let code = { result.status() };
    let s = { result.text().await.unwrap() };
    info!("delete {} ", s);

    if code == StatusCode::ACCEPTED {
        Ok(())
    } else {
        Err(s)
    }
}

#[cfg(test)]
mod tests {
    use log::info;

    #[actix_rt::test]
    async fn test_upload() {
        crate::config::Config::get_instance();

        let result = super::upload("/tmp/123.zip", "132.zip".to_string()).await;
        match result {
            Ok(fid) => {
                let result = super::delete(&fid).await;
                match result {
                    Ok(_) => {
                        assert!(true);
                    }
                    Err(err) => {
                        info!("err = {}", err);
                        assert!(false);
                    }
                }
            }
            Err(err) => {
                info!("err = {}", err);
                assert!(false);
            }
        }
    }

    #[actix_rt::test]
    async fn test_delete() {
        crate::config::Config::get_instance();
        let result = super::delete("26,06d488a59e3a").await;
        match result {
            Ok(_) => {
                assert!(true);
            }
            Err(err) => {
                info!("err = {}", err);
                assert!(false);
            }
        }
    }
}
