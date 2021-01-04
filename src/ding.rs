use std::{cmp, collections::HashMap};

use chrono::Local;
use log::info;
use once_cell::sync::OnceCell;
use serde_json::json;

use crate::{build_params::AppParams, get_default, get_upload_url, result_err};

static MOBILE_MAP: OnceCell<HashMap<&str, &str>> = OnceCell::new();
const DING_URL:&'static str = "https://oapi.dingtalk.com/robot/send?access_token=bf650de5c1ab6d8c05edcd826db6c0808dcfa0f673d217de466240652643ad3f";

async fn _ding(title: &str, content: &str) -> Result<(), String> {
    info!("start ding with {}", title);

    let client = reqwest::Client::new();
    let body = json!({
        "msgtype": "markdown",
        "markdown": {
            "title":format!("{} 打包通知", title),
            "text": content
        },
         "at": {
             "isAtAll": true
         }

    });
    let res = client
        .post(DING_URL)
        .json(&body)
        .send()
        .await
        .map_err(result_err!())?;

    info!("ding response : {:?}", res.text().await);

    Ok(())
}

pub async fn post_ding(app: &AppParams) -> Result<(), String> {
    let email = app.params.email.clone();
    if email.is_some() {
        let id = app.build_id;
        let converted = app.date.with_timezone(&Local).to_rfc3339();
        let msg = app.status.msg.clone();

        let n = app
            .params
            .version
            .project_name
            .clone()
            .unwrap_or(id.to_string());

        let content = if app.status.is_success() {
            format!(
                r#" 
## 恭喜 {} 打包成功了
---
###  打包结果如下:
* 打包任务: {}
* 打包时间: {}
* 打包耗时: {} 秒
* 点击下载: [`点我!`]({})

####  版本信息: 

```json
{}
```

---n
>  `PowerBy {}`
                "#,
                n,
                id,
                converted,
                app.build_time,
                get_upload_url!(get_default!(app.fid)),
                serde_json::to_string_pretty(&app.params.version).unwrap(),
                get_default!(app.operate),
            )
        } else {
            format!(
                r#"
## 抱歉 {} 打包失败了
---                     
###  打包结果如下:  

* 打包任务: {}
* 打包时间: {}

### 错误日志
```
{}
```

> 详细日志请查询邮件或者使用[查询接口](http://192.168.2.34:7002/app/query/{})
---
>  `PowerBy {}`                  
                "#,
                n,
                id,
                converted,
                &msg[0..cmp::min(512, msg.len() - 1)],
                id,
                get_default!(app.operate)
            )
        };

        return _ding(&n, &content).await;
    } else {
        info!("not found emial ...");
    }
    Ok(())
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

    async fn post_ding_by_id(id: &str) -> Result<(), String> {
        if Db::contians(COLLECTION_BUILD, filter_build_id!(id)).await {
            let result = Db::find_one(COLLECTION_BUILD, filter_build_id!(id), None)
                .await
                .unwrap();

            match result {
                Some(doc) => {
                    let result = bson::from_bson::<AppParams>(Bson::Document(doc));
                    match result {
                        Ok(app) => {
                            return super::post_ding(&app).await;
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
    async fn test_ding() {
        crate::config::Config::get_instance();

        let result = super::_ding("18094603771", "## 你知道吗 \n > 你是 \n ### 什么").await;
        assert!(result.is_ok());
    }

    #[actix_rt::test]
    async fn test_send_ding() {
        crate::config::Config::get_instance();

        init_db("mongodb://192.168.2.36:27017").await;

        let result = post_ding_by_id("08028b97-00e2-4ef8-8e03-5f90fe930e4c").await;
        assert!(result.is_ok());
    }

    #[actix_rt::test]
    async fn test_send_ding_failed() {
        crate::config::Config::get_instance();

        init_db("mongodb://192.168.2.36:27017").await;

        let result = post_ding_by_id("effc2750-e1c8-11ea-bde6-7fab7a770bf7").await;
        assert!(result.is_ok());
    }
}
