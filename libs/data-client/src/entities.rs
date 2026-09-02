use serde::{Deserialize, Serialize};
use sqlx::types::chrono;

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(from = "Option<i8>")]
pub enum FeedbackStatus {
    Unconfirmed,
    Confirmed,
    Resolved,
}

impl From<Option<i8>> for FeedbackStatus {
    fn from(value: Option<i8>) -> Self {
        match value {
            Some(0) => FeedbackStatus::Unconfirmed,
            Some(1) | Some(2) => FeedbackStatus::Confirmed,
            Some(3) => FeedbackStatus::Resolved,
            None => FeedbackStatus::Unconfirmed,
            _ => panic!("Invalid feedback status: {:?}", value),
        }
    }
}

impl From<FeedbackStatus> for i8 {
    fn from(value: FeedbackStatus) -> Self {
        match value {
            FeedbackStatus::Unconfirmed => 0,
            FeedbackStatus::Confirmed => 1,
            FeedbackStatus::Resolved => 3,
        }
    }
}

impl std::fmt::Debug for FeedbackStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status_str = match self {
            FeedbackStatus::Unconfirmed => "Unconfirmed",
            FeedbackStatus::Confirmed => "Confirmed",
            FeedbackStatus::Resolved => "Resolved",
        };
        write!(f, "{}", status_str)
    }
}

#[derive(Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

#[derive(Deserialize)]
pub struct FeedbackList {
    pub rows: Vec<FeedbackDetail>,
    pub count: u32,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum FeedbackMsgType {
    Comment,
}

impl From<String> for FeedbackMsgType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "comment" => FeedbackMsgType::Comment,
            _ => FeedbackMsgType::Comment,
        }
    }
}

impl From<FeedbackMsgType> for String {
    fn from(value: FeedbackMsgType) -> Self {
        let s = match value {
            FeedbackMsgType::Comment => "comment",
        };
        s.to_string()
    }
}

#[derive(Serialize, Deserialize)]
pub struct FeedbackMsg {
    pub id: u32,
    pub typ: FeedbackMsgType,
    pub msg: Option<String>,
    #[serde(rename = "feedbackId")]
    pub feedback_id: u32,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Serialize, Deserialize)]
pub struct FeedbackDetail {
    pub id: i32,
    #[allow(unused)]
    pub contact: Option<String>,
    #[serde(rename = "createdAt")]
    pub create_time: chrono::NaiveDateTime,
    pub desc: String,
    #[serde(rename = "imgUrl")]
    pub img_url: Option<String>,
    #[serde(rename = "stuId")]
    pub stu_id: Option<String>,
    #[allow(unused)]
    pub status: FeedbackStatus,
    #[serde(rename = "updatedAt")]
    pub update_time: chrono::NaiveDateTime,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub msgs: Vec<FeedbackMsg>,
}

#[derive(Deserialize, Debug)]
pub struct RabbitFeedbackMessage {
    pub stu_id: Option<String>,
    pub desc: String,
    pub img_url: Option<String>,
    pub id: u64,
}
