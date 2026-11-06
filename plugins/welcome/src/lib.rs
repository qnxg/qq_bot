mod config;

use kovi::{Message, PluginBuilder as plugin};
use serde_json::Value;

use crate::config::CFG;

/// 欢迎文本中代表 @新成员 的占位符
const AT_PLACEHOLDER: &str = "{at}";

#[kovi::plugin]
async fn main() {
    let notice_bot = plugin::get_runtime_bot();

    plugin::on_notice(move |event| {
        let bot = notice_bot.clone();
        async move {
            if event.notice_type != "group_increase" {
                return;
            }

            let Some(group_id) = event.get("group_id").and_then(Value::as_i64) else {
                return;
            };
            let Some(user_id) = event.get("user_id").and_then(Value::as_i64) else {
                return;
            };

            // 机器人自己被拉进群时也会触发该事件，无需欢迎自己
            if user_id == event.self_id {
                return;
            }

            if !CFG
                .welcome
                .group_ids
                .iter()
                .any(|id| id == &group_id.to_string())
            {
                return;
            }

            let template = if CFG.welcome.text.trim().is_empty() {
                config::DEFAULT_TEXT
            } else {
                &CFG.welcome.text
            };
            let message = build_message(template, user_id);
            if let Err(e) = bot.send_group_msg_return(group_id, message).await {
                tracing::error!("向群 {} 发送入群欢迎失败: {:?}", group_id, e);
            } else {
                tracing::info!("已欢迎 {} 加入群 {}", user_id, group_id);
            }
        }
    });
}

/// 按模板生成欢迎消息：`{at}` 替换为 @新成员；
/// 模板中没有 `{at}` 时，自动在消息开头 @新成员。
fn build_message(template: &str, user_id: i64) -> Message {
    let user_id = user_id.to_string();
    let mut message = Message::new();
    let mut rest = template;
    let mut has_at = false;

    while let Some(index) = rest.find(AT_PLACEHOLDER) {
        let (text, after) = rest.split_at(index);
        if !text.is_empty() {
            message = message.add_text(text);
        }
        message = message.add_at(&user_id);
        has_at = true;
        rest = &after[AT_PLACEHOLDER.len()..];
    }

    if !has_at {
        message = message.add_at(&user_id);
    }
    if !rest.is_empty() {
        message = message.add_text(rest);
    }

    message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_at_placeholder() {
        let message = build_message("{at} 欢迎", 123);
        assert_eq!(message[0].type_, "at");
        assert_eq!(message[0].data["qq"], "123");
        assert_eq!(message[1].data["text"], " 欢迎");
    }

    #[test]
    fn prepends_at_when_placeholder_missing() {
        let message = build_message("欢迎", 123);
        assert_eq!(message[0].type_, "at");
        assert_eq!(message[0].data["qq"], "123");
        assert_eq!(message[1].data["text"], "欢迎");
    }
}
