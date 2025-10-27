use crate::{
    commands::framework::{CommandContext, CommandHandler},
    database, utils,
};
use anyhow::Result;
use async_trait::async_trait;
use kovi::Message;

pub struct FastReplyListCommand;
#[async_trait]
impl CommandHandler for FastReplyListCommand {
    fn command_name(&self) -> &'static str {
        "回复列表"
    }

    fn command_usage(&self) -> &'static str {
        "回复列表：获取快捷回复列表"
    }

    async fn handle_command<'a>(&self, _ctx: CommandContext<'a>) -> Result<Option<Message>> {
        let fast_replies = database::get_fast_reply_list().await?;
        if fast_replies.is_empty() {
            return Ok(Some(Message::new().add_text("当前没有任何快捷回复。")));
        }
        Ok(Some(
            Message::new().add_text(
                fast_replies
                    .iter()
                    .map(|(id, content)| {
                        format!("#{}\n{}", id, utils::truncate_string(content, 10))
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n"),
            ),
        ))
    }
}

pub struct FastReplyUpdateCommand;
#[async_trait]
impl CommandHandler for FastReplyUpdateCommand {
    fn command_name(&self) -> &'static str {
        "回复更新"
    }

    fn command_usage(&self) -> &'static str {
        "回复更新 #<快捷回复id> <...快捷回复内容>：添加/更新快捷回复"
    }

    async fn handle_command<'a>(&self, mut ctx: CommandContext<'a>) -> Result<Option<Message>> {
        let fast_reply_id = match ctx.next_fast_reply_id() {
            Some(id) => id,
            None => return Ok(Some(Message::new().add_text(self.command_usage()))),
        };
        let content = match ctx.get_content() {
            Some(c) => c,
            None => return Ok(Some(Message::new().add_text(self.command_usage()))),
        };
        database::update_fast_reply(fast_reply_id, &content).await?;
        Ok(Some(Message::new().add_text("快捷回复已更新。")))
    }
}

pub struct FastReplyDeleteCommand;
#[async_trait]
impl CommandHandler for FastReplyDeleteCommand {
    fn command_name(&self) -> &'static str {
        "回复删除"
    }

    fn command_usage(&self) -> &'static str {
        "回复删除 #<快捷回复id>：删除快捷回复"
    }

    async fn handle_command<'a>(&self, mut ctx: CommandContext<'a>) -> Result<Option<Message>> {
        let fast_reply_id = match ctx.next_fast_reply_id() {
            Some(id) => id,
            None => return Ok(Some(Message::new().add_text(self.command_usage()))),
        };
        let res = database::get_fast_reply_content(fast_reply_id).await?;
        if res.is_none() {
            return Ok(Some(Message::new().add_text("未找到指定 ID 的快捷回复。")));
        }
        database::delete_fast_reply(fast_reply_id).await?;
        Ok(Some(Message::new().add_text("快捷回复已删除。")))
    }
}
