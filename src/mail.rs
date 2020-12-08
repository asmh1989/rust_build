use std::fs;

use lettre::{
    transport::smtp::authentication::Credentials,
    transport::smtp::client::{Certificate, Tls, TlsParameters},
    Message, SmtpTransport, Transport,
};

use log::info;
use regex::Regex;

use crate::{build_params::AppParams, get_upload_url};

async fn _email(mail: &str, title: &str, content: &str) -> Result<(), String> {
    info!(" start send email to {}", mail);
    let email = Message::builder()
        .from("androidBuild <androidbuild@justsafe.com>".parse().unwrap())
        .to(mail.parse().unwrap())
        .subject(title)
        .body(content)
        .unwrap();

    let creds = Credentials::new(
        "androidbuild@justsafe.com".to_string(),
        "Justsy123".to_string(),
    );

    let pem_cert = fs::read("config/certificate.pem").unwrap();
    let cert = Certificate::from_pem(&pem_cert).unwrap();

    let mut tls = TlsParameters::builder("mail.justsafe.com".to_string());
    tls.add_root_certificate(cert);
    tls.dangerous_accept_invalid_certs(true);
    tls.dangerous_accept_invalid_hostnames(true);
    let tls = tls.build().unwrap();

    // Open a remote connection to gmail
    let mailer = SmtpTransport::builder_dangerous("mail.justsafe.com")
        // .unwrap()
        .port(587)
        .tls(Tls::Required(tls))
        .credentials(creds)
        .build();

    // Send the email
    match mailer.send(&email) {
        Ok(_) => info!("Email sent successfully!"),
        Err(e) => info!("Could not send email: {:?}", e),
    }
    Ok(())
}

pub async fn send_email(app: &AppParams) -> Result<(), String> {
    let url = app.params.response_url.clone();

    if url.is_some() {
        let client = reqwest::Client::new();
        let result = client.post(url.unwrap()).send().await;
        match result {
            Ok(_) => {
                info!("response url success!")
            }
            Err(err) => {
                info!("response url failed! {}", err)
            }
        }
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
            let (title, content) = if app.status.is_success() {
                (
                    format!("打包通知: 恭喜 {} 打包成功了!!", id),
                    format!(
                        r#"
<h3> 打包结果如下:  </h3>
<ul>
<li>打包任务: <code>{}</code></li>
<li>打包时间: <code>{:?}</code></li>
<li>打包结果: <code>成功</code></li>
<li>打包耗时: <code>{} 秒</code></li>
<li>点击下载: <a href="{}" target="_blank"> 点我! </a></li>
<li>版本信息: </li>
</ul>
<ul>
<pre><code>${:?}</code></pre>
</ul>

--------------------------------------------
<p>PowerBy <code>{:?}</code></p>
                "#,
                        id,
                        app.date,
                        app.build_time,
                        get_upload_url!(app.fid.clone().unwrap()),
                        app.params.version,
                        app.operate,
                    ),
                )
            } else {
                (
                    format!("打包通知: 抱歉  {} 打包失败了..", id),
                    format!(
                        r#"
<p> 打包结果如下:  </p>
<ul>
<li>打包任务: <code>{}</code></li>
<li>打包时间: <code>{:?}</code></li>
<li>打包结果: <code>失败</code></li>
<li>失败原因: </li>
</ul>
<pre><code>{}</code></pre>
<ul>
</ul>
                    
<p>-----------------------------------------</p>
<p>PowerBy <code>$ip</code></p>                    
                "#,
                        id, app.date, app.status.msg
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

#[cfg(test)]
mod tests {
    use bson::Bson;
    use log::info;

    use crate::{build_params::AppParams, db::*, filter_build_id};

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
    async fn test_send_email() {
        crate::config::Config::get_instance();

        init_db("mongodb://192.168.2.36:27017").await;

        let result = Db::find_one(
            COLLECTION_BUILD,
            filter_build_id!("effc2750-e1c8-11ea-bde6-7fab7a770bf7"),
            None,
        )
        .await
        .unwrap();

        match result {
            Some(doc) => {
                let result = bson::from_bson::<AppParams>(Bson::Document(doc));
                match result {
                    Ok(app) => {
                        let r = super::send_email(&app).await;
                        assert!(r.is_ok());
                    }
                    Err(err) => {
                        info!("{}", err);
                        assert!(false);
                    }
                }
            }
            None => {
                assert!(false)
            }
        }
    }
}
