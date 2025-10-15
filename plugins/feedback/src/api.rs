use anyhow::{Result, anyhow};
use once_cell::sync::Lazy;
use reqwest::{Client, header::HeaderMap, redirect::Policy};
use serde_json::Value;
use std::time::Duration;

use crate::{config::CFG, entities::FeedbackStatus};

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
    status: Option<FeedbackStatus>,
    comment: Option<&str>,
) -> Result<()> {
    let url = format!("{}/feedback/{}", CFG.yqwork.url, feedback_id);
    let mut body = serde_json::Map::new();
    if let Some(s) = status {
        body.insert("status".to_string(), Value::Number(i8::from(s).into()));
    }
    if let Some(c) = comment {
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
        if code.as_i64().unwrap_or(-1) == 200 {
            return Ok(());
        }
    }
    Err(anyhow!("更新问题反馈失败: {:#?}", res))
}
