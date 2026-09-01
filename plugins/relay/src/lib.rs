use std::sync::Arc;

use data_client::{CFG, rabbitmq};
use kovi::{PluginBuilder as plugin, RuntimeBot, futures_util::StreamExt};
use lapin::{options::BasicConsumeOptions, types::FieldTable};

#[kovi::plugin]
async fn main() {
    let bot = plugin::get_runtime_bot();
    kovi::spawn(listen(bot));
}

async fn listen(bot: Arc<RuntimeBot>) {
    let channel = rabbitmq::get_channel().await;
    // consumer tag 需与 feedback 插件的 "qq_robot" 不同，两者共用同一个 channel
    let mut consumer = channel
        .basic_consume(
            &CFG.relay.message_queue,
            "qq_robot_relay",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("创建 consumer 失败");
    while let Some(delivery) = consumer.next().await {
        let delivery = delivery.expect("error in consumer");
        let text = String::from_utf8_lossy(&delivery.data);
        match bot
            .send_group_msg_return(CFG.relay.group_id.parse().unwrap(), text.to_string())
            .await
        {
            Ok(_) => {
                delivery
                    .ack(lapin::options::BasicAckOptions::default())
                    .await
                    .expect("ack 失败");
            }
            Err(e) => {
                tracing::error!("转发消息失败: {:?}", e);
            }
        }
    }
    unreachable!()
}
