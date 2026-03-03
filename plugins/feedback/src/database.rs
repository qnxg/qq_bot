use crate::{
    config::CFG,
    entities::{FeedbackDetail, FeedbackStatus},
    utils,
};
use anyhow::Result;
use kovi::tokio::sync::OnceCell;
use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use std::time::Duration;

static DB_POOL: OnceCell<MySqlPool> = OnceCell::const_new();

/// # Performance
/// 参见 [`sqlx::pool::Pool`] 文档：
/// > Cloning `Pool` is cheap as it is simply a
/// > reference-counted handle to the inner pool state.
///
/// 因此实际上没有必要将[`MySqlPool`]用[`std::sync::Arc`]等包裹。
/// 可以直接调用此函数获得全局数据库池。
///
/// # Side Effects
/// 数据库连接异常时，这个函数可能会结束进程。
pub async fn get_db_pool() -> MySqlPool {
    DB_POOL
        .get_or_init(|| async {
            match MySqlPoolOptions::new()
                .max_connections(CFG.database.max_connections)
                .acquire_timeout(Duration::from_secs(3))
                .connect(&CFG.database.database_url)
                .await
            {
                Ok(pool) => {
                    tracing::info!("🔥 Successfully connected to MySQL");
                    pool
                }
                Err(e) => {
                    tracing::error!("🪨 Failed to connect to MySQL: {:?}", e);
                    std::process::exit(1);
                }
            }
        })
        .await
        .clone()
}

pub async fn get_fast_reply_list() -> Result<Vec<(String, String)>> {
    let replies = sqlx::query!(
        r#"
        SELECT id, content
        FROM fast_reply
        "#
    )
    .fetch_all(&get_db_pool().await)
    .await?
    .into_iter()
    .map(|record| (record.id, record.content))
    .collect();
    Ok(replies)
}

pub async fn get_fast_reply_content(id: &str) -> Result<Option<String>> {
    let content = sqlx::query_scalar!(
        r#"
        SELECT content
        FROM fast_reply
        WHERE id = ?
        "#,
        id
    )
    .fetch_optional(&get_db_pool().await)
    .await?;
    Ok(content)
}

pub async fn update_fast_reply(id: &str, content: &str) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO fast_reply
        (id, content)
        VALUES (?, ?)
        ON DUPLICATE KEY UPDATE content = ?
        "#,
        id,
        content,
        content
    )
    .execute(&get_db_pool().await)
    .await?;
    Ok(())
}

pub async fn delete_fast_reply(id: &str) -> Result<()> {
    sqlx::query!(
        r#"
        DELETE FROM fast_reply
        WHERE id = ?
        "#,
        id
    )
    .execute(&get_db_pool().await)
    .await?;
    Ok(())
}

pub async fn get_feedback_id_by_msg(msg_id: i64) -> Result<Option<u32>> {
    let feedback_id = sqlx::query_scalar!(
        r#"
        SELECT id
        FROM feedbacks
        WHERE qqbot_msg_id = ?
        "#,
        msg_id
    )
    .fetch_optional(&get_db_pool().await)
    .await?;
    Ok(feedback_id.map(|id| id as u32))
}
