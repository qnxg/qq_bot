use std::{fs::File, io::Read};

use once_cell::sync::Lazy;
use serde::Deserialize;

pub static CFG: Lazy<Configs> = Lazy::new(init);

/// 未配置欢迎文本时使用的默认文本
pub const DEFAULT_TEXT: &str = "{at} 欢迎来到湖大反考研/反保研/反本科搞科研/反升学/反计拔群";

#[derive(Deserialize, Debug, Default)]
pub struct Configs {
    /// 入群欢迎配置；未配置 `[welcome]` 节时默认不启用
    #[serde(default)]
    pub welcome: Welcome,
}

#[derive(Deserialize, Debug, Default)]
pub struct Welcome {
    /// 启用入群欢迎的群号列表
    #[serde(default)]
    pub group_ids: Vec<String>,
    /// 欢迎文本模板，`{at}` 会被替换为 @新成员；未包含 `{at}` 时自动在开头 @新成员
    #[serde(default = "default_text")]
    pub text: String,
}

fn default_text() -> String {
    DEFAULT_TEXT.to_string()
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
