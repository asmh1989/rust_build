use std::{collections::HashMap, fs, path::Path, process::Command};

use log::{error, info, warn};
use shell::Shell;
use uuid::Uuid;

use crate::shell;
use crate::utils;
use crate::{build_params, config};
use crate::{
    build_params::{AppParams, Scm},
    framework::base::BuildStep,
};
use crate::{config::Config, framework::*};

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

fn get_channel_command<'a>(channel: Option<String>, log: &'a str) -> String {
    let command = match channel {
        Some(s) => format!(
            "./gradlew assemble{}{}Release --no-daemon > {}",
            (&s[..1].to_string()).to_uppercase(),
            &s[1..],
            &log
        ),

        None => format!("./gradlew assembleRelease --no-daemon >  {}", &log),
    };

    return command;
}

pub fn release_build(app: &AppParams) -> Result<(), String> {
    let dir = get_source_path(app.build_id);
    let log = get_log_file(app.build_id);

    info!("start build in .... {}", &dir);

    let shell = shell::Shell::new(&dir);

    shell.run(&format!("chmod a+x gradlew && ./gradlew clean > {}", &log))?;

    shell.run(&get_channel_command(
        app.params.version.channel.clone(),
        &log,
    ))?;

    Ok(())
}

pub async fn upload_build(app: &mut AppParams) -> Result<(), String> {
    let dir = get_source_path(app.build_id);
    let shell = shell::Shell::new(&dir);
    let apk = shell.run("fd -I -a  'release.apk'")?;
    info!("found apk ... {}", apk);

    let fid = crate::weed::upload(
        apk.trim(),
        format!(
            "{}_{}.apk",
            app.params.version.project_name.clone().unwrap(),
            app.params.version.version_name.clone().unwrap()
        ),
    )
    .await?;

    app.fid = Some(fid);

    Ok(())
}

pub fn change_config(app: &AppParams) -> Result<(), String> {
    let source = get_source_path(app.build_id);
    let android_manifest_xml = source.clone() + "/app/src/main/AndroidManifest.xml";
    let shell = Shell::new(&source);

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

        let output = shell.run("git rev-parse HEAD")?;
        meta.insert("git_version".to_string(), output.trim().to_string());

        info!("change AndroidManifestXml...");

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
    } else {
        return Err("AndroidManifestXml not exist !!".to_string());
    }

    if let Some(app_config) = &app.params.configs.app_config {
        if !app_config.is_empty() {
            info!("change properies file...");

            let file = &format!("{}/app/src/main/assets/config.properties", source);
            utils::change_properies_file(file, app_config)?
        }
    }

    let gradle_file = format!("{}/app/build.gradle", source);

    if let Some(_) = &app.params.version.version_code {
        shell.run(&format!("sd 'versionCode .*' '' {} ", gradle_file))?;
    }

    if let Some(_) = &app.params.version.version_name {
        shell.run(&format!("sd 'versionName .*' '' {} ", gradle_file))?;
    }

    Ok(())
}

pub async fn start(app: &mut AppParams) -> Result<(), String> {
    match app.params.configs.framework {
        crate::build_params::Framework::Normal => {
            normal::NormalBuild().step(app).await?;
        }
        crate::build_params::Framework::Normal45 => mdm::MdmBuild().step(app).await?,
    }

    Ok(())
}

pub async fn start_build(mut app: AppParams) {
    info!("start build {} ... ", app.build_id);
    let time = chrono::Utc::now().timestamp();
    app.status = build_params::BuildStatus::building();
    if let Err(e) = app.save_db().await {
        info!("{}", e);
    }
    match start(&mut app).await {
        Ok(_) => {
            info!("{}  build finish ....", app.build_id);

            app.build_time = (chrono::Utc::now().timestamp() - time) as i16;

            app.status = build_params::BuildStatus::success();
            if let Err(e) = app.save_db().await {
                info!("{}", e);
            }
        }
        Err(e) => {
            warn!(
                "{} error \n------------------------\n{}\n------------------------",
                app.build_id, e
            );

            app.status = build_params::BuildStatus::failed(e);
            if let Err(err) = app.save_db().await {
                info!("{}", err);
            }
        }
    }
    Config::change_building(false);
}

#[cfg(test)]
mod tests {
    use crate::{
        build_params::{AppParams, BuildParams},
        utils,
    };

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
                "framework" : "normal_4.5",
                "base_config" : {
                    "app_name" : "自助助手",
                    "meta" : {
                        "brank" : "common",
                        "model" : "common"
                    },
                    "assets_config" : "http://192.168.2.34:8086/jpm/nas/MDM45-buildConfig/e0d79b5647b241a98c90c19509d9eb63-G贵州公安-45.1.1.201126.1/config.zip"
                },
                "app_config" : {
                    "is_check_root" : "false",
                    "is_check_support_sim_card" : "true",
                    "is_overseas" : "false",
                    "is_black_sim" : "false"
                }
            }
        }"#;

        let p: BuildParams = serde_json::from_str(data)?;

        Ok(p)
    }
    #[actix_rt::test]
    async fn test_normal_build() {
        let result = http_params();

        let params = result.unwrap();

        let mut app = AppParams::new(params, "");
        app.build_id = Uuid::parse_str("936DA01F9ABD4d9d80C702AF85C822A8").unwrap();
        let path = super::get_source_path(app.build_id);

        // 删除存在目录
        utils::remove_dir(&path);

        match super::start(&mut app).await {
            Ok(_) => {
                assert!(true)
            }
            Err(error) => {
                println!("error = {}", error);
                assert!(false);
            }
        }
    }

    #[test]
    fn test_channel_command() {
        let log = "111";
        let command = super::get_channel_command(Some("master".to_string()), log);
        assert_eq!(command, "./gradlew assembleMasterRelease --no-daemon > 111")
    }
}
