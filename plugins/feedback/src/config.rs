use std::{fs::File, io::Read};

use data_client::Configs;
use once_cell::sync::Lazy;

pub static CFG: Lazy<Configs> = Lazy::new(init);

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
