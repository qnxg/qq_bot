use anyhow::{Context, Result, bail};
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;

use crate::config::SlotConfig;

/// 截断过长输出，便于 QQ 摘要与日志
pub fn truncate_output(s: &str, max_len: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len).collect();
        format!("{truncated}...")
    }
}

async fn run_cmd(
    program: &str,
    args: &[&str],
    cwd: Option<&Path>,
) -> Result<(i32, String, String)> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd
        .output()
        .await
        .with_context(|| format!("执行命令失败: {} {}", program, args.join(" ")))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    Ok((code, stdout, stderr))
}

fn extract_digest(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() || s == "<no value>" {
        return None;
    }
    // RepoDigests 形如 repo@sha256:... 或直接 sha256:...
    if let Some(idx) = s.rfind('@') {
        Some(s[idx + 1..].to_string())
    } else if s.starts_with("sha256:") {
        Some(s.to_string())
    } else if let Some(idx) = s.find("sha256:") {
        // 从行内截取 digest
        let rest = &s[idx..];
        let end = rest
            .find(|c: char| c.is_whitespace() || c == '"' || c == ',')
            .unwrap_or(rest.len());
        Some(rest[..end].to_string())
    } else {
        None
    }
}

/// 本地镜像 digest；镜像不存在返回 None
pub async fn local_image_digest(image: &str) -> Result<Option<String>> {
    let (code, stdout, stderr) = run_cmd(
        "docker",
        &[
            "image",
            "inspect",
            "--format",
            "{{index .RepoDigests 0}}",
            image,
        ],
        None,
    )
    .await?;
    if code != 0 {
        // 镜像不存在
        if stderr.contains("No such object") || stderr.contains("No such image") {
            return Ok(None);
        }
        // 也可能没有 RepoDigests
        let (code2, stdout2, _) = run_cmd(
            "docker",
            &["image", "inspect", "--format", "{{.Id}}", image],
            None,
        )
        .await?;
        if code2 != 0 {
            return Ok(None);
        }
        return Ok(extract_digest(&stdout2).or_else(|| Some(stdout2.trim().to_string())));
    }
    Ok(extract_digest(&stdout))
}

/// 远端镜像 digest：优先 buildx imagetools，回退 manifest inspect
pub async fn remote_image_digest(image: &str) -> Result<String> {
    // 1) docker buildx imagetools inspect
    let (code, stdout, stderr) = run_cmd(
        "docker",
        &["buildx", "imagetools", "inspect", "--raw", image],
        None,
    )
    .await?;
    if code == 0 {
        // --raw 输出 manifest JSON，不一定含 digest；改用默认输出解析
        let (code2, stdout2, _) =
            run_cmd("docker", &["buildx", "imagetools", "inspect", image], None).await?;
        if code2 == 0 {
            for line in stdout2.lines() {
                if line.to_lowercase().contains("digest:")
                    && let Some(d) = extract_digest(line)
                {
                    return Ok(d);
                }
            }
        }
        // 尝试从 raw 的 config 或其他字段找
        if let Some(d) = extract_digest(&stdout) {
            return Ok(d);
        }
    } else {
        tracing::debug!(
            "buildx imagetools 不可用: {}",
            truncate_output(&stderr, 200)
        );
    }

    // 2) docker manifest inspect
    let (code, stdout, stderr) = run_cmd("docker", &["manifest", "inspect", image], None).await?;
    if code != 0 {
        bail!("无法获取远端镜像 digest: {}", truncate_output(&stderr, 300));
    }

    // 解析 JSON 中的 Descriptor.digest 或 config.digest
    if let Ok(v) = serde_json_lite_digest(&stdout) {
        return Ok(v);
    }

    for line in stdout.lines() {
        if line.contains("sha256:")
            && let Some(d) = extract_digest(line)
        {
            return Ok(d);
        }
    }

    bail!(
        "无法从 manifest 输出解析 digest: {}",
        truncate_output(&stdout, 300)
    );
}

