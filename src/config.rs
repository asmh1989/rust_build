use std::env;
use std::sync::Arc;
use std::sync::Mutex;
#[derive(Clone, Debug)]
pub struct Config {
    pub android_home: String,
    pub java_home: String,
    pub cache_home: String,
}

impl Config {
    pub fn get_instance() -> Arc<Mutex<Config>> {
        static mut CONFIG: Option<Arc<Mutex<Config>>> = None;

        unsafe {
            // Rust中使用可变静态变量都是unsafe的
            CONFIG
                .get_or_insert_with(|| {
                    // 初始化单例对象的代码
                    Arc::new(Mutex::new(Config {
                        android_home: "/opt/android/sdk".to_string(),
                        java_home: "".to_string(),
                        cache_home: format!("{}/.mdm_build", env::var("HOME").unwrap()).to_string(),
                    }))
                })
                .clone()
        }
    }

    pub fn set_cache_home(&mut self, cache: &str) {
        self.cache_home = cache.to_string();
    }

    pub fn set_java_home(&mut self, java: &str) {
        self.java_home = java.to_string();
    }

    pub fn set_android_home(&mut self, android: &str) {
        self.android_home = android.to_string();
    }

    pub fn cache_home() -> String {
        Config::get_instance().lock().unwrap().cache_home.clone()
    }
}
