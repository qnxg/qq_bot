mod caddy;
mod config;
mod deploy;
mod docker;

use std::sync::Arc;

use kovi::{Message, PluginBuilder as plugin};
use once_cell::sync::Lazy;
use tokio::sync::Mutex;

use crate::config::CFG;

static DEPLOY_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[kovi::plugin]
async fn main() {
    let bot = plugin::get_runtime_bot();
    let user_id = bot.get_login_info().await.expect("获取登录信息失败").data["user_id"]
        .as_i64()
        .expect("无法解析登录信息");

    plugin::on_msg(move |event| async move {
        let Some(group_id) = event.group_id else {
            return Ok::<(), anyhow::Error>(());
        };

        if group_id.to_string() != CFG.deploy.group_id {
            return Ok(());
        }

        let sender_qq = event.user_id.to_string();
        if !CFG.deploy.admin_qq.iter().any(|qq| qq == &sender_qq) {
            return Ok(());
        }

        let mut at = false;
        let mut text = String::new();
        for item in event.message.iter() {
            match item.type_.as_str() {
                "at" => {
                    if item.data["qq"].as_str() == Some(user_id.to_string().as_str()) {
                        at = true;
                    }
                }
                "text" => {
                    text.push_str(item.data["text"].as_str().unwrap_or(""));
                }
                _ => {}
            }
        }

        if !at {
            return Ok(());
        }

        let cmd = text.trim();
        let is_deploy = cmd == "部署";
        let is_rollback = cmd == "回滚";
        if !is_deploy && !is_rollback {
            return Ok(());
        }

        // 非阻塞尝试获取锁
        let guard = match DEPLOY_LOCK.try_lock() {
            Ok(g) => g,
            Err(_) => {
                event.reply(
                    Message::new()
                        .add_text("部署或回滚进行中，请稍后再试")
                        .add_reply(event.message_id),
                );
                return Ok(());
            }
        };

        let message_id = event.message_id;
        let reply_fn = {
            let event = event.clone();
            Arc::new(move |msg: String| {
                let event = event.clone();
                Box::pin(async move {
                    event.reply(Message::new().add_text(msg).add_reply(message_id));
                })
                    as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
            })
        };

        if is_deploy {
            if let Err(e) = deploy::run_deploy(Box::new(move |msg| reply_fn(msg))).await {
                tracing::error!("部署失败: {:?}", e);
            }
        } else if let Err(e) = deploy::run_rollback(Box::new(move |msg| reply_fn(msg))).await {
            tracing::error!("回滚失败: {:?}", e);
        }
        drop(guard);

        Ok(())
    });
}
