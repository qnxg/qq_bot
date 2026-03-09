use crate::entities::{FeedbackDetail, FeedbackListResponse, FeedbackMsg, FeedbackMsgListResponse, FeedbackResponse, FeedbackStatus};
use anyhow::Result;
use once_cell::sync::Lazy;
use reqwest::{Client, header::HeaderMap, redirect::Policy};
use serde_json::json;
use std::time::Duration;

use crate::{config::CFG};

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
    let url = format!("{}/feedback/{}", CFG.yqwork.url, id);
    let mut res = CLIENT
        .get(&url)
        .send()
        .await?
        .json::<FeedbackResponse>()
        .await?;

    if let Some(feedback) = &mut res.data {
        feedback.msgs = get_feedback_msg_list(id).await?;
    }

    Ok(res.data)
}

pub async fn get_feedback_msg_list(feedback_id: u32) -> Result<Vec<FeedbackMsg>> {
    let url = format!("{}/feedback/{}/msg", CFG.yqwork.url, feedback_id);
    let res = CLIENT
        .get(&url)
        .send()
        .await?
        .json::<FeedbackMsgListResponse>()
        .await?;
    Ok(res.data)
}

pub async fn get_feedback_count(status: &FeedbackStatus) -> Result<u32> {
    let url = format!("{}/feedback?status={}&page=1&pageSize=0", CFG.yqwork.url, (*status) as i8);
    let res = CLIENT
        .get(&url)
        .send()
        .await?
        .json::<FeedbackListResponse>()
       .await?;
    Ok(res.data.count)
}

pub async fn add_feedback_msg(
    feedback_id: u32,
    msg: String,
) -> Result<()> {
    let url = format!("{}/feedback/{}/msg", CFG.yqwork.url, feedback_id);
    let body = json!({
        "typ": "comment",
        "msg": msg
    });
    
    CLIENT
        .post(&url)
        .json(&body)
        .send()
        .await?;
    
    Ok(())
}

pub async fn update_feedback_status(
    feedback_id: u32,
    status: FeedbackStatus,
) -> Result<()> {
    if let Some(feedback_detail) = get_feedback_detail(feedback_id).await? {
        if feedback_detail.status as i8 != status as i8 {
            let url = format!("{}/feedback/{}", CFG.yqwork.url, feedback_id);

            let body = json!({
                "status": i8::from(status),
            });

            CLIENT
                .put(&url)
                .json(&body)
                .send()
                .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kovi::tokio;
    use crate::entities::{FeedbackStatus};

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
            Ok(Some(_)) => {
                println!("成功获取ID为{}的反馈", test_id);
            }
            Ok(None) => {
                println!("未找到ID为{}的反馈", test_id);
            }
            Err(e) => {
                assert!(false, "测试失败: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_get_feedback_count() {

        let status = FeedbackStatus::Unconfirmed;

        let result = get_feedback_count(&status).await.unwrap();
        println!("get_feedback_count 条数：{}",result);
    }

    #[tokio::test]
    async fn test_add_feedback_msg() {
        let feedback_id = 2879;
        let msg = "测试添加消息".to_string();

        match add_feedback_msg(feedback_id, msg).await {
            Ok(_) => {
                println!("成功为反馈 ID {} 添加消息", feedback_id);
            }
            Err(e) => {
                assert!(false, "测试失败: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_update_feedback_status() {
        let feedback_id = 2879;

        let statuses = [
            FeedbackStatus::Unconfirmed,
            FeedbackStatus::Confirmed,
            FeedbackStatus::Resolved,
        ];

        for status in statuses.iter() {
            match update_feedback_status(feedback_id, *status).await {
                Ok(_) => {
                    println!("成功更新反馈 ID {} 为状态 {:?}", feedback_id, status);
                }
                Err(e) => {
                    assert!(false, "测试失败: {}", e);
                }
            }
        }
    }

}
