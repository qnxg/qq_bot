use std::path::Path;
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use data_client::entities::NewChatRecord;
use kovi::RuntimeBot;
use kovi::bot::runtimebot::CanSendApi;
use kovi::{MsgEvent, NoticeEvent};
use serde_json::{Value, json};

use crate::config::CFG;

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .expect("构建 reqwest client 失败")
});

/// 收集一条群消息：下载其中的图片、文件到本地，并将尽可能详尽的信息写入数据库。
pub async fn collect(bot: &RuntimeBot, event: &MsgEvent) -> Result<()> {
    let group_id = match event.group_id {
        Some(id) => id,
        None => return Ok(()),
    };

    // 下载消息中的所有图片
    let images = download_images(event, group_id).await;
    let images_json = if images.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&images)?)
    };

    // 下载消息中的所有文件
    let files = download_message_files(bot, event, group_id).await;
    let files_json = if files.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&files)?)
    };

    let record = NewChatRecord {
        message_id: event.message_id as i64,
        group_id,
        user_id: event.user_id,
        nickname: event.sender.nickname.clone(),
        card: event.sender.card.clone(),
        role: event.sender.role.clone(),
        message_type: Some(event.message_type.clone()),
        sub_type: Some(event.sub_type.clone()),
        raw_message: Some(event.raw_message.clone()),
        plain_text: event.text.clone(),
        human_text: Some(event.human_text.clone()),
        images: images_json,
        files: files_json,
        message_json: serde_json::to_string(&event.message).ok(),
        original_json: serde_json::to_string(&event.original_json).ok(),
        self_id: event.self_id,
        font: event.font as i64,
        msg_time: event.time,
    };

    data_client::database::insert_chat_record(&record).await?;
    Ok(())
}

/// 收集一条群文件上传通知（`group_upload`）：下载文件到本地并写入数据库。
///
/// 群文件在 OneBot 协议中通常以 `group_upload` 通知事件而非消息段的形式出现，
/// 因此这里单独处理，保证「收到文件即下载」。
pub async fn collect_group_upload(bot: &RuntimeBot, event: &NoticeEvent) -> Result<()> {
    let group_id = event
        .get("group_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow!("group_upload 通知缺少 group_id"))?;
    let user_id = event.get("user_id").and_then(Value::as_i64).unwrap_or(0);
    let file_obj = event
        .get("file")
        .ok_or_else(|| anyhow!("group_upload 通知缺少 file 字段"))?;

    let saved = download_file_from_obj(bot, group_id, file_obj).await;
    let name = saved.name.clone().unwrap_or_default();
    let files_json = serde_json::to_string(&vec![saved])?;

    let record = NewChatRecord {
        message_id: 0,
        group_id,
        user_id,
        message_type: Some("notice".to_string()),
        sub_type: Some("group_upload".to_string()),
        human_text: Some(format!("[file] {name}")),
        files: Some(files_json),
        original_json: serde_json::to_string(&event.original_json).ok(),
        self_id: event.self_id,
        msg_time: event.time,
        ..Default::default()
    };

    data_client::database::insert_chat_record(&record).await?;
    Ok(())
}

/// 记录单张图片的来源与落地路径。
#[derive(serde::Serialize)]
struct SavedImage {
    /// 图片文件名（QQ 侧的 file 字段，可能为空）
    file: Option<String>,
    /// 图片原始 URL
    url: Option<String>,
    /// 下载到本地的相对路径，下载失败时为 None
    local_path: Option<String>,
}

/// 记录单个文件的来源与落地路径。
#[derive(serde::Serialize, Default)]
struct SavedFile {
    /// 文件名
    name: Option<String>,
    /// 文件 ID（用于向 OneBot 端请求下载地址）
    file_id: Option<String>,
    /// 文件大小（字节）
    size: Option<i64>,
    /// 业务 ID（部分实现下载时需要）
    busid: Option<i64>,
    /// 解析得到的下载 URL
    url: Option<String>,
    /// 下载到本地的相对路径，下载失败时为 None
    local_path: Option<String>,
}

/// 遍历消息段，下载其中的图片，返回每张图片的记录。
async fn download_images(event: &MsgEvent, group_id: i64) -> Vec<SavedImage> {
    let mut saved = Vec::new();
    for (idx, seg) in event.message.iter().enumerate() {
        if seg.type_ != "image" {
            continue;
        }
        let url = seg
            .data
            .get("url")
            .and_then(Value::as_str)
            .map(str::to_string);
        let file = seg
            .data
            .get("file")
            .and_then(Value::as_str)
            .map(str::to_string);

        let local_path = match &url {
            Some(url) => {
                match save_image(url, group_id, event.message_id, idx, file.as_deref()).await {
                    Ok(path) => Some(path),
                    Err(e) => {
                        tracing::error!("下载图片失败 ({}): {:?}", url, e);
                        None
                    }
                }
            }
            None => None,
        };

        saved.push(SavedImage {
            file,
            url,
            local_path,
        });
    }
    saved
}

/// 遍历消息段，下载其中的文件（`file` 段），返回每个文件的记录。
async fn download_message_files(
    bot: &RuntimeBot,
    event: &MsgEvent,
    group_id: i64,
) -> Vec<SavedFile> {
    let mut saved = Vec::new();
    for seg in event.message.iter() {
        if seg.type_ != "file" {
            continue;
        }
        saved.push(download_file_from_obj(bot, group_id, &seg.data).await);
    }
    saved
}

/// 从一个文件描述对象（消息段的 data，或 group_upload 通知的 file 对象）解析信息并下载。
async fn download_file_from_obj(bot: &RuntimeBot, group_id: i64, obj: &Value) -> SavedFile {
    // 不同实现字段名不完全一致，尽量兼容
    let name = obj
        .get("name")
        .or_else(|| obj.get("file"))
        .or_else(|| obj.get("file_name"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let file_id = obj
        .get("file_id")
        .or_else(|| obj.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let size = obj
        .get("size")
        .or_else(|| obj.get("file_size"))
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        });
    let busid = obj.get("busid").and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    });

    // 优先使用对象自带的 url，否则向 OneBot 端请求下载地址
    let mut url = obj.get("url").and_then(Value::as_str).map(str::to_string);
    if url.is_none()
        && let Some(id) = &file_id
    {
        url = resolve_file_url(bot, group_id, id, busid).await;
    }

    let local_path = match &url {
        Some(url) => match save_file(url, group_id, name.as_deref(), file_id.as_deref()).await {
            Ok(path) => Some(path),
            Err(e) => {
                tracing::error!("下载文件失败 ({}): {:?}", url, e);
                None
            }
        },
        None => {
            tracing::warn!(
                "无法解析文件下载地址: name={:?} file_id={:?}",
                name,
                file_id
            );
            None
        }
    };

    SavedFile {
        name,
        file_id,
        size,
        busid,
        url,
        local_path,
    }
}

