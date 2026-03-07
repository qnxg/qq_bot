use crate::{
    entities::{FeedbackDetail,FeedbackListResponse, FeedbackStatus},
};
use anyhow::{Result, anyhow};
use once_cell::sync::Lazy;
use reqwest::{Client, header::HeaderMap, redirect::Policy};
use serde_json::{Value, json};
use std::time::Duration;

use crate::{config::CFG, utils};

pub static CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .connection_verbose(false)
        .danger_accept_invalid_certs(true)
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

pub async fn get_feedback_list(
    status: &FeedbackStatus,
    page: u32,
    page_size: u32,
) -> Result<Vec<FeedbackDetail>> {
    let url = format!("{}/feedback?status={}&page={}&pageSize={}", CFG.yqwork.url, (*status) as i8, page, page_size);
    let res = CLIENT
        .get(&url)
        .send()
        .await?
        .json::<FeedbackListResponse>()
        .await?;
    Ok(res.data.rows) 
}

pub async fn get_feedback_detail(id: u32) -> Result<Option<FeedbackDetail>> {
    todo!()
}

pub async fn get_feedback_count(status: &FeedbackStatus) -> Result<u32> {
    todo!()
}

pub async fn update_feedback(
    feedback_id: u32,
    status: FeedbackStatus,
    comment: String,
    notice_stuid: Option<String>,
) -> Result<()> {
    let url = format!("{}/feedback/{}", CFG.yqwork.url, feedback_id);
    let mut body = serde_json::Map::new();
    body.insert("status".to_string(), Value::Number(i8::from(status).into()));
    body.insert("comment".to_string(), Value::String(comment.clone()));
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
            "content": format!("您的问题反馈有了新的进展：{} 。点击下方按钮查看详情。", comment),
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

#[cfg(test)]
mod tests {
    use super::*;
    use kovi::tokio;
    use crate::entities::{FeedbackDetail, FeedbackListResponse, FeedbackStatus};

    #[tokio::test]
    async fn test_get_feedback_list() {
        
        let status = FeedbackStatus::Unconfirmed;
        let page = 1;
        let page_size = 10;
        
        let result = get_feedback_list(&status, page, page_size).await.unwrap();
        println!("get_feedback_list 条数：{}", result.len());
    }

    #[tokio::test]
    async fn test_get_feedback_detail() {

        let test_id = 1;
        
        let result = get_feedback_detail(test_id).await;
        match result {
            Ok(Some(feedback)) => {
                println!("成功获取ID为{}的反馈", test_id);
            }
            Ok(None) => {
                println!("未找到ID为{}的反馈", test_id);
            }
            Err(e) => {
                eprintln!("get_feedback_detail运行失败: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_get_feedback_count() {

        let status = FeedbackStatus::Unconfirmed;

        let result = get_feedback_count(&status).await.unwrap();
        println!("get_feedback_count 条数：{}",result);
    }

}
