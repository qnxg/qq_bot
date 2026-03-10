use crate::entities::{
    FeedbackDetail,
    FeedbackList,
    FeedbackMsg,
    FeedbackStatus
};
use crate::entities::ApiResponse;
use anyhow::Result;
use jsonwebtoken::{decode, DecodingKey, EncodingKey, Header, Validation};
use once_cell::sync::Lazy;
use reqwest::{Client, Method, redirect::Policy};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use kovi::tokio::sync::RwLock;

use crate::config::CFG;

// JWT payload 结构
#[derive(Deserialize, Debug, Serialize)]
struct Payload {
    id: u32,
    exp: usize,
}

static TOKEN_CACHE: Lazy<Arc<RwLock<String>>> = Lazy::new(|| {
    Arc::new(RwLock::new(generate_token()))
});

fn generate_token() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as usize;

    let payload = Payload {
        id: CFG.yqwork.uid,
        exp: now + 60 * 60 * 8, // 8 小时过期
    };

    jsonwebtoken::encode(
        &Header::default(),
        &payload,
        &EncodingKey::from_secret(CFG.yqwork.secret.as_bytes()),
    ).expect("生成 token 失败")
}

// 获取有效 token，过期自动刷新
async fn get_token() -> String {
    let token = TOKEN_CACHE.read().await;

    let token_data = decode::<Payload>(
        &token,
        &DecodingKey::from_secret(CFG.yqwork.secret.as_bytes()),
        &Validation::default(),
    );

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as usize;

    match token_data {
        Ok(data) if data.claims.exp > now + 3600 => token.clone(),
        _ => {
            drop(token);
            let mut token = TOKEN_CACHE.write().await;
            *token = generate_token();
            token.clone()
        }
    }
}

pub static CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .connection_verbose(false)
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(6))
        .redirect(Policy::none())
        .build()
        .unwrap()
});

// 统一请求函数，自动添加 token 并从 ApiResponse 中提取数据
// GET 返回 Some(data)，POST/PUT/DELETE 返回 None
async fn request<T: for<'de> Deserialize<'de>>(method: Method, url: &str, body: Option<serde_json::Value>) -> Result<Option<T>> {
    let is_get = &method == &Method::GET;

    let token = get_token().await;
    let mut req = CLIENT.request(method, url).header("Authorization", token);

    if let Some(json) = body {
        req = req.json(&json);
    }

    let res = req.send().await?;

    if is_get {
        let response: ApiResponse<T> = res.json().await?;
        Ok(Some(response.data))
    } else {
        Ok(None)
    }
}

pub async fn get_feedback_list(
    status: &FeedbackStatus,
    page: u32,
    page_size: u32,
) -> Result<Vec<FeedbackDetail>> {
    let url = format!("{}/feedback?status={}&page={}&pageSize={}", CFG.yqwork.url, (*status) as i8, page, page_size);
    let res: FeedbackList = request(Method::GET, &url, None).await?.unwrap();
    Ok(res.rows)
}

pub async fn get_feedback_detail(id: u32) -> Result<Option<FeedbackDetail>> {
    let url = format!("{}/feedback/{}", CFG.yqwork.url, id);
    let mut res: Option<FeedbackDetail> = request(Method::GET, &url, None).await?.and_then(|x| x);

    if let Some(feedback) = &mut res {
        feedback.msgs = get_feedback_msg_list(id).await?;
    }

    Ok(res)
}

pub async fn get_feedback_msg_list(feedback_id: u32) -> Result<Vec<FeedbackMsg>> {
    let url = format!("{}/feedback/{}/msg", CFG.yqwork.url, feedback_id);
    let res: Vec<FeedbackMsg> = request(Method::GET, &url, None).await?.unwrap();
    Ok(res)
}

pub async fn get_feedback_count(status: &FeedbackStatus) -> Result<u32> {
    let url = format!("{}/feedback?status={}&page=1&pageSize=0", CFG.yqwork.url, (*status) as i8);
    let res: FeedbackList = request(Method::GET, &url, None).await?.unwrap();
    Ok(res.count)
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

    request::<()>(Method::POST, &url, Some(body)).await?;

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

            request::<()>(Method::PUT, &url, Some(body)).await?;
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
