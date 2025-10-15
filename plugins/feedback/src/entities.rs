use serde::Deserialize;
use sqlx::types::chrono;

#[derive(Clone, Copy)]
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

#[derive(sqlx::FromRow)]
pub struct FeedbackDetail {
    pub id: i32,
    #[allow(unused)]
    pub contact: Option<String>,
    #[sqlx(rename = "createTime")]
    pub create_time: chrono::NaiveDateTime,
    pub desc: String,
    #[sqlx(rename = "imgUrl")]
    pub img_url: Option<String>,
    #[sqlx(rename = "stuId")]
    pub stu_id: Option<String>,
    #[sqlx(try_from = "Option<i8>")]
    #[allow(unused)]
    pub status: FeedbackStatus,
    pub comment: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct RabbitFeedbackMessage {
    pub stu_id: Option<String>,
    pub desc: String,
    pub img_url: Option<String>,
    pub id: u64,
}
