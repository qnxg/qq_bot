use std::{fs::File, io::Read};

use once_cell::sync::Lazy;
use serde::Deserialize;

pub static CFG: Lazy<Configs> = Lazy::new(init);

#[derive(Deserialize, Debug)]
pub struct Configs {
    pub deploy: DeployConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DeployConfig {
    pub group_id: String,
    pub admin_qq: Vec<String>,

    pub remote_image: String,
    pub local_image: String,
    pub older_image: String,
    pub project_dir: String,

    #[serde(default = "default_pull_retries")]
    pub pull_retries: u32,
    #[serde(default = "default_health_timeout")]
    pub health_timeout_secs: u64,
    #[serde(default = "default_health_poll_interval")]
    pub health_poll_interval_secs: u64,

    pub caddy_import_file: String,
    pub caddy_template: String,

    pub docker_run: DockerRunConfig,
}

fn default_pull_retries() -> u32 {
    5
}

fn default_health_timeout() -> u64 {
    180
}

fn default_health_poll_interval() -> u64 {
    3
}

#[derive(Deserialize, Debug, Clone)]
pub struct DockerRunConfig {
    #[serde(default)]
    pub common_args: Vec<String>,
    pub slots: SlotsConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SlotsConfig {
    pub a: SlotConfig,
    pub b: SlotConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SlotConfig {
    pub name: String,
    pub host_port: u16,
    pub container_port: u16,
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
