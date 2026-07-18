use std::{fs::File, io::Read};

use once_cell::sync::Lazy;
use serde::Deserialize;

pub static CFG: Lazy<Configs> = Lazy::new(init);

#[derive(Deserialize, Debug)]
pub struct Configs {
    pub daily_greeting: DailyGreeting,
}

#[derive(Deserialize, Debug)]
pub struct DailyGreeting {
    pub group_id: String,
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
