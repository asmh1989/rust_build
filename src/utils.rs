use log::{debug, info};
use std::fs::{remove_dir_all, File};

use std::fs;

use crate::shell::Shell;

/// git clone 代码
// pub fn clone_src(url: &str, path: &str) -> Result<(), String> {
//     if url.starts_with("http") {
//         info!("start clone {} to {}", url, path);

//         let result = Repository::clone(url, path);
//         match result {
//             Ok(_) => Ok(()),
//             Err(error) => Err(error.message().to_string()),
//         }
//     } else {
//         info!("start ssh clone {} to {}", url, path);
//         // Prepare callbacks.
//         let mut callbacks = RemoteCallbacks::new();
//         callbacks.credentials(|_url, username_from_url, _allowed_types| {
//             Cred::ssh_key(
//                 username_from_url.unwrap(),
//                 None,
//                 std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
//                 None,
//             )
//         });

//         // Prepare fetch options.
//         let mut fo = git2::FetchOptions::new();
//         fo.remote_callbacks(callbacks);

//         // Prepare builder.
//         let mut builder = git2::build::RepoBuilder::new();
//         builder.fetch_options(fo);

//         // Clone the project.
//         let ressult = builder.clone(url, Path::new(path));
//         match ressult {
//             Ok(_) => Ok(()),
//             Err(error) => Err(error.message().to_string()),
//         }
//     }
// }

pub fn clone_src(
    url: &str,
    path: &str,
    branch: Option<String>,
    revision: Option<String>,
) -> Result<(), String> {
    info!("start git clone {} to {}", url, path);

    let shell = Shell::new(String::from("/tmp"));
    let mut command = format!("git clone {} ", url);

    if let Some(b) = branch {
        command.push_str(&format!(" -b {} ", &b));
    }

    command.push_str(path.clone());

    shell.run(&command)?;

    if let Some(commit) = revision {
        let shell = Shell::new(String::from(path));
        info!(" checkout {} ", &commit);
        let command = format!("git checkout {}", commit);
        shell.run(&command)?;
    }

    Ok(())
}

pub fn remove_dir(name: &str) {
    let f = File::open(name);

    if let Ok(_) = f {
        remove_dir_all(name).expect("删除文件失败");
        debug!("delete {} success", name);
    }
}

pub fn remove_file(name: &str) {
    let f = File::open(name);

    if let Ok(_) = f {
        fs::remove_file(name).expect("删除文件失败");
        debug!("delete {} success", name);
    }
}

pub fn file_exist(path: &str) -> bool {
    fs::metadata(path).is_ok()
}

#[cfg(test)]
mod tests {
    static NAME: &'static str = "/tmp/okhttp4_demo";

    #[test]
    fn http_clone() {
        super::remove_dir(NAME);
        let result = super::clone_src(
            "https://github.com/asmh1989/okhttp4_demo.git",
            NAME,
            None,
            None,
        );
        assert!(None == result.err());
    }

    #[test]
    fn ssh_clone() {
        super::remove_dir(NAME);
        let result = super::clone_src(
            "git@github.com:asmh1989/okhttp4_demo.git",
            NAME,
            Some("test".to_string()),
            None,
        );
        assert!(None == result.err());
    }

    #[test]
    fn ssh_clone_commit() {
        super::remove_dir(NAME);
        let result = super::clone_src(
            "git@github.com:asmh1989/okhttp4_demo.git",
            NAME,
            None,
            Some(format!("e9406d9d41cdbff36603fb0de488f09d5e18b93b")),
        );
        assert!(None == result.err());
    }
}