/// 轻量解析 digest，避免额外依赖 serde_json（若已有可改用）
fn serde_json_lite_digest(s: &str) -> Result<String> {
    // 优先找 "digest": "sha256:..."
    const KEY: &str = "\"digest\"";
    let mut search_from = 0;
    while let Some(pos) = s[search_from..].find(KEY) {
        let abs = search_from + pos + KEY.len();
        let rest = &s[abs..];
        if let Some(colon) = rest.find(':') {
            let after = rest[colon + 1..].trim_start();
            if let Some(stripped) = after.strip_prefix('"')
                && let Some(end) = stripped.find('"')
            {
                let dig = &stripped[..end];
                if dig.starts_with("sha256:") {
                    return Ok(dig.to_string());
                }
            }
        }
        search_from = abs;
    }
    bail!("JSON 中未找到 digest")
}

pub async fn images_differ(local: &str, remote: &str) -> Result<bool> {
    let local_dig = local_image_digest(local).await?;
    match local_dig {
        None => {
            tracing::info!("本地镜像 {} 不存在，视为需要更新", local);
            Ok(true)
        }
        Some(ld) => {
            let rd = remote_image_digest(remote).await?;
            tracing::info!("本地 digest={}, 远端 digest={}", ld, rd);
            Ok(ld != rd)
        }
    }
}

pub async fn pull_image(image: &str) -> Result<(), String> {
    let (code, stdout, stderr) = run_cmd("docker", &["pull", image], None)
        .await
        .map_err(|e| e.to_string())?;
    if code != 0 {
        return Err(format!(
            "docker pull 失败 (exit {}): {}",
            code,
            truncate_output(&format!("{}\n{}", stdout, stderr), 400)
        ));
    }
    Ok(())
}

pub async fn image_exists(image: &str) -> Result<bool> {
    let (code, _, _) = run_cmd("docker", &["image", "inspect", image], None).await?;
    Ok(code == 0)
}

pub async fn tag_image(src: &str, dst: &str) -> Result<()> {
    let (code, _, stderr) = run_cmd("docker", &["tag", src, dst], None).await?;
    if code != 0 {
        bail!(
            "docker tag {} -> {} 失败: {}",
            src,
            dst,
            truncate_output(&stderr, 300)
        );
    }
    Ok(())
}

pub async fn is_container_running(name: &str) -> Result<bool> {
    let (code, stdout, _) = run_cmd(
        "docker",
        &["inspect", "-f", "{{.State.Running}}", name],
        None,
    )
    .await?;
    if code != 0 {
        return Ok(false);
    }
    Ok(stdout.trim().eq_ignore_ascii_case("true"))
}

pub async fn container_exists(name: &str) -> Result<bool> {
    let (code, _, _) = run_cmd("docker", &["inspect", name], None).await?;
    Ok(code == 0)
}

pub async fn start_container(name: &str) -> Result<()> {
    let (code, _, stderr) = run_cmd("docker", &["start", name], None).await?;
    if code != 0 {
        bail!(
            "docker start {} 失败: {}",
            name,
            truncate_output(&stderr, 300)
        );
    }
    Ok(())
}

/// 基于镜像 ID 安全交换两个 tag，避免互相覆盖丢失引用
pub async fn swap_image_tags(local: &str, older: &str) -> Result<()> {
    let (code_local, id_local, stderr_local) = run_cmd(
        "docker",
        &["image", "inspect", "--format", "{{.Id}}", local],
        None,
    )
    .await?;
    if code_local != 0 {
        bail!(
            "无法获取镜像 {} 的 ID: {}",
            local,
            truncate_output(&stderr_local, 300)
        );
    }
    let (code_older, id_older, stderr_older) = run_cmd(
        "docker",
        &["image", "inspect", "--format", "{{.Id}}", older],
        None,
    )
    .await?;
    if code_older != 0 {
        bail!(
            "无法获取镜像 {} 的 ID: {}",
            older,
            truncate_output(&stderr_older, 300)
        );
    }
    let id_local = id_local.trim();
    let id_older = id_older.trim();
    // 先用 ID 打标，互不依赖当前 tag 指向
    tag_image(id_older, local).await?;
    tag_image(id_local, older).await?;
    Ok(())
}

