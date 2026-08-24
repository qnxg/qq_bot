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

#[derive(Deserialize, Debug, Clone)]
pub struct YQWork {
    pub uid: u32,
    pub secret: String,
    pub url: String,
}

fn init() -> Configs {
    let mut file = File::open(if cfg!(test) {
        "../../config.toml"
    } else {
        "config.toml"
    })
    .expect("读取配置文件失败");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("读取配置文件失败");
    toml::from_str(&contents).expect("解析配置文件失败")
}

impl From<&Configs> for data_client::Config {
    fn from(cfg: &Configs) -> Self {
        data_client::Config {
            rabbitmq: data_client::config::RabbitMQ {
                url: cfg.rabbitmq.url.clone(),
            },
            database: data_client::config::Database {
                database_url: cfg.database.database_url.clone(),
                max_connections: cfg.database.max_connections,
            },
            yqwork: data_client::config::YQWork {
                uid: cfg.yqwork.uid,
                secret: cfg.yqwork.secret.clone(),
                url: cfg.yqwork.url.clone(),
            },
        }
    }
}
