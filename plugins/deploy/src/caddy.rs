use anyhow::{Context, Result, bail};
use tokio::fs;
use tokio::process::Command;

use crate::docker::truncate_output;

/// 按模板写入 Caddy import 文件（`{port}` 替换为 host_port），返回写入前的旧内容（若存在）
pub async fn write_import_file(
    path: &str,
    template: &str,
    host_port: u16,
) -> Result<Option<String>> {
    let content = template.replace("{port}", &host_port.to_string());
    let old = match fs::read_to_string(path).await {
        Ok(s) => Some(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e).with_context(|| format!("读取 Caddy import 文件失败: {}", path)),
    };
    if let Some(parent) = std::path::Path::new(path).parent() {
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("创建目录失败: {}", parent.display()))?;
    }
    fs::write(path, content.as_bytes())
        .await
        .with_context(|| format!("写入 Caddy import 文件失败: {}", path))?;
    Ok(old)
}

pub async fn restore_import_file(path: &str, old: Option<&str>) -> Result<()> {
    match old {
        Some(content) => {
            fs::write(path, content.as_bytes())
                .await
                .with_context(|| format!("恢复 Caddy import 文件失败: {}", path))?;
        }
        None => {
            let _ = fs::remove_file(path).await;
        }
    }
    Ok(())
}

pub async fn reload_caddy() -> Result<()> {
    let output = Command::new("systemctl")
        .args(["reload", "caddy"])
        .output()
        .await
        .context("执行 systemctl reload caddy 失败")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!(
            "systemctl reload caddy 失败: {}",
            truncate_output(&format!("{}\n{}", stdout, stderr), 400)
        );
    }
    Ok(())
}

/// 从 import 文件内容中解析当前反向代理的 host_port
pub fn parse_active_port(content: &str) -> Option<u16> {
    // 匹配 127.0.0.1:PORT 或 localhost:PORT
    for token in content.split_whitespace() {
        for prefix in ["127.0.0.1:", "localhost:", "[::1]:"] {
            if let Some(port_str) = token.strip_prefix(prefix) {
                // 去掉可能的尾部标点
                let port_str = port_str.trim_end_matches(|c: char| !c.is_ascii_digit());
                if let Ok(p) = port_str.parse::<u16>() {
                    return Some(p);
                }
            }
        }
    }
    // 宽松：找任意 :数字
    for part in content.split(':') {
        let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty()
            && let Ok(p) = digits.parse::<u16>()
            && p > 0
        {
            return Some(p);
        }
    }
    None
}

pub async fn read_active_port(path: &str) -> Result<Option<u16>> {
    match fs::read_to_string(path).await {
        Ok(content) => Ok(parse_active_port(&content)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("读取 Caddy import 文件失败: {}", path)),
    }
}
