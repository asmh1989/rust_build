use std::{collections::HashMap, fs, path::Path, process::Command};

use crate::redis::Redis;
use bson::Bson;
use log::{error, info, warn};
use shell::Shell;
use uuid::Uuid;

use crate::{build_params, config, get_upload_url, utils::file_exist};
use crate::{
    build_params::{AppParams, Scm},
    framework::base::BuildStep,
};
use crate::{config::Config, framework::*};
use crate::{
    db::{Db, COLLECTION_BUILD},
    filter_build_id, shell,
};
use crate::{get_default, utils};

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

    if Scm::Git == app.params.version.scm.clone().unwrap_or(Scm::Git) {
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

    info!("start build in .... {}  log = {}", &dir, &log);

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
    let apk = shell.run("find `pwd` -name '*release.apk'")?;
    info!("found apk ... {}", apk);

    let fid = crate::weed::upload(
        apk.trim(),
        format!(
            "{}_{}.apk",
            get_default!(app.params.version.project_name),
            get_default!(app.params.version.version_name)
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

        let app_name = match &app.params.configs.base_config {
            Some(c) => c.app_name.clone(),
            None => None,
        };

        match utils::change_xml(
            &fs::read_to_string(Path::new(&android_manifest_xml.as_str())).unwrap(),
            &meta,
            app.params.version.version_code.clone(),
            app.params.version.version_name.clone(),
            app_name,
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
        shell.run(&format!("sed -i -e '/versionCode .*/d' {} ", gradle_file))?;
    }

    if let Some(_) = &app.params.version.version_name {
        shell.run(&format!("sed -i -e '/versionName .*/d' {} ", gradle_file))?;
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

pub async fn start_build_by_id(id: String) {
    if !Config::is_building() {
        if !Redis::lock(&id).await {
            return;
        }
        let doc = filter_build_id!(id);

        let result = Db::find_one(COLLECTION_BUILD, doc, None).await;

        match result {
            Ok(doc) => {
                if let Some(doc) = doc {
                    let result = bson::from_bson::<AppParams>(Bson::Document(doc));
                    match result {
                        Ok(app) => {
                            start_build(app).await;
                        }
                        Err(err) => {
                            info!("{}", err);
                        }
                    }
                } else {
                    info!("start_build_by_id db not find");
                }
            }
            Err(err) => {
                info!("start_build_by_id err = {}", err);
            }
        }
    } else {
        info!("waiting ....")
    }
}

async fn start_build(mut app: AppParams) {
    Config::change_building(true);

    info!("start build {} ... ", app.build_id);
    let time = chrono::Utc::now().timestamp();
    app.status = build_params::BuildStatus::building();
    app.operate = Some(Config::ip());
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

            app.build_time = (chrono::Utc::now().timestamp() - time) as i16;

            app.status = build_params::BuildStatus::failed(e);

            let log = get_log_file(app.build_id);

            if file_exist(&log) {
                match crate::weed::upload(&log, format!("{}.txt", app.build_id)).await {
                    Ok(fid) => {
                        app.status.msg = format!(
                            "{}\n 详细日志地址: {}",
                            app.status.msg,
                            get_upload_url!(fid)
                        );
                    }
                    Err(err) => {
                        info!("error upload log file : {}", err);
                    }
                }
            }

            if let Err(err) = app.save_db().await {
                info!("{}", err);
            }
        }
    }

    match crate::mail::send_email(&app).await {
        Ok(_) => {}
        Err(err) => {
            info!("send mail err = {}", err)
        }
    }

    Redis::unlock(&app.build_id.to_string()).await;

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
                "project_name" : "seed",
                "module_name" : "seed",
                "scm" : "git",
                "source_url" : "ssh://git@gitlab.justsafe.com:8442/ht5.0/mdm.git",
                "channel" : "master",
                "branch" : "device_provisioned",
                "version_code" : 20121701,
                "version_name" : "5.0.20201217r1.v"
            },
            "configs" : {
                "framework" : "normal",
                "base_config" : {},
                "app_config" : {
                    "is_check_root" : "true",
                    "is_overseas" : "false",
                    "is_black_sim" : "false",
                    "is_check_support_sim_card" : "true"
                }
            },
            "email" : "sunmh@justsafe.com"
        }"#;

        let p: BuildParams = serde_json::from_str(data)?;

        Ok(p)
    }
    #[actix_rt::test]
    async fn test_normal_build() {
        let result = http_params();

        let params = result.unwrap();

        let mut app = AppParams::new(params, "test", None);
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
