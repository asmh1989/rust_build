use git2::Repository;
use git2::{Cred, Error, RemoteCallbacks};
use std::env;
use std::path::Path;

/// git clone 代码
pub fn clone_src(url: &str, path: &str) -> Result<Repository, Error> {
    if url.starts_with("http") {
        println!("start clone {} to {}", url, path);

        Repository::clone(url, path)
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
        builder.clone(url, Path::new(path))
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{remove_dir_all, File};

    #[test]
    fn http_clone() {
        let name = "/tmp/okhttp4_demo";

        let f = File::open(name);

        if let Ok(_) = f {
            remove_dir_all(name).expect("删除文件失败");
            println!("delete {} success", name);
        }

        let result = super::clone_src("https://github.com/asmh1989/okhttp4_demo.git", name);

        assert!(None == result.err());
    }

    #[test]
    fn ssh_clone() {
        let name = "/tmp/okhttp4_demo";

        let f = File::open(name);

        if let Ok(_) = f {
            remove_dir_all(name).expect("删除文件失败");
            println!("delete {} success", name);
        }

        let result = super::clone_src("git@github.com:asmh1989/okhttp4_demo.git", name);

        assert!(None == result.err());
    }
}
