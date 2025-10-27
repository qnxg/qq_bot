mod framework;
mod handler;
use crate::commands::framework::{CommandContext, CommandHandler};
use anyhow::Result;
use kovi::Message;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tracing::info;

pub async fn parse_command<'a>(
    mut args: impl Iterator<Item = &'a str> + Send + 'a,
    feedback_id: Option<u32>,
) -> Result<Option<Message>> {
    if let Some(command) = args.next() {
        info!("Parsing command: {}", command);
        let context = CommandContext::new(Box::new(args), feedback_id);
        for handler in COMMANDS.iter() {
            if handler.command_name() == command {
                info!("Use handler: {}", handler.command_name());
                return handler.handle_command(context).await;
            }
        }
        Ok(None)
    } else {
        Ok(None)
    }
}

const COMMANDS: Lazy<Vec<Arc<dyn CommandHandler>>> = Lazy::new(|| {
    vec![
        Arc::new(handler::misc::HelperCommand),
        Arc::new(handler::feedback::FeedbackListCommand),
        Arc::new(handler::feedback::FeedbackImageCommand),
        Arc::new(handler::feedback::FeedbackDetailCommand),
        Arc::new(handler::feedback::FeedbackConfirmCommand),
        Arc::new(handler::feedback::FeedbackResolveCommand),
        Arc::new(handler::fast_reply::FastReplyListCommand),
        Arc::new(handler::fast_reply::FastReplyUpdateCommand),
        Arc::new(handler::fast_reply::FastReplyDeleteCommand),
        Arc::new(handler::fast_reply::FastReplyDetailCommand),
    ]
});
