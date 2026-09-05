mod collector;
mod commands;
mod config;

use data_client::database;
use kovi::{Message, PluginBuilder as plugin};

use crate::config::CFG;

#[kovi::plugin]
async fn main() {
    let bot = plugin::get_runtime_bot();
    let self_id = bot.get_login_info().await.expect("获取登录信息失败").data["user_id"]
        .as_i64()
        .expect("无法解析登录信息");

    let msg_bot = bot.clone();
    plugin::on_msg(move |event| {
        let bot = msg_bot.clone();
        async move {
            let Some(group_id) = event.group_id else {
                return Ok::<(), anyhow::Error>(());
            };

            // 1. 先尝试处理监听管理指令（需管理员 @ 机器人）
            if let Some(reply) = try_handle_command(&event, self_id).await? {
                event.reply(reply.add_reply(event.message_id));
                return Ok(());
            }

            // 2. 若消息来源群正在被监听，则收集聊天记录
            if database::is_group_monitored(group_id).await?
                && let Err(e) = collector::collect(&bot, &event).await
            {
                tracing::error!("收集群 {} 聊天记录失败: {:?}", group_id, e);
            }

            Ok(())
        }
    });

    // 群文件通常以 group_upload 通知事件出现，单独收集，保证「收到文件即下载」
    let notice_bot = bot.clone();
    plugin::on_notice(move |event| {
        let bot = notice_bot.clone();
        async move {
            if event.notice_type != "group_upload" {
                return;
            }
            let Some(group_id) = event.get("group_id").and_then(|v| v.as_i64()) else {
                return;
            };
            match database::is_group_monitored(group_id).await {
                Ok(true) => {
                    if let Err(e) = collector::collect_group_upload(&bot, &event).await {
                        tracing::error!("收集群 {} 上传文件失败: {:?}", group_id, e);
                    }
                }
                Ok(false) => {}
                Err(e) => tracing::error!("查询群监听状态失败: {:?}", e),
            }
        }
    });
}

/// 尝试将消息解析为监听管理指令。仅当发送者为管理员且 @ 了机器人时才处理，
/// 返回 `Ok(Some(reply))` 表示命中指令并需要回复。
async fn try_handle_command(
    event: &kovi::MsgEvent,
    self_id: i64,
) -> anyhow::Result<Option<Message>> {
    let sender_qq = event.user_id.to_string();
    if !CFG.chat_logger.admin_qq.iter().any(|qq| qq == &sender_qq) {
        return Ok(None);
    }

    let mut at = false;
    let mut text = String::new();
    for item in event.message.iter() {
        match item.type_.as_str() {
            "at" => {
                if item.data["qq"].as_str() == Some(self_id.to_string().as_str()) {
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
        return Ok(None);
    }

    commands::handle(text.trim()).await
}
