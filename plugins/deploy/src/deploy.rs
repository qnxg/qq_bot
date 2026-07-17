use anyhow::{Result, bail};
use std::future::Future;
use std::pin::Pin;

use crate::caddy;
use crate::config::{CFG, SlotConfig};
use crate::docker;

type ReplyFn = Box<dyn Fn(String) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

async fn reply(cb: &ReplyFn, msg: impl Into<String>) {
    cb(msg.into()).await;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotId {
    A,
    B,
}

impl SlotId {
    fn other(self) -> Self {
        match self {
            SlotId::A => SlotId::B,
            SlotId::B => SlotId::A,
        }
    }

    fn label(self) -> &'static str {
        match self {
            SlotId::A => "a",
            SlotId::B => "b",
        }
    }

    fn config(self) -> &'static SlotConfig {
        match self {
            SlotId::A => &CFG.deploy.docker_run.slots.a,
            SlotId::B => &CFG.deploy.docker_run.slots.b,
        }
    }
}

fn slot_by_port(port: u16) -> Option<SlotId> {
    let a = &CFG.deploy.docker_run.slots.a;
    let b = &CFG.deploy.docker_run.slots.b;
    if a.host_port == port {
        Some(SlotId::A)
    } else if b.host_port == port {
        Some(SlotId::B)
    } else {
        None
    }
}

/// 选择要启动的新槽位，以及需要在切流后停止的旧槽位（若有）
async fn pick_slots() -> Result<(SlotId, Option<SlotId>)> {
    let a = SlotId::A;
    let b = SlotId::B;
    let a_run = docker::is_container_running(a.config().name.as_str()).await?;
    let b_run = docker::is_container_running(b.config().name.as_str()).await?;

    match (a_run, b_run) {
        (true, false) => Ok((SlotId::B, Some(SlotId::A))),
        (false, true) => Ok((SlotId::A, Some(SlotId::B))),
        (false, false) => Ok((SlotId::A, None)),
        (true, true) => {
            let port = caddy::read_active_port(&CFG.deploy.caddy_import_file).await?;
            let Some(port) = port else {
                bail!("a/b 槽均在运行，且无法从 Caddy import 文件解析当前端口，请人工处理后再部署");
            };
            let Some(active) = slot_by_port(port) else {
                bail!(
                    "a/b 槽均在运行，Caddy 端口 {} 与配置的槽位端口不匹配，请人工处理后再部署",
                    port
                );
            };
            Ok((active.other(), Some(active)))
        }
    }
}

async fn fail(reply_fn: &ReplyFn, msg: impl Into<String>) -> Result<()> {
    let msg = msg.into();
    reply(reply_fn, msg.clone()).await;
    bail!(msg);
}

