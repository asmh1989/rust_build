use std::{fs::create_dir, fs::File, io::Write, process::Command};

use crate::config::Config;
use crate::utils;
use log::{debug, error};
use uuid::Uuid;

#[derive(Debug)]
pub struct Shell {
    pub current_dir: String,
    pub path: String,
}

impl Shell {
    pub fn new(dir: &str) -> Self {
        Self {
            current_dir: dir.to_string(),
            path: Config::cache_home() + "/tmp",
        }
    }

    pub fn run(&self, command: &str) -> Result<String, String> {
        if !utils::file_exist(&self.path) {
            let result = create_dir(&self.path);
            if result.is_err() {
                error!("create dir error");
                return Err(format!(
                    "mkdir path = {}, error = {:?}",
                    &self.path,
                    result.err()
                ));
            };
        }

        let path = format!("{}/{}.sh", self.path, Uuid::new_v4().to_string());

        if let Ok(mut file) = File::create(&path) {
            file.write_all(command.as_bytes()).expect("write failed");
            debug!("command:{}", command);
        }

        let result = Command::new("sh")
            .arg(&path)
            .current_dir(&self.current_dir)
            .env("ANDROID_HOME", Config::android_home())
            .output();

        utils::remove_file(&path);

        match result {
            Ok(output) => {
                if output.status.code().unwrap() != 0 {
                    let err = String::from_utf8_lossy(&output.stderr).to_string();
                    // if !err.is_empty() {
                    //     warn!("stderr: {}", err);
                    // }
                    Err(format!("{}", err))
                } else {
                    Ok(String::from_utf8_lossy(&output.stdout).to_string())
                }
            }
            Err(error) => Err(error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_shell() {
        let shell = super::Shell::new("/tmp");
        let result = shell.run("ls -al");

        assert!(result.is_ok());
    }
}
