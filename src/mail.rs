use bson::Bson;
use chrono::Local;

use crate::result_err;
use log::info;
use regex::Regex;
use serde_json::json;

use crate::{
    build_params::AppParams,
    db::{Db, COLLECTION_BUILD},
    filter_build_id, get_default, get_upload_url,
    http::QueryResponse,
};

async fn _email(mail: &str, title: &str, content: &str) -> Result<(), String> {
    info!(" start send email to {}", mail);

    let client = reqwest::Client::new();

    let _ = client
        .post(format!("http://192.168.2.36:9876/mail").as_str())
        .json(&json!({
            "mail":mail,
            "title": title,
            "content": content
        }))
        .send()
        .await
        .map_err(result_err!())?;

    Ok(())
}

async fn send_response(app: &AppParams) {
    let url = app.params.response_url.clone();

    if url.is_some() {
        let client = reqwest::Client::new();
        let mut res = QueryResponse::new();
        res.to_response(app);
        info!("reponse url = {}", url.clone().unwrap());
        let result = client.post(url.unwrap()).json(&res).send().await;
        match result {
            Ok(res) => {
                info!("response url success! {} ", res.text().await.unwrap())
            }
            Err(err) => {
                info!("response url failed! {}", err)
            }
        }
    }
}

pub async fn send_email(app: &AppParams) -> Result<(), String> {
    send_response(app).await;

    if crate::config::Config::enable_ding() {
        let _ = crate::ding::post_ding(&app).await;
    }

    let email = app.params.email.clone();

    if email.is_some() {
        let email = email.unwrap();
        let email_regex = Regex::new(
            r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})",
        )
        .unwrap();
        if email_regex.is_match(&email) {
            let id = app.build_id;
            let converted = app.date.with_timezone(&Local).to_rfc3339();

            let n = app
                .params
                .version
                .project_name
                .clone()
                .unwrap_or(id.to_string());

            let (title, content) = if app.status.is_success() {
                (
                    format!("打包通知: 恭喜 {} 打包成功了!!", n),
                    format!(
                        r#"
<h3> 打包结果如下:  </h3>
<ul>
<li>打包任务: <code>{}</code></li>
<li>打包时间: <code>{}</code></li>
<li>打包结果: <code>成功</code></li>
<li>打包耗时: <code>{} 秒</code></li>
<li>点击下载: <a href="{}" target="_blank"> 点我! </a></li>
<li>版本信息: </li>
</ul>
<ul>
<pre><code>{}</code></pre>
</ul>

--------------------------------------------
<p>PowerBy <code>{}</code></p>
                "#,
                        id,
                        converted,
                        app.build_time,
                        get_upload_url!(get_default!(app.fid)),
                        serde_json::to_string_pretty(&app.params.version).unwrap(),
                        get_default!(app.operate),
                    ),
                )
            } else {
                (
                    format!("打包通知: 抱歉  {} 打包失败了..", n),
                    format!(
                        r#"
<p> 打包结果如下:  </p>
<ul>
<li>打包任务: <code>{}</code></li>
<li>打包时间: <code>{}</code></li>
<li>打包结果: <code>失败</code></li>
<li>失败原因: </li>
</ul>
<pre><code>{}</code></pre>
<ul>
</ul>
                    
<p>-----------------------------------------</p>
<p>PowerBy <code>{}</code></p>                    
                "#,
                        id,
                        converted,
                        app.status.msg,
                        get_default!(app.operate)
                    ),
                )
            };

            return _email(&email, &title, &content).await;
        } else {
            info!("{} is not email address!", email);
        }
    }

    Ok(())
}

pub async fn send_email_by_id(id: &str) -> Result<(), String> {
    if Db::contians(COLLECTION_BUILD, filter_build_id!(id)).await {
        let result = Db::find_one(COLLECTION_BUILD, filter_build_id!(id), None)
            .await
            .unwrap();

        match result {
            Some(doc) => {
                let result = bson::from_bson::<AppParams>(Bson::Document(doc));
                match result {
                    Ok(app) => send_email(&app).await,
                    Err(err) => {
                        info!("{}", err);
                        Err(format!("{:?}", err))
                    }
                }
            }
            None => Err(format!("not found this document !!!")),
        }
    } else {
        Err(format!("not found this build id!!!"))
    }
}

#[cfg(test)]
mod tests {
    use bson::Bson;
    use log::info;

    use crate::{
        build_params::AppParams,
        db::{init_db, Db, COLLECTION_BUILD},
        filter_build_id,
    };

    use super::send_response;

    async fn send_response_by_id(id: &str) -> Result<(), String> {
        if Db::contians(COLLECTION_BUILD, filter_build_id!(id)).await {
            let result = Db::find_one(COLLECTION_BUILD, filter_build_id!(id), None)
                .await
                .unwrap();

            match result {
                Some(doc) => {
                    let result = bson::from_bson::<AppParams>(Bson::Document(doc));
                    match result {
                        Ok(app) => {
                            send_response(&app).await;
                            Ok(())
                        }
                        Err(err) => {
                            info!("{}", err);
                            Err(format!("{:?}", err))
                        }
                    }
                }
                None => Err(format!("not found this document !!!")),
            }
        } else {
            Err(format!("not found this build id!!!"))
        }
    }

    #[actix_rt::test]
    async fn test_send_email1() {
        crate::config::Config::get_instance();

        assert!(
            super::_email("sunmh@justsafe.com", "test from rust build", "hello world!")
                .await
                .is_ok()
        );
    }

    #[actix_rt::test]
    async fn test_send_response() {
        crate::config::Config::get_instance();

        init_db("mongodb://192.168.2.36:27017").await;

        let result = send_response_by_id("effc2750-e1c8-11ea-bde6-7fab7a770bf7").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_email_failed() {
        crate::config::Config::get_instance();

        init_db("mongodb://192.168.2.36:27017").await;

        let result = super::send_email_by_id("effc2750-e1c8-11ea-bde6-7fab7a770bf7").await;
        assert!(result.is_ok());
    }

    #[actix_rt::test]
    async fn test_send_email_success() {
        crate::config::Config::get_instance();

        init_db("mongodb://192.168.2.36:27017").await;

        let result = super::send_email_by_id("6d77795e-c910-4562-9609-1fc4105c8971").await;
        assert!(result.is_ok());
    }
}
