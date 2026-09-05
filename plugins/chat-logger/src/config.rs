use std::{fs::File, io::Read};

use once_cell::sync::Lazy;
use serde::Deserialize;

pub static CFG: Lazy<Configs> = Lazy::new(init);

#[derive(Deserialize, Debug)]
pub struct Configs {
    pub chat_logger: ChatLogger,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChatLogger {
    /// 管理员 QQ 号列表，只有管理员 @ 机器人才能使用监听管理指令
    pub admin_qq: Vec<String>,
    /// 聊天记录中图片下载保存的目录
    #[serde(default = "default_image_dir")]
    pub image_dir: String,
    /// 聊天记录中文件下载保存的目录
    #[serde(default = "default_file_dir")]
    pub file_dir: String,
}

fn default_image_dir() -> String {
    "data/chat_images".to_string()
}

fn default_file_dir() -> String {
    "data/chat_files".to_string()
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
