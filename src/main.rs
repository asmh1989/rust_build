#![allow(dead_code)]

use build_params::BuildParams;
use log::{error, info};
use serde_json::Result;

mod build_params;
mod config;
mod shell;
mod utils;
mod work;

fn typed_example() -> Result<BuildParams> {
    // Some JSON input data as a &str. Maybe this comes from the user.
    let data = r#"
    {
        "version" : {
            "project_name" : "seed",
            "module_name" : "seed",
            "scm" : "git",
            "source_url" : "ssh://git@gitlab.justsafe.com:8442/ht5.0/mdm.git",
            "version_code" : 20111101,
            "version_name" : "5.0.20201111r1",
            "channel" : "master"
        },
        "configs" : {
            "framework": "normal",
            "app_config" : {
                "is_check_root" : "true",
                "is_check_support_sim_card" : "true",
                "is_overseas" : "false",
                "is_black_sim" : "false"
            }
        },
        "email" : "zhangtc@justsafe.com"
    }"#;

    // Parse the string of data into a Person object. This is exactly the
    // same function as the one that produced serde_json::Value above, but
    // now we are asking it for a Person as output.
    let p: BuildParams = serde_json::from_str(data)?;

    // Do things just like with any other Rust data structure.
    info!("build params =  {:?}", p);

    info!(
        "build params =  {}",
        serde_json::to_string_pretty(&p).ok().unwrap()
    );

    Ok(p)
}
fn main() {
    // 修改config
    // config::Config::get_instance()
    // .lock()
    // .unwrap()
    // .set_cache_home("/tmp");

    config::Config::get_instance();

    info!("start ...");

    let result = typed_example();

    if let Err(e) = result {
        error!("error parsing header: {:?}", e);
        return;
    }
}
