use anyhow::Result;
use async_trait::async_trait;
use kovi::Message;

use crate::database;

pub struct CommandContext<'a> {
    args: Box<dyn Iterator<Item = &'a str> + Send + 'a>,
    feedback_id: Option<u32>,
}

impl<'a> CommandContext<'a> {
    pub fn new(
        args: Box<dyn Iterator<Item = &'a str> + Send + 'a>,
        feedback_id: Option<u32>,
    ) -> Self {
        Self { args, feedback_id }
    }

    pub fn get_feedback_id(&mut self) -> Option<u32> {
        if self.feedback_id.is_some() {
            return self.feedback_id;
        }
        self.args.next().and_then(|s| s.parse::<u32>().ok())
    }

    pub fn next_token(&mut self) -> Option<&'a str> {
        self.args.next()
    }

    pub fn next_number(&mut self) -> Option<i64> {
        self.next_token().and_then(|s| s.parse::<i64>().ok())
    }

    pub fn next_fast_reply_id(&mut self) -> Option<&'a str> {
        self.next_token().and_then(|s| {
            if s.starts_with('#') {
                Some(s[1..].as_ref())
            } else {
                None
            }
        })
    }

    pub fn get_content(self) -> Option<String> {
        let content: String = self.args.collect::<Vec<&str>>().join(" ");
        if content.is_empty() {
            None
        } else {
            Some(content)
        }
    }

    pub async fn get_content_or_fast_reply(mut self) -> Result<Option<String>> {
        let next = self.args.next();
        if let Some(next) = next {
            if next.starts_with('#') {
                if let Some(fast_reply_id) = next.get(1..) {
                    return Ok(database::get_fast_reply_content(fast_reply_id).await?);
                } else {
                    return Ok(None);
                }
            } else {
                let mut content: String = String::from(next);
                for arg in self.args {
                    content.push(' ');
                    content.push_str(arg);
                }
                if content.is_empty() {
                    return Ok(None);
                } else {
                    return Ok(Some(content));
                }
            }
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
pub trait CommandHandler: Send + Sync {
    fn command_name(&self) -> &'static str;
    fn command_usage(&self) -> &'static str;

    async fn handle_command<'a>(&self, ctx: CommandContext<'a>) -> Result<Option<Message>>;
}