/// 执行完整滚动发布流程；通过 `reply_fn` 推送 QQ 进度
pub async fn run_deploy(reply_fn: ReplyFn) -> Result<()> {
    let cfg = &CFG.deploy;

    reply(&reply_fn, "正在检查镜像更新…").await;

    let need_update = match docker::images_differ(&cfg.local_image, &cfg.remote_image).await {
        Ok(v) => v,
        Err(e) => {
            return fail(
                &reply_fn,
                format!(
                    "检查镜像失败\n{}",
                    docker::truncate_output(&format!("{e:#}"), 300)
                ),
            )
            .await;
        }
    };
    if !need_update {
        reply(&reply_fn, "没有可用的更新（本地与远端镜像一致）").await;
        return Ok(());
    }

    // 拉取（带重试）
    let retries = cfg.pull_retries.max(1);
    let mut last_err = String::new();
    let mut pulled = false;
    for attempt in 1..=retries {
        reply(
            &reply_fn,
            format!(
                "正在拉取镜像 ({}/{})…\n{}",
                attempt, retries, cfg.remote_image
            ),
        )
        .await;
        match docker::pull_image(&cfg.remote_image).await {
            Ok(()) => {
                pulled = true;
                break;
            }
            Err(e) => {
                last_err = e;
                tracing::warn!("pull attempt {} failed: {}", attempt, last_err);
                if attempt < retries {
                    let backoff = std::cmp::min(2u64.pow(attempt - 1), 30);
                    reply(
                        &reply_fn,
                        format!(
                            "拉取失败，{}s 后重试…\n{}",
                            backoff,
                            docker::truncate_output(&last_err, 200)
                        ),
                    )
                    .await;
                    tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
                }
            }
        }
    }
    if !pulled {
        return fail(
            &reply_fn,
            format!(
                "镜像拉取失败（已重试 {} 次）\n{}",
                retries,
                docker::truncate_output(&last_err, 300)
            ),
        )
        .await;
    }

    // 打标：先 local -> older，再 remote -> local
    reply(&reply_fn, "正在更新本地镜像标签…").await;
    match async {
        if docker::image_exists(&cfg.local_image).await? {
            docker::tag_image(&cfg.local_image, &cfg.older_image).await?;
        }
        docker::tag_image(&cfg.remote_image, &cfg.local_image).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await
    {
        Ok(()) => {}
        Err(e) => {
            return fail(
                &reply_fn,
                format!(
                    "镜像打标失败\n{}",
                    docker::truncate_output(&format!("{e:#}"), 300)
                ),
            )
            .await;
        }
    }

    // 选槽
    let (new_slot, old_slot) = match pick_slots().await {
        Ok(v) => v,
        Err(e) => {
            return fail(
                &reply_fn,
                format!(
                    "槽位选择失败\n{}",
                    docker::truncate_output(&format!("{e:#}"), 300)
                ),
            )
            .await;
        }
    };
    let new_cfg = new_slot.config();
    reply(
        &reply_fn,
        format!(
            "正在启动槽位 {}（{}，端口 {}）…",
            new_slot.label(),
            new_cfg.name,
            new_cfg.host_port
        ),
    )
    .await;

    if let Err(e) = docker::run_container(
        new_cfg,
        &cfg.docker_run.common_args,
        &cfg.local_image,
        &cfg.project_dir,
    )
    .await
    {
        return fail(
            &reply_fn,
            format!(
                "启动容器失败\n{}",
                docker::truncate_output(&format!("{e:#}"), 300)
            ),
        )
        .await;
    }

    // 健康检查
    reply(
        &reply_fn,
        format!("等待容器健康（超时 {}s）…", cfg.health_timeout_secs),
    )
    .await;

    let health_result = docker::wait_until_healthy(
        &new_cfg.name,
        cfg.health_timeout_secs,
        cfg.health_poll_interval_secs,
    )
    .await;

    let had_healthcheck = match health_result {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("健康检查失败: {:?}", e);
            let _ = docker::stop_and_rm_container(&new_cfg.name).await;
            return fail(
                &reply_fn,
                format!(
                    "新容器未就绪，已停止新容器，未切换流量\n{}",
                    docker::truncate_output(&format!("{e:#}"), 300)
                ),
            )
            .await;
        }
    };

    // 写 Caddy + reload
    reply(&reply_fn, "正在切换 Caddy 流量…").await;
    let old_caddy = match caddy::write_import_file(
        &cfg.caddy_import_file,
        &cfg.caddy_template,
        new_cfg.host_port,
    )
    .await
    {
        Ok(old) => old,
        Err(e) => {
            let _ = docker::stop_and_rm_container(&new_cfg.name).await;
            return fail(
                &reply_fn,
                format!(
                    "写入 Caddy 配置失败，已停止新容器\n{}",
                    docker::truncate_output(&format!("{e:#}"), 300)
                ),
            )
            .await;
        }
    };

    if let Err(e) = caddy::reload_caddy().await {
        tracing::error!("caddy reload 失败: {:?}", e);
        let _ = caddy::restore_import_file(&cfg.caddy_import_file, old_caddy.as_deref()).await;
        let _ = caddy::reload_caddy().await;
        let _ = docker::stop_and_rm_container(&new_cfg.name).await;
        return fail(
            &reply_fn,
            format!(
                "Caddy reload 失败，已尝试恢复配置并停止新容器\n{}",
                docker::truncate_output(&format!("{e:#}"), 300)
            ),
        )
        .await;
    }

    // 停旧容器
    if let Some(old) = old_slot {
        let old_name = old.config().name.clone();
        reply(
            &reply_fn,
            format!("正在停止旧槽位 {}（{}）…", old.label(), old_name),
        )
        .await;
        if let Err(e) = docker::stop_container(&old_name).await {
            tracing::warn!("停止旧容器失败（流量已切换）: {:?}", e);
            reply(
                &reply_fn,
                format!(
                    "流量已切换，但停止旧容器失败: {}\n请人工检查 {}",
                    docker::truncate_output(&format!("{e:#}"), 200),
                    old_name
                ),
            )
            .await;
        }
    }

    let health_note = if had_healthcheck {
        String::new()
    } else {
        "\n（容器无 HEALTHCHECK，已按 running 视为就绪）".to_string()
    };

    reply(
        &reply_fn,
        format!(
            "部署成功\n新槽位: {} ({})\n端口: {}{}",
            new_slot.label(),
            new_cfg.name,
            new_cfg.host_port,
            health_note
        ),
    )
    .await;

    Ok(())
}
