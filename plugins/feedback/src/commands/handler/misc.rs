use crate::commands::{
    COMMANDS,
    framework::{CommandContext, CommandHandler},
};
use anyhow::Result;
use async_trait::async_trait;
use kovi::Message;

const TIPS: &str = r#"
注：必须 @ 机器人，且在指定反馈群内才会处理消息。
注：问题 id 参数可以通过回复消息或是直接指定获得。
"#;

pub struct HelperCommand;
#[async_trait]
impl CommandHandler for HelperCommand {
    fn command_name(&self) -> &'static str {
        "帮助"
    }

    fn command_usage(&self) -> &'static str {
        "帮助\n    查看帮助信息"
    }

    async fn handle_command<'a>(&self, _ctx: CommandContext<'a>) -> Result<Option<Message>> {
        let command_list = COMMANDS
            .iter()
            .map(|item| item.command_usage().to_string())
            .collect::<Vec<String>>()
            .join("\n\n");
        Ok(Some(Message::new().add_text(format!(
            "{}\n\n{}",
            command_list,
            TIPS.trim()
        ))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::framework::CommandContext;
    use kovi::tokio;

    #[tokio::test]
    async fn test_helper_command() {
        let handler = HelperCommand;
        let ctx = CommandContext::new(Box::new(std::iter::empty()), None);
        let result = handler.handle_command(ctx).await.unwrap();
        if let Some(msg) = result {
            let text = msg
                .iter()
                .filter_map(|seg| seg.data.get("text").and_then(|v| v.as_str()))
                .collect::<Vec<_>>()
                .join("");
            println!("=== HelperCommand Output ===");
            println!("{}", text);
            println!("===========================");
        }
    }
}
