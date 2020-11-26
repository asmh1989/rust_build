use std::{collections::HashMap, fs, path::Path, process::Command};

use log::{error, info};
use shell::Shell;
use uuid::Uuid;

use crate::build_params::{AppParams, Scm};
use crate::config;
use crate::shell;
use crate::utils;

pub fn get_source_path(build_id: Uuid) -> String {
    let path = config::Config::cache_home();
    path + "/apps/" + &build_id.to_string()
}

pub fn get_log_file(build_id: Uuid) -> String {
    let path = config::Config::cache_home();
    if !utils::file_exist(&(path.clone() + "/logs")) {
        let _r = Command::new("mkdir")
            .args(&["-p", &(path.clone() + "/logs")])
            .output();
    }
    path + "/logs/" + &build_id.to_string() + ".txt"
}

pub fn fetch_source(app: &AppParams) -> Result<(), String> {
    let url = app.params.version.source_url.clone();

    if Scm::Git == app.params.version.scm {
        utils::clone_src(
            url.as_str(),
            &get_source_path(app.build_id),
            app.params.version.branch.clone(),
            app.params.version.revision.clone(),
        )
    } else {
        Err("不支持的scm".to_string())
    }
}

pub fn release_build(app: &AppParams) -> Result<(), String> {
    let dir = get_source_path(app.build_id);
    let log = get_log_file(app.build_id);

    info!("start build in .... {}", &dir);

    let shell = shell::Shell::new(dir);

    shell.run(&format!("chmod a+x gradlew && ./gradlew clean > {}", &log))?;

    shell.run(&format!(
        "./gradlew assembleRelease --no-daemon >  {}",
        &log
    ))?;

    info!("build success!!");

    Ok(())
}

pub fn change_config(app: &AppParams) -> Result<(), String> {
    let source = get_source_path(app.build_id);
    let android_manifest_xml = source.clone() + "/app/src/main/AndroidManifest.xml";

    if utils::file_exist(&android_manifest_xml) {
        let mut meta: HashMap<String, String> = HashMap::new();
        let mut attrs: HashMap<String, String> = HashMap::new();
        if let Some(c) = &app.params.configs.base_config {
            if let Some(m) = &c.meta {
                meta.clone_from(m);
            }

            if let Some(app_name) = &c.app_name {
                attrs.insert(format!("android:label"), app_name.clone());
            }
        }

        let shell = Shell::new(source.clone());
        let output = shell.run("git rev-parse HEAD")?;
        meta.insert("git_version".to_string(), output.trim().to_string());

        match utils::change_xml(
            &fs::read_to_string(Path::new(&android_manifest_xml.as_str())).unwrap(),
            &meta,
            app.params.version.version_code.clone(),
            app.params.version.version_name.clone(),
            Some(android_manifest_xml.as_str()),
        ) {
            Ok(_) => {}
            Err(e) => {
                error!("{}", e.to_string());
                return Err(e.to_string());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        build_params::{AppParams, BuildParams},
        utils::file_exist,
    };

    use super::fetch_source;
    use serde_json::Result;
    use uuid::Uuid;

    fn http_params() -> Result<BuildParams> {
        // Some JSON input data as a &str. Maybe this comes from the user.
        let data = r#"
        {
            "version" : {
                "scm" : "git",
                "source_url" : "https://github.com/asmh1989/build_demo.git",
                "version_code" : 20112601,
                "version_name" : "45.1.1.201126.1"
            },
            "configs" : {
                "framework" : "normal",
                "base_config" : {
                    "app_name" : "自助助手",
                    "meta" : {
                        "brank" : "common",
                        "model" : "common"
                    },
                    "assets_config" : "http://192.168.2.34:8086/jpm/nas/MDM45-buildConfig/e0d79b5647b241a98c90c19509d9eb63-G贵州公安-45.1.1.201126.1/config.zip"
                },
                "app_config" : {}
            }
        }"#;

        let p: BuildParams = serde_json::from_str(data)?;

        Ok(p)
    }
    #[test]
    fn test_normal_build() {
        let result = http_params();

        let params = result.unwrap();

        let mut app = AppParams::new(params, "");
        app.build_id = Uuid::parse_str("936DA01F9ABD4d9d80C702AF85C822A8").unwrap();
        let path = super::get_source_path(app.build_id);

        // 删除存在目录
        // remove_dir(&path);

        if !file_exist(&path) {
            let result = fetch_source(&app);

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
        let result = super::change_config(&app);
        assert!(result.is_ok());

        let result = super::release_build(&app);

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
