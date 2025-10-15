use kovi::tokio::sync::OnceCell;
use lapin::{Channel, Connection, ConnectionProperties};

use crate::config::CFG;

static RABBIT_CHANNEL: OnceCell<Channel> = OnceCell::const_new();

pub async fn get_channel() -> Channel {
    RABBIT_CHANNEL
        .get_or_init(|| async {
            let conn = match Connection::connect(&CFG.rabbitmq.url, ConnectionProperties::default())
                .await
            {
                Ok(conn) => {
                    tracing::info!("🔥 Successfully connected to RabbitMQ");
                    conn
                }
                Err(e) => {
                    tracing::error!("🪨 Failed to connect to RabbitMQ: {:?}", e);
                    std::process::exit(1);
                }
            };
            match conn.create_channel().await {
                Ok(channel) => {
                    tracing::info!("🔥 Successfully create RabbitMQ channel");
                    channel
                }
                Err(e) => {
                    tracing::error!("🪨 Failed to create RabbitMQ channel: {:?}", e);
                    std::process::exit(1);
                }
            }
        })
        .await
        .clone()
}
