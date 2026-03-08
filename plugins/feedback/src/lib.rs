mod api;
mod commands;
mod config;
mod database;
mod entities;
mod rabbitmq;
mod utils;

use std::sync::Arc;

use kovi::{Message, PluginBuilder as plugin, RuntimeBot, futures_util::StreamExt};
use lapin::{
    options::BasicConsumeOptions,
    types::FieldTable,
};

use crate::{config::CFG, entities::RabbitFeedbackMessage};

#[kovi::plugin]
async fn main() {
    let bot = plugin::get_runtime_bot();
    let user_id = bot.get_login_info().await.expect("获取登录信息失败").data["user_id"]
        .as_i64()
        .expect("无法解析登录信息");
    kovi::spawn(listen_feedback(bot.clone()));
    plugin::on_msg(move |event| async move {
        if let Some(group_id) = event.group_id {
            if group_id.to_string() != CFG.feedback.group_id {
                return Ok::<(), anyhow::Error>(());
            }
            let mut body = String::new();
            // 只有 @ 机器人，机器人才处理
            let mut at = false;
            // 通过回复消息或是指定问题 id 均可以进行反馈
            let mut target = None;
            for item in event.message.iter() {
                match item.type_.as_str() {
                    "reply" => {
                        target = item.data["id"]
                            .as_str()
                            .map(|s| s.parse::<i64>().ok())
                            .flatten();
                        body.push(' ');
                    }
                    "at" => {
                        if item.data["qq"].as_str() == Some(user_id.to_string().as_str()) {
                            at = true;
                        }
                        body.push(' ');
                    }
                    "text" => {
                        body.push_str(&item.data["text"].as_str().unwrap_or(""));
                    }
                    _ => {}
                }
            }
            if !at {
                return Ok(());
            }
            let mut feedback_id: Option<u32> = None;
            if let Some(id) = target {
                feedback_id = database::get_feedback_id_by_msg(id).await?;
            }

            let args = body
                .split_ascii_whitespace()
                .filter(|s| !s.trim().is_empty());
            match commands::parse_command(args, feedback_id).await {
                Ok(Some(reply)) => {
                    event.reply(reply.add_reply(event.message_id));
                    return Ok(());
                }
                Err(e) => {
                    event.reply(
                        Message::new()
                            .add_text(format!("处理命令时出错: {:?}", e))
                            .add_reply(event.message_id),
                    );
                    return Ok(());
                }
                Ok(None) => {}
            }
        }
        Ok(())
    });
}

async fn listen_feedback(bot: Arc<RuntimeBot>) {
    let channel = rabbitmq::get_channel().await;
    let mut consumer = channel
        .basic_consume(
            &CFG.rabbitmq.feedback_queue,
            "qq_robot",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("创建 consumer 失败");
    while let Some(delivery) = consumer.next().await {
        let delivery = delivery.expect("error in consumer");
        let data = std::str::from_utf8(&delivery.data).expect("无法解析数据");
        let feedback: RabbitFeedbackMessage = serde_json::from_str(data).expect("无法解析反馈数据");
        tracing::info!("Received feedback: #{}", feedback.id);
        let mut msg = format!(
            "收到新的反馈 #{} \n学号: {}\n{}",
            feedback.id,
            feedback.stu_id.unwrap_or("未提供".to_string()),
            feedback.desc
        );
        if let Some(_) = &feedback.img_url {
            msg.push_str(&format!("\n（含有图片）"));
        }
        match bot
            .send_group_msg_return(CFG.feedback.group_id.parse().unwrap(), msg)
            .await
        {
            Ok(msg_id) => {
                match database::update_feedback_msg_id(feedback.id as u32, msg_id).await {
                    Ok(_) => {
                        delivery
                            .ack(lapin::options::BasicAckOptions::default())
                            .await
                            .expect("ack 失败");
                    }
                    Err(e) => {
                        tracing::error!("插入反馈消息的 qqbot_msg_id 失败: {:?}", e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("发送消息失败: {:?}", e);
            }
        }
    }
    unreachable!()
}
