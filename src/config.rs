use std::env;
use std::sync::Arc;
use std::sync::Mutex;
#[derive(Clone, Debug)]
pub struct Config {
    pub android_home: String,
    pub java_home: String,
    pub cache_home: String,
    pub building: bool,
    pub ip: String,
}

impl Config {
    pub fn get_instance() -> Arc<Mutex<Config>> {
        static mut CONFIG: Option<Arc<Mutex<Config>>> = None;

        unsafe {
            // Rust中使用可变静态变量都是unsafe的
            CONFIG
                .get_or_insert_with(|| {
                    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();
                    // info!("log4rs init ...");
                    // 初始化单例对象的代码
                    Arc::new(Mutex::new(Config {
                        android_home: "/opt/android/sdk".to_string(),
                        java_home: "".to_string(),
                        cache_home: format!("{}/.mdm_build", env::var("HOME").unwrap()).to_string(),
                        building: false,
                        ip: whoami::hostname(),
                    }))
                })
                .clone()
        }
    }

    pub fn set_cache_home(&mut self, cache: &str) {
        self.cache_home = cache.to_string();
    }

    pub fn set_ip(&mut self, ip: &str) {
        self.ip = ip.to_string();
    }

    pub fn set_java_home(&mut self, java: &str) {
        self.java_home = java.to_string();
    }

    pub fn set_android_home(&mut self, android: &str) {
        self.android_home = android.to_string();
    }

    pub fn set_building(&mut self, building: bool) {
        self.building = building;
    }

    pub fn cache_home() -> String {
        Config::get_instance().lock().unwrap().cache_home.clone()
    }

    pub fn android_home() -> String {
        Config::get_instance().lock().unwrap().android_home.clone()
    }

    pub fn is_building() -> bool {
        Config::get_instance().lock().unwrap().building
    }

    pub fn ip() -> String {
        Config::get_instance().lock().unwrap().ip.clone()
    }

    pub fn change_building(b: bool) {
        Config::get_instance().lock().unwrap().set_building(b);
    }
}
