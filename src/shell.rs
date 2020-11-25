use std::{fs::create_dir, fs::File, io::Write, process::Command, process::Stdio};

use crate::config::Config;
use crate::utils;
use log::{debug, warn};
use uuid::Uuid;

#[derive(Debug)]
pub struct Shell {
    pub current_dir: String,
    pub path: String,
}

impl Shell {
    pub fn new(dir: String) -> Self {
        Self {
            current_dir: dir,
            path: Config::cache_home() + "/tmp",
        }
    }

    pub fn run(&self, command: &str) -> Result<(), String> {
        if !utils::dir_exist(&self.path) {
            create_dir(&self.path).unwrap();
        }

        let path = format!("{}/{}.sh", self.path, Uuid::new_v4().to_string());

        utils::remove_file(&path);
        let result = File::create(path.clone());

        if let Ok(mut file) = result {
            file.write_all(command.as_bytes()).expect("write failed");
            debug!("command:{}", command);
        }

        let result = Command::new("sh")
            .arg(&path)
            .current_dir(self.current_dir.clone())
            .env("ANDROID_HOME", Config::android_home())
            .stderr(Stdio::piped())
            .output();

        utils::remove_file(&path);

        match result {
            Ok(output) => {
                if output.status.code().unwrap() != 0 {
                    let err = String::from_utf8_lossy(&output.stderr);
                    warn!("stderr: {}", err);
                    Err(format!("{}", err))
                } else {
                    Ok(())
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
        let shell = super::Shell::new(String::from("/tmp"));
        let result = shell.run("ls -al");
        match result {
            Ok(_) => {
                assert!(true);
            }
            Err(_) => {
                assert!(false);
            }
        }
    }
}
