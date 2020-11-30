use bson::DateTime;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use std::collections::HashMap;

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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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
    pub scm: Scm,
    pub source_url: Url,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_code: Option<u32>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_url: Option<Url>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildStatus {
    pub code: i8,
    pub msg: String,
}

impl BuildStatus {
    pub fn success() -> Self {
        BuildStatus {
            code: 0,
            msg: String::from("打包成功"),
        }
    }

    pub fn failed(msg: String) -> Self {
        BuildStatus { code: 1, msg }
    }

    pub fn waiting() -> Self {
        BuildStatus {
            code: 2,
            msg: String::from("等待中"),
        }
    }

    pub fn building() -> Self {
        Self {
            code: 3,
            msg: String::from("编译中"),
        }
    }

    pub fn illegal() -> Self {
        BuildStatus {
            code: -1,
            msg: String::from("非法id"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppParams {
    pub date: DateTime,
    pub build_id: Uuid,
    #[serde(flatten)]
    pub status: BuildStatus,
    pub params: BuildParams,
    pub build_time: u16,
    pub fid: Option<String>,
    pub operate: String,
}

impl AppParams {
    pub fn new(params: BuildParams, operate: &str) -> Self {
        Self {
            date: DateTime(chrono::Utc::now()),
            build_id: Uuid::new_v4(),
            status: BuildStatus::waiting(),
            params: params,
            build_time: 0,
            fid: None,
            operate: String::from(operate),
        }
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
        assert_eq!(params.version.scm, Scm::Git);
        assert_eq!(params.configs.framework, Framework::Normal);
    }
}
