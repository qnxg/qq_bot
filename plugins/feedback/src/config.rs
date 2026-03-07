use std::{fs::File, io::Read};

use once_cell::sync::Lazy;
use serde::Deserialize;

pub static CFG: Lazy<Configs> = Lazy::new(init);

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

#[derive(Deserialize, Debug)]
pub struct YQWork {
    pub token: String,
    pub url: String,
}

fn init() -> Configs {
    let mut file = File::open( if cfg!(test) {"../../config.toml"} else {"config.toml"}).expect("读取配置文件失败");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("读取配置文件失败");
    toml::from_str(&contents).expect("解析配置文件失败")
}
