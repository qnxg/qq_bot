use anyhow::{Result, anyhow};
use once_cell::sync::Lazy;
use reqwest::{Client, header::HeaderMap, redirect::Policy};
use serde_json::{Value, json};
use std::time::Duration;

use crate::{config::CFG, entities::FeedbackStatus, utils};

pub static CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .connection_verbose(false)
        .timeout(Duration::from_secs(6))
        .default_headers({
            let mut headers = HeaderMap::new();
            headers.insert("Authorization", CFG.yqwork.token.parse().unwrap());
            headers
        })
        .redirect(Policy::none())
        .build()
        .unwrap()
});

pub async fn update_feedback(
    feedback_id: u32,
    status: FeedbackStatus,
    comment: Option<String>,
    notice_stuid: Option<String>,
) -> Result<()> {
    let url = format!("{}/feedback/{}", CFG.yqwork.url, feedback_id);
    let mut body = serde_json::Map::new();
    body.insert("status".to_string(), Value::Number(i8::from(status).into()));
    if let Some(ref c) = comment {
        body.insert("comment".to_string(), Value::String(c.to_string()));
    }
    let res: Value = CLIENT
        .put(&url)
        .json(&body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    if let Some(code) = res["code"].as_number() {
        if code.as_i64().unwrap_or(-1) != 200 {
            return Err(anyhow!("更新问题反馈失败: {:#?}", res));
        }
    } else {
        return Err(anyhow!("更新问题反馈失败: {:#?}", res));
    }
    // 然后推送消息
    if let Some(stu_id) = notice_stuid {
        let url = format!("{}/notice", CFG.yqwork.url);
        let body = json!({
            "bindId": feedback_id,
            "bindType": "feedback",
            "btnConfig": "[{\"text\":\"已解决\",\"type\":\"button\",\"theme\":\"primary\",\"value\":\"已解决\"},{\"text\":\"未解决\",\"type\":\"button\",\"theme\":\"default\",\"value\":\"未解决\"},{\"text\":\"前往查看\",\"type\":\"link\",\"theme\":\"plain\",\"value\":\"/pages/feedbackHistory/index\"}]",
            "content": format!("您的问题反馈有了新的进展：{} 。点击下方按钮查看详情。", comment.unwrap_or(match status {
                FeedbackStatus::Unconfirmed => "状态更新为未确认".to_string(),
                FeedbackStatus::Confirmed => "已经确认该问题，正在解决...".to_string(),
                FeedbackStatus::Resolved => "问题已解决".to_string()
            })),
            "isShow": true,
            "sendTime": utils::get_now_time(),
            "stuId": stu_id
        });
        let res: Value = CLIENT
            .post(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if let Some(code) = res["code"].as_number() {
            if code.as_i64().unwrap_or(-1) != 200 {
                return Err(anyhow!("发送通知消息失败: {:#?}", res));
            }
        } else {
            return Err(anyhow!("发送通知消息失败: {:#?}", res));
        }
    }
    Ok(())
}
