use bson::Bson;
use chrono::Local;
use lettre::{
    smtp::{
        authentication::{Credentials, Mechanism},
        extension::ClientId,
        ConnectionReuseParameters,
    },
    ClientSecurity, ClientTlsParameters, SmtpClient, Transport,
};
use lettre_email::EmailBuilder;
use log::info;
use native_tls::{Protocol, TlsConnector};
use regex::Regex;

use crate::{
    build_params::AppParams,
    db::{Db, COLLECTION_BUILD},
    filter_build_id, get_upload_url,
};

async fn _email(mail: &str, title: &str, content: &str) -> Result<(), String> {
    info!(" start send email to {}", mail);
    let email = EmailBuilder::new()
        .from("androidbuild@justsafe.com")
        .to(mail)
        .subject(title)
        .html(content)
        .build()
        .unwrap();

    let mut tls_builder = TlsConnector::builder();
    // Disable as many security features as possible ( no luck :( )
    tls_builder.min_protocol_version(Some(Protocol::Sslv3));
    tls_builder.use_sni(false);
    tls_builder.danger_accept_invalid_certs(true);
    tls_builder.danger_accept_invalid_hostnames(true);
    let tls_parameters = ClientTlsParameters::new(
        "mail.justsafe.com".to_string(),
        tls_builder.build().unwrap(),
    );

    let mut mailer = SmtpClient::new(
        ("mail.justsafe.com", 587),
        ClientSecurity::Required(tls_parameters),
    )
    .unwrap()
    .authentication_mechanism(Mechanism::Login) // Mechanism::Login does not work either
    .hello_name(ClientId::Domain("mail.justsafe.com".to_string()))
    .credentials(Credentials::new(
        "androidbuild@justsafe.com".to_string(),
        "Justsy123".to_string(),
    ))
    .connection_reuse(ConnectionReuseParameters::ReuseUnlimited)
    .transport();

    // Send the email
    match mailer.send(email.into()) {
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
            let converted = app.date.with_timezone(&Local).to_rfc3339();

            let (title, content) = if app.status.is_success() {
                (
                    format!("打包通知: 恭喜 {} 打包成功了!!", id),
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
                        get_upload_url!(app.fid.clone().unwrap()),
                        serde_json::to_string_pretty(&app.params.version).unwrap(),
                        app.operate.clone().unwrap(),
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
                        app.operate.clone().unwrap()
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
    use crate::db::init_db;

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
