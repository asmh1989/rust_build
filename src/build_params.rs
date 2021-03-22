use bson::DateTime;
use log::info;
use serde::{Deserialize, Serialize, Serializer};
use url::Url;
use uuid::Uuid;

use std::collections::HashMap;

/// Serializes a bson::DateTime as an ISO string.
pub fn bson_datetime_as_iso_string<S: Serializer>(
    val: &DateTime,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&val.to_string())
}

use crate::{
    db::{Db, COLLECTION_BUILD},
    filter_build_id,
};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Framework {
    // #[serde(rename = "mdm_4")]
    // Mdm4,
    // #[serde(rename = "mdm_4.1")]
    // Mdm41,
    // #[serde(rename = "mdm_4.2")]
    // Mdm42,
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "normal_4.5")]
    Normal45,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Scm {
    #[serde(rename = "git")]
    Git,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Version {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_name: Option<String>,
    pub scm: Option<Scm>,
    pub source_url: Url,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Configs {
    // 打包框架
    pub framework: Framework,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_config: Option<BaseConfig>,
    // 应用配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_config: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BaseConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets_config: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildParams {
    pub version: Version,
    pub configs: Configs,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "responseUrl")]
    pub response_url: Option<Url>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildStatus {
    pub code: i32,
    pub msg: String,
}

const CODE_SUCCESS: i32 = 0;
pub const CODE_ILLEGAL: i32 = -1;
pub const CODE_WAITING: i32 = 2;
pub const CODE_BUILDING: i32 = 3;

pub const MSG_ILLEGAL: &'static str = "非法id";

impl BuildStatus {
    pub fn success() -> Self {
        BuildStatus {
            code: CODE_SUCCESS,
            msg: String::from("打包成功"),
        }
    }

    pub fn is_success(&self) -> bool {
        self.code == CODE_SUCCESS
    }

    pub fn failed(msg: String) -> Self {
        BuildStatus { code: 1, msg }
    }

    pub fn waiting() -> Self {
        BuildStatus {
            code: CODE_WAITING,
            msg: String::from("等待中"),
        }
    }

    pub fn building() -> Self {
        Self {
            code: CODE_BUILDING,
            msg: String::from("编译中"),
        }
    }

    pub fn illegal() -> Self {
        BuildStatus {
            code: CODE_ILLEGAL,
            msg: String::from(MSG_ILLEGAL),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppParams {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<bson::oid::ObjectId>,
    // #[serde(serialize_with = "bson_datetime_as_iso_string")]
    pub date: DateTime,
    pub build_id: Uuid,
    #[serde(flatten)]
    pub status: BuildStatus,
    pub params: BuildParams,
    pub build_time: i16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<DateTime>,
}

impl AppParams {
    pub fn new(params: BuildParams, operate: &str, email: Option<String>) -> Self {
        let date = DateTime(chrono::Utc::now());
        Self {
            id: None,
            date,
            build_id: Uuid::new_v4(),
            status: BuildStatus::waiting(),
            params,
            build_time: 0,
            email,
            fid: Some("".to_string()),
            operate: Some(operate.to_string()),
            update_time: Some(date),
        }
    }

    pub async fn save_db(&self) -> Result<(), String> {
        let doc = match bson::to_bson(&self) {
            Ok(d) => d.as_document().unwrap().clone(),
            Err(e) => {
                info!("to_bson err {}", e);
                return Err(format!("to_bson error : {}", e));
            }
        };

        if let Err(e) = Db::save(
            COLLECTION_BUILD,
            filter_build_id!(self.build_id.to_string()),
            doc.clone(),
        )
        .await
        {
            info!("db save error{} ", e);
            return Err(format!("db save error{} ", e));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{BuildParams, Framework, Scm};
    use serde_json::Result;

    fn typed_example() -> Result<BuildParams> {
        // Some JSON input data as a &str. Maybe this comes from the user.
        let data = r#"
        {
            "version" : {
                "project_name" : "seed",
                "module_name" : "seed",
                "scm" : "git",
                "source_url" : "ssh://git@gitlab.justsafe.com:8442/ht5.0/mdm.git",
                "version_code" : 20111101,
                "version_name" : "5.0.20201111r1",
                "channel" : "master"
            },
            "configs" : {
                "framework": "normal",
                "app_config" : {
                    "is_check_root" : "true",
                    "is_check_support_sim_card" : "true",
                    "is_overseas" : "false",
                    "is_black_sim" : "false"
                }
            },
            "email" : "zhangtc@justsafe.com"
        }"#;

        // Parse the string of data into a Person object. This is exactly the
        // same function as the one that produced serde_json::Value above, but
        // now we are asking it for a Person as output.
        let p: BuildParams = serde_json::from_str(data)?;

        // Do things just like with any other Rust data structure.
        // println!("build params =  {:?}", p);

        // println!(
        //     "build params =  {}",
        //     serde_json::to_string_pretty(&p).ok().unwrap()
        // );

        Ok(p)
    }
    #[test]
    fn params_vaild() {
        let result = typed_example();

        let params = result.unwrap();
        assert_eq!(params.version.project_name.unwrap(), "seed");
        assert_eq!(params.version.module_name.unwrap(), "seed");
        assert_eq!(params.version.scm.unwrap(), Scm::Git);
        assert_eq!(params.configs.framework, Framework::Normal);
    }
}
