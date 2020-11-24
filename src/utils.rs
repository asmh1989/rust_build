use git2::Repository;
use git2::{Cred, RemoteCallbacks};
use std::env;
use std::fs::{remove_dir_all, File};
use std::path::Path;

/// git clone 代码
pub fn clone_src(url: &str, path: &str) -> Result<(), String> {
    if url.starts_with("http") {
        println!("start clone {} to {}", url, path);

        let result = Repository::clone(url, path);
        match result {
            Ok(_) => Ok(()),
            Err(error) => Err(error.message().to_string()),
        }
    } else {
        println!("start ssh clone {} to {}", url, path);
        // Prepare callbacks.
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key(
                username_from_url.unwrap(),
                None,
                std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
                None,
            )
        });

        // Prepare fetch options.
        let mut fo = git2::FetchOptions::new();
        fo.remote_callbacks(callbacks);

        // Prepare builder.
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fo);

        // Clone the project.
        let ressult = builder.clone(url, Path::new(path));
        match ressult {
            Ok(_) => Ok(()),
            Err(error) => Err(error.message().to_string()),
        }
    }
}

pub fn remove_dir(name: &str) {
    let f = File::open(name);

    if let Ok(_) = f {
        remove_dir_all(name).expect("删除文件失败");
        println!("delete {} success", name);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn http_clone() {
        let name = "/tmp/okhttp4_demo";

        super::remove_dir(name);

        let result = super::clone_src("https://github.com/asmh1989/okhttp4_demo.git", name);

        assert!(None == result.err());
    }

    #[test]
    fn ssh_clone() {
        let name = "/tmp/okhttp4_demo";

        super::remove_dir(name);

        let result = super::clone_src("git@github.com:asmh1989/okhttp4_demo.git", name);

        assert!(None == result.err());
    }
}