pub async fn run_container(
    slot: &SlotConfig,
    common_args: &[String],
    image: &str,
    project_dir: &str,
) -> Result<String> {
    // 若同名容器已存在（停止状态），先移除以免名字冲突
    let (exists_code, _, _) = run_cmd("docker", &["inspect", &slot.name], None).await?;
    if exists_code == 0 {
        let _ = run_cmd("docker", &["rm", "-f", &slot.name], None).await?;
    }

    let port_map = format!("{}:{}", slot.host_port, slot.container_port);
    let mut args: Vec<&str> = vec!["run", "-d", "--name", &slot.name, "-p", &port_map];
    let common_refs: Vec<&str> = common_args.iter().map(|s| s.as_str()).collect();
    args.extend(common_refs);
    args.push(image);

    let cwd = Path::new(project_dir);
    let (code, stdout, stderr) = run_cmd("docker", &args, Some(cwd)).await?;
    if code != 0 {
        bail!(
            "docker run 失败: {}",
            truncate_output(&format!("{}\n{}", stdout, stderr), 400)
        );
    }
    Ok(stdout.trim().to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    RunningNoHealthcheck,
    Unhealthy,
    Starting,
    NotRunning,
    Unknown(String),
}

pub async fn container_health(name: &str) -> Result<HealthStatus> {
    let (code, running_out, _) = run_cmd(
        "docker",
        &["inspect", "-f", "{{.State.Running}}", name],
        None,
    )
    .await?;
    if code != 0 || !running_out.trim().eq_ignore_ascii_case("true") {
        return Ok(HealthStatus::NotRunning);
    }

    let (code, health_out, _) = run_cmd(
        "docker",
        &["inspect", "-f", "{{.State.Health.Status}}", name],
        None,
    )
    .await?;
    if code != 0 {
        // 无 HEALTHCHECK 时模板可能报错或为空
        return Ok(HealthStatus::RunningNoHealthcheck);
    }
    let status = health_out.trim();
    if status.is_empty() || status == "<no value>" || status == "<nil>" {
        return Ok(HealthStatus::RunningNoHealthcheck);
    }
    Ok(match status {
        "healthy" => HealthStatus::Healthy,
        "unhealthy" => HealthStatus::Unhealthy,
        "starting" => HealthStatus::Starting,
        other => HealthStatus::Unknown(other.to_string()),
    })
}

/// 轮询直至 healthy / running(无健康检查) 或超时
/// 返回 Ok(had_healthcheck) 或 Err
pub async fn wait_until_healthy(
    name: &str,
    timeout_secs: u64,
    poll_interval_secs: u64,
) -> Result<bool> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    let interval = Duration::from_secs(poll_interval_secs.max(1));

    loop {
        match container_health(name).await? {
            HealthStatus::Healthy => return Ok(true),
            HealthStatus::RunningNoHealthcheck => return Ok(false),
            HealthStatus::Unhealthy => {
                bail!("容器 {} 健康检查失败 (unhealthy)", name);
            }
            HealthStatus::NotRunning => {
                bail!("容器 {} 未在运行", name);
            }
            HealthStatus::Starting | HealthStatus::Unknown(_) => {}
        }

        if tokio::time::Instant::now() >= deadline {
            bail!("等待容器 {} 健康超时 ({}s)", name, timeout_secs);
        }
        sleep(interval).await;
    }
}

pub async fn stop_container(name: &str) -> Result<()> {
    let (code, _, stderr) = run_cmd("docker", &["stop", name], None).await?;
    if code != 0 {
        // 已停止可忽略
        if stderr.contains("No such container") {
            return Ok(());
        }
        bail!(
            "docker stop {} 失败: {}",
            name,
            truncate_output(&stderr, 300)
        );
    }
    Ok(())
}

pub async fn stop_and_rm_container(name: &str) -> Result<()> {
    let _ = run_cmd("docker", &["stop", name], None).await;
    let (code, _, stderr) = run_cmd("docker", &["rm", name], None).await?;
    if code != 0 && !stderr.contains("No such container") {
        bail!("docker rm {} 失败: {}", name, truncate_output(&stderr, 300));
    }
    Ok(())
}