/// 通过 OneBot 端的 `get_group_file_url` 接口解析群文件的下载地址。
async fn resolve_file_url(
    bot: &RuntimeBot,
    group_id: i64,
    file_id: &str,
    busid: Option<i64>,
) -> Option<String> {
    let mut params = json!({ "group_id": group_id, "file_id": file_id });
    if let Some(busid) = busid {
        params["busid"] = json!(busid);
    }
    match bot.send_api_return("get_group_file_url", params).await {
        Ok(ret) => ret
            .data
            .get("url")
            .and_then(Value::as_str)
            .map(str::to_string),
        Err(e) => {
            tracing::error!("请求 get_group_file_url 失败: {:?}", e);
            None
        }
    }
}

/// 下载单张图片并保存到配置的目录，返回保存的相对路径。
async fn save_image(
    url: &str,
    group_id: i64,
    message_id: i32,
    idx: usize,
    file: Option<&str>,
) -> Result<String> {
    let dir = Path::new(&CFG.chat_logger.image_dir).join(group_id.to_string());
    tokio::fs::create_dir_all(&dir).await?;

    let ext = image_extension(url, file);
    // 加入时间戳避免同一消息重复处理时覆盖
    let ts = now_millis();
    let filename = format!("{message_id}_{idx}_{ts}.{ext}");
    let path = dir.join(&filename);

    let bytes = CLIENT
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    tokio::fs::write(&path, &bytes).await?;

    Ok(path.to_string_lossy().into_owned())
}

/// 下载单个文件并保存到配置的目录，返回保存的相对路径。
///
/// 支持 http(s) 下载；若 OneBot 端返回的是本地文件路径（`file://` 或绝对路径），
/// 则直接复制到目标目录。
async fn save_file(
    url: &str,
    group_id: i64,
    name: Option<&str>,
    file_id: Option<&str>,
) -> Result<String> {
    let dir = Path::new(&CFG.chat_logger.file_dir).join(group_id.to_string());
    tokio::fs::create_dir_all(&dir).await?;

    let base = sanitize_filename(name.or(file_id).unwrap_or("file"));
    let ts = now_millis();
    let filename = format!("{ts}_{base}");
    let path = dir.join(&filename);

    if let Some(local) = url.strip_prefix("file://").or_else(|| {
        // 无协议头的绝对路径也当作本地文件
        if url.starts_with('/') {
            Some(url)
        } else {
            None
        }
    }) {
        tokio::fs::copy(local, &path).await?;
    } else {
        let bytes = CLIENT
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        tokio::fs::write(&path, &bytes).await?;
    }

    Ok(path.to_string_lossy().into_owned())
}

/// 从 URL 或 file 字段推断图片扩展名，无法判断时默认 jpg。
fn image_extension(url: &str, file: Option<&str>) -> String {
    let candidate = file.unwrap_or(url);
    // 去掉查询参数后取扩展名
    let without_query = candidate.split(['?', '#']).next().unwrap_or(candidate);
    let ext = Path::new(without_query)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let ext = ext.to_ascii_lowercase();
    if matches!(
        ext.as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp"
    ) {
        ext
    } else {
        "jpg".to_string()
    }
}

/// 清理文件名，去除路径分隔符等不安全字符，避免目录穿越。
fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            _ => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.');
    if trimmed.is_empty() {
        "file".to_string()
    } else {
        trimmed.to_string()
    }
}

/// 当前 Unix 毫秒时间戳。
fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
