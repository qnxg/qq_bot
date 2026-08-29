use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize, Debug, Clone)]
pub struct Configs {
    pub rabbitmq: RabbitMQ,
    pub feedback: Feedback,
    pub database: Database,
    pub yqwork: YQWork,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RabbitMQ {
    pub url: String,
    pub feedback_queue: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Feedback {
    #[allow(unused)]
    pub admin_qq: Vec<String>,
    pub group_id: String,
}

#[derive(Deserialize, Debug, Clone)]
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

static CONFIG: OnceLock<Configs> = OnceLock::new();

/// 注入全局配置。必须在使用 [`crate::api`]、[`crate::database`]、
/// [`crate::rabbitmq`] 之前调用一次；重复调用会被忽略。
pub fn init(config: Configs) {
    let _ = CONFIG.set(config);
}

pub(crate) fn config() -> &'static Configs {
    CONFIG
        .get()
        .expect("data-client 未初始化，请先调用 data_client::init")
}

/// 测试辅助：从项目根的 config.toml 读取配置并初始化
#[cfg(test)]
pub(crate) fn init_test_config() {
    use std::{fs::File, io::Read};

    let mut file = File::open("../../config.toml").expect("读取配置文件失败");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("读取配置文件失败");
    let config: Configs = toml::from_str(&contents).expect("解析配置文件失败");
    init(config);
}
