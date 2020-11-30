use crate::build_params::AppParams;
use crate::work::*;

pub trait BuildStep {
    fn step_source(&self, app: &AppParams) -> Result<(), String> {
        fetch_source(app)
    }

    fn step_change(&self, app: &AppParams) -> Result<(), String> {
        change_config(app)
    }

    fn step_build(&self, app: &AppParams) -> Result<(), String> {
        release_build(app)
    }
}

pub fn step(build: &dyn BuildStep, app: &AppParams) -> Result<(), String> {
    // 1. 下载代码
    build.step_source(app)?;

    // 2. 修改配置
    build.step_change(app)?;

    // 3. 开始打包
    build.step_build(app)?;

    Ok(())
}
