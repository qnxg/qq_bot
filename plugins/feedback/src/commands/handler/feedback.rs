use crate::{
    api,
    commands::framework::{CommandContext, CommandHandler},
    database,
    entities::FeedbackStatus,
    utils,
};
use anyhow::Result;
use async_trait::async_trait;
use kovi::Message;

pub struct FeedbackDetailCommand;
#[async_trait]
impl CommandHandler for FeedbackDetailCommand {
    fn command_name(&self) -> &'static str {
        "查看"
    }

    fn command_usage(&self) -> &'static str {
        "查看 <问题 id>：查看指定问题的详细信息"
    }

    async fn handle_command<'a>(&self, mut ctx: CommandContext<'a>) -> Result<Option<Message>> {
        let feedback_id = match ctx.get_feedback_id() {
            Some(id) => id,
            None => return Ok(Some(Message::new().add_text(self.command_usage()))),
        };
        if let Some(feedback) = database::get_feedback_detail(feedback_id).await? {
            Ok(Some(
                Message::new().add_text(utils::format_feedback_detail(&feedback)),
            ))
        } else {
            Ok(Some(Message::new().add_text("未找到指定 ID 的问题反馈。")))
        }
    }
}

pub struct FeedbackImageCommand;
#[async_trait]
impl CommandHandler for FeedbackImageCommand {
    fn command_name(&self) -> &'static str {
        "图片"
    }

    fn command_usage(&self) -> &'static str {
        "图片 <问题 id>：查看指定问题反馈的附加图片"
    }

    async fn handle_command<'a>(&self, mut ctx: CommandContext<'a>) -> Result<Option<Message>> {
        let feedback_id = match ctx.get_feedback_id() {
            Some(id) => id,
            None => return Ok(Some(Message::new().add_text(self.command_usage()))),
        };
        if let Some(feedback) = database::get_feedback_detail(feedback_id).await? {
            if let Some(img_url) = &feedback.img_url {
                return Ok(Some(Message::new().add_image(img_url)));
            } else {
                return Ok(Some(Message::new().add_text("该反馈没有附加图片。")));
            }
        } else {
            Ok(Some(Message::new().add_text("未找到指定 ID 的问题反馈。")))
        }
    }
}

pub struct FeedbackConfirmCommand;
#[async_trait]
impl CommandHandler for FeedbackConfirmCommand {
    fn command_name(&self) -> &'static str {
        "确认"
    }

    fn command_usage(&self) -> &'static str {
        "确认 <问题 id> [...回复内容]/#[快捷回复id]： 标记问题为已确认并回复"
    }

    async fn handle_command<'a>(&self, mut ctx: CommandContext<'a>) -> Result<Option<Message>> {
        let feedback_id = match ctx.get_feedback_id() {
            Some(id) => id,
            None => return Ok(Some(Message::new().add_text(self.command_usage()))),
        };
        let reply_content = ctx.get_content_or_fast_reply().await?;
        if let Some(feedback) = database::get_feedback_detail(feedback_id).await? {
            api::update_feedback(
                feedback_id,
                FeedbackStatus::Confirmed,
                reply_content,
                feedback.stu_id,
            )
            .await?;
            if let Some(feedback) = database::get_feedback_detail(feedback_id).await? {
                Ok(Some(
                    Message::new().add_text(utils::format_feedback_detail(&feedback)),
                ))
            } else {
                Ok(Some(
                    Message::new().add_text("内部错误：问题反馈在更新后被删除。"),
                ))
            }
        } else {
            Ok(Some(Message::new().add_text("未找到指定 ID 的问题反馈。")))
        }
    }
}

pub struct FeedbackResolveCommand;
#[async_trait]
impl CommandHandler for FeedbackResolveCommand {
    fn command_name(&self) -> &'static str {
        "解决"
    }

    fn command_usage(&self) -> &'static str {
        "解决 <问题 id> [...回复内容]/#[快捷回复id]： 标记问题为已解决并回复"
    }

    async fn handle_command<'a>(&self, mut ctx: CommandContext<'a>) -> Result<Option<Message>> {
        let feedback_id = match ctx.get_feedback_id() {
            Some(id) => id,
            None => return Ok(Some(Message::new().add_text(self.command_usage()))),
        };
        let reply_content = ctx.get_content_or_fast_reply().await?;
        if let Some(feedback) = database::get_feedback_detail(feedback_id).await? {
            api::update_feedback(
                feedback_id,
                FeedbackStatus::Resolved,
                reply_content,
                feedback.stu_id,
            )
            .await?;
            if let Some(feedback) = database::get_feedback_detail(feedback_id).await? {
                Ok(Some(
                    Message::new().add_text(utils::format_feedback_detail(&feedback)),
                ))
            } else {
                Ok(Some(
                    Message::new().add_text("内部错误：问题反馈在更新后被删除。"),
                ))
            }
        } else {
            Ok(Some(Message::new().add_text("未找到指定 ID 的问题反馈。")))
        }
    }
}

pub struct FeedbackListCommand;
#[async_trait]
impl CommandHandler for FeedbackListCommand {
    fn command_name(&self) -> &'static str {
        "列表"
    }

    fn command_usage(&self) -> &'static str {
        "列表 [未确认/已确认/已解决] [页码] [每页个数]： 查看反馈列表，默认为未确认，第 1 页，每页 5 条"
    }

    async fn handle_command<'a>(&self, mut ctx: CommandContext<'a>) -> Result<Option<Message>> {
        let status = match ctx.next_token() {
            Some("未确认") => FeedbackStatus::Unconfirmed,
            Some("已确认") => FeedbackStatus::Confirmed,
            Some("已解决") => FeedbackStatus::Resolved,
            Some(_) => return Ok(Some(Message::new().add_text(self.command_usage()))),
            None => FeedbackStatus::Unconfirmed,
        };
        let page = ctx.next_number().unwrap_or(1).max(1) as u32;
        let per_page = ctx.next_number().unwrap_or(5).clamp(1, 20) as u32;
        let total_count = database::get_feedback_count(&status).await?;
        let total_pages = (total_count + per_page - 1) / per_page;
        let feedbacks = database::get_feedback_list(&status, page, per_page).await?;
        if feedbacks.is_empty() {
            return Ok(Some(Message::new().add_text("没有找到符合条件的反馈。")));
        }
        let mut msg = feedbacks
            .iter()
            .map(|feedback| utils::format_feedback_summary(feedback))
            .collect::<Vec<_>>()
            .join("\n");
        msg.push_str(&format!(
            "\n\n第 {} / {} 页，共 {} 条反馈。",
            page, total_pages, total_count
        ));
        Ok(Some(Message::new().add_text(msg)))
    }
}
