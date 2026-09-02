use std::{fs::File, io::Read, sync::LazyLock};

use serde::Deserialize;

pub static CFG: LazyLock<Configs> = LazyLock::new(init);

#[derive(Deserialize, Debug)]
pub struct Configs {
    pub rabbitmq: RabbitMQ,
    pub feedback: Feedback,
    pub database: Database,
    pub yqwork: YQWork,
}

#[derive(Deserialize, Debug)]
pub struct RabbitMQ {
    pub url: String,
    pub feedback_queue: String,
}

#[derive(Deserialize, Debug)]
pub struct Feedback {
    #[allow(unused)]
    pub admin_qq: Vec<String>,
    pub group_id: String,
}

#[derive(Deserialize, Debug)]
pub struct Database {
    pub database_url: String,
    pub max_connections: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct YQWork {
    pub uid: u32,
    pub secret: String,
    pub url: String,
}

fn init() -> Configs {
    // 运行时从项目根目录启动；跑测试时 cwd 在 crate 目录下
    let path = ["config.toml", "../../config.toml"]
        .into_iter()
        .find(|p| std::path::Path::new(p).exists())
        .expect("找不到配置文件 config.toml");
    let mut file = File::open(path).expect("读取配置文件失败");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("读取配置文件失败");
    toml::from_str(&contents).expect("解析配置文件失败")
}
