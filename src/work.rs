use uuid::Uuid;

use crate::build_params::*;
use crate::config;
use crate::utils;

pub fn getSourcePath(build_id: Uuid) -> String {
    let path = config::Config::cache_home();
    path + "/apps/" + &build_id.to_string()
}

pub fn getSource(app: AppParams) -> Result<(), String> {
    let url = app.params.version.source_url;

    if Scm::Git == app.params.version.scm {
        utils::clone_src(url.as_str(), &getSourcePath(app.build_id))
    } else {
        Err("不支持的scm".to_string())
    }
}

pub fn start(_params: BuildParams) {}

#[cfg(test)]
mod tests {
    use crate::{build_params::AppParams, utils::remove_dir};

    use super::{getSource, BuildParams};
    use serde_json::Result;
    use uuid::Uuid;

    fn http_params() -> Result<BuildParams> {
        // Some JSON input data as a &str. Maybe this comes from the user.
        let data = r#"
        {
            "version" : {
                "scm" : "git",
                "source_url" : "https://github.com/asmh1989/build_demo.git"
            },
            "configs" : {
                "framework": "normal"
            }
        }"#;

        let p: BuildParams = serde_json::from_str(data)?;

        Ok(p)
    }
    #[test]
    fn test_http_clone() {
        let result = http_params();

        let params = result.unwrap();

        let mut app = AppParams::new(params, "");
        app.build_id = Uuid::parse_str("936DA01F9ABD4d9d80C702AF85C822A8").unwrap();
        let path = super::getSourcePath(app.build_id);

        remove_dir(&path);

        let result = getSource(app);

        match result {
            Ok(_) => {
                assert!(true)
            }
            Err(error) => {
                println!("error = {}", error);
                assert!(false);
            }
        }
    }
}
