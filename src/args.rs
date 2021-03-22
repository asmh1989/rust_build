use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "rust_build")]
pub struct Opt {
    #[structopt(short = "v", long, help = "显示版本")]
    pub version: bool,

    #[structopt(long = "manager", help = "打包管理服务")]
    pub manager: bool,

    #[structopt(long = "ding", help = "钉钉通知")]
    pub ding: bool,

    #[structopt(long = "no-upload", help = "打包结果不上传到文件服务器")]
    pub no_upload: bool,

    #[structopt(long = "manager-build", help = "打包管理服务, 同时进行打包任务")]
    pub manager_build: bool,

    #[structopt(short = "p", long = "port", default_value = "7002", help = "端口")]
    pub port: u16,

    #[structopt(
        short = "s",
        long = "sql",
        default_value = "192.168.2.36:27017",
        help = "mongodb 服务地址"
    )]
    pub sql: String,

    #[structopt(
        short = "i",
        long = "ip",
        default_value = "",
        help = "服务名称(一般用ip表示)"
    )]
    pub ip: String,

    #[structopt(
        short = "r",
        long = "redis",
        default_value = "192.168.2.36:6379",
        help = "redis服务地址"
    )]
    pub redis: String,

    #[structopt(short = "c", long = "cache", default_value = "", help = "缓存路径")]
    pub cache_path: String,
    #[structopt(
        short = "a",
        long = "android",
        default_value = "",
        help = "android sdk路径"
    )]
    pub android_home: String,
}
