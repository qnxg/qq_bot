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

/// 待写入 `chat_records` 表的一条群聊天记录，尽可能详尽地保留原始信息。
#[derive(Debug, Default, Clone)]
pub struct NewChatRecord {
    /// QQ 消息 ID
    pub message_id: i64,
    /// 来源群号
    pub group_id: i64,
    /// 发送者 QQ 号
    pub user_id: i64,
    /// 发送者昵称
    pub nickname: Option<String>,
    /// 发送者群名片
    pub card: Option<String>,
    /// 发送者群角色（owner/admin/member）
    pub role: Option<String>,
    /// 消息类型
    pub message_type: Option<String>,
    /// 消息子类型
    pub sub_type: Option<String>,
    /// 原始消息串（含 CQ 码）
    pub raw_message: Option<String>,
    /// 提取出的纯文本
    pub plain_text: Option<String>,
    /// 人类可读文本（含 \[image\] 等占位）
    pub human_text: Option<String>,
    /// JSON 数组，记录图片原始 URL 与下载到本地的路径
    pub images: Option<String>,
    /// JSON 数组，记录文件名/ID/大小/URL 与下载到本地的路径
    pub files: Option<String>,
    /// 完整的 OneBot 消息段 JSON
    pub message_json: Option<String>,
    /// 完整的原始事件 JSON
    pub original_json: Option<String>,
    /// 收到消息的机器人登录号
    pub self_id: i64,
    /// 字体
    pub font: i64,
    /// 消息事件时间戳（秒）
    pub msg_time: i64,
}
