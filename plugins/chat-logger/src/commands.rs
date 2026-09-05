use anyhow::Result;
use data_client::database;
use kovi::Message;

/// 解析并执行监听管理指令，返回需要回复的消息。
///
/// 支持的指令：
/// - `监听 <群号>`
/// - `取消监听 <群号>`
/// - `统计监听`
pub async fn handle(text: &str) -> Result<Option<Message>> {
    let mut args = text.split_ascii_whitespace().filter(|s| !s.is_empty());
    let Some(command) = args.next() else {
        return Ok(None);
    };

    match command {
        "监听" => {
            let Some(group_id) = args.next().and_then(|s| s.parse::<i64>().ok()) else {
                return Ok(Some(Message::new().add_text("用法：监听 <群号>")));
            };
            database::add_monitored_group(group_id).await?;
            Ok(Some(
                Message::new().add_text(format!("已开始监听群 {group_id} 的聊天记录。")),
            ))
        }
        "取消监听" => {
            let Some(group_id) = args.next().and_then(|s| s.parse::<i64>().ok()) else {
                return Ok(Some(Message::new().add_text("用法：取消监听 <群号>")));
            };
            let removed = database::remove_monitored_group(group_id).await?;
            let msg = if removed {
                format!("已取消监听群 {group_id}。")
            } else {
                format!("群 {group_id} 本就不在监听列表中。")
            };
            Ok(Some(Message::new().add_text(msg)))
        }
        "统计监听" => {
            let counts = database::get_monitored_group_counts().await?;
            if counts.is_empty() {
                return Ok(Some(Message::new().add_text("当前没有正在监听的群。")));
            }
            let mut total = 0i64;
            let mut lines = Vec::with_capacity(counts.len());
            for (group_id, count) in &counts {
                total += count;
                lines.push(format!("群 {group_id}：{count} 条"));
            }
            lines.push(format!(
                "共 {} 个群，累计 {} 条聊天记录。",
                counts.len(),
                total
            ));
            Ok(Some(Message::new().add_text(lines.join("\n"))))
        }
        _ => Ok(None),
    }
}
