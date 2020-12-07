use crate::build_params::AppParams;
use crate::work::*;
use async_trait::async_trait;

#[async_trait]
pub trait BuildStep {
    fn step_source(&self, app: &AppParams) -> Result<(), String> {
        fetch_source(app)
    }

    async fn step_change(&self, app: &AppParams) -> Result<(), String> {
        change_config(app)
    }

    fn step_build(&self, app: &AppParams) -> Result<(), String> {
        release_build(app)
    }

    async fn step_upload(&self, app: &mut AppParams) -> Result<(), String> {
        upload_build(app).await
    }

    async fn step(&self, app: &mut AppParams) -> Result<(), String> {
        // 1. 下载代码
        self.step_source(app)?;

        // 2. 修改配置
        self.step_change(app).await?;

        // 3. 开始打包
        self.step_build(app)?;

        // 4. 结果上传
        self.step_upload(app).await?;

        Ok(())
    }
}
