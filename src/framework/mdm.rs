use std::{
    fs,
    fs::File,
    io::{self, Write},
};

use log::info;
use reqwest::Url;

use crate::build_params::AppParams;

use super::base::BuildStep;
use crate::utils;

pub struct MdmBuild();

/// 下载文件
fn download_file(path: &str, url: Url) -> Result<(), String> {
    info!("download file {} ...", url);
    match reqwest::blocking::get(url) {
        Ok(response) => {
            let mut file = match File::create(&path) {
                Err(why) => return Err(format!("couldn't create {}", why)),
                Ok(file) => file,
            };

            match response.bytes() {
                Ok(content) => {
                    if file.write_all(&content).is_err() {
                        return Err("read response bytes error!!".to_string());
                    }
                }
                Err(error) => return Err(error.to_string()),
            };
        }
        Err(error) => return Err(error.to_string()),
    };

    Ok(())
}

/// zip解压
fn zip_file(zip: &str, dir: &str) -> Result<(), String> {
    if !crate::utils::file_exist(dir) {
        std::fs::create_dir_all(dir).unwrap();
    }

    match fs::File::open(zip) {
        Ok(f) => match zip::ZipArchive::new(f) {
            Ok(mut archive) => {
                for i in 0..archive.len() {
                    let mut zip_file = archive.by_index(i).unwrap();

                    let name = &format!("{}/{}", dir, zip_file.name());

                    utils::remove_file(name);

                    let mut outfile = File::create(name).unwrap();
                    io::copy(&mut zip_file, &mut outfile).unwrap();
                }
            }
            Err(error) => {
                return Err(error.to_string());
            }
        },
        Err(_) => {
            return Err("zip file open error!!".to_string());
        }
    }

    Ok(())
}

impl BuildStep for MdmBuild {
    fn step_change(&self, app: &AppParams) -> Result<(), String> {
        crate::work::change_config(app)?;

        if let Some(config) = &app.params.configs.base_config {
            if let Some(url) = config.assets_config.clone() {
                info!("config assets config url = {}", url);
                let source = &crate::work::get_source_path(app.build_id);
                let path = format!("{}/.test.zip", source);
                download_file(path.as_str(), url)?;
                zip_file(
                    &path,
                    &format!("{}/core_main/src/main/assets/config", source),
                )?;
            }
        }

        Ok(())
    }

    fn step_source(&self, app: &AppParams) -> Result<(), String> {
        crate::work::fetch_source(app)
    }

    fn step_build(&self, app: &AppParams) -> Result<(), String> {
        crate::work::release_build(app)
    }
}

#[cfg(test)]
mod tests {
    use log::error;
    use reqwest::Url;

    use super::download_file;

    const URL:&'static str = "http://192.168.2.34:8086/jpm/nas/MDM45-buildConfig/5e677a0b0ba94e83ac9a51f7821bddc5-S深圳公安-45.8.1.201127.1/config.zip";
    const PATH: &'static str = "/tmp/123.zip";

    #[test]
    fn test_download_zip() {
        crate::config::Config::get_instance();

        match download_file(PATH, Url::parse(URL).unwrap()) {
            Ok(_) => {
                assert!(true);
            }
            Err(e) => {
                error!("{}", e);
                assert!(false);
            }
        }
    }

    #[test]
    fn test_zip() {
        crate::config::Config::get_instance();
        if !crate::utils::file_exist(PATH) {
            test_download_zip();
        }

        assert!(super::zip_file(PATH, "/tmp/zip").is_ok());
    }
}
