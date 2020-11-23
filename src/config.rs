use std::env;

pub struct Config {
    pub android_home: String,
    pub java_home: String,
    pub cache_home: String,
}

impl Config {
    fn new() -> Config {
        Config {
            android_home: "",
            java_home: "",
            cache_home: &format!("{}/.mdm_build", env::var("HOME").unwrap(), CACHE_PATH),
        }
    }

    fn set_cache_home(&self, cache: &str) -> Config {
        self.cache_home = cache.clone();
        self
    }

    fn set_java_home(&self, java: &str) -> Config {
        self.java_home = java.clone();
        self
    }

    fn set_android_home(&self, android: &str) -> Config {
        self.android_home = android.clone();
        self
    }
}
