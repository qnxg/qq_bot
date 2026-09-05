use crate::config::CFG;
use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;
use std::time::Duration;
use tokio::sync::OnceCell;

static DB_POOL: OnceCell<SqlitePool> = OnceCell::const_new();

/// 数据库初始化脚本，随二进制一起编译进来，启动时对 local.db 执行。
const INIT_SQL: &str = include_str!("../../../init.sql");

/// # Performance
/// 参见 [`sqlx::pool::Pool`] 文档：
/// > Cloning `Pool` is cheap as it is simply a
/// > reference-counted handle to the inner pool state.
///
/// 因此实际上没有必要将[`SqlitePool`]用[`std::sync::Arc`]等包裹。
/// 可以直接调用此函数获得全局数据库池。
///
/// 首次获取连接池时会自动创建数据库文件（若不存在），并执行 `init.sql`
/// 中的建表语句，因此所有插件共用的 `local.db` 表结构总是最新的。
///
/// # Side Effects
/// 数据库连接异常时，这个函数可能会结束进程。
pub async fn get_db_pool() -> SqlitePool {
    DB_POOL
        .get_or_init(|| async {
            let options = match SqliteConnectOptions::from_str(&CFG.database.database_url) {
                Ok(options) => options.create_if_missing(true),
                Err(e) => {
                    tracing::error!("🪨 Invalid database url: {:?}", e);
                    std::process::exit(1);
                }
            };
            let pool = match SqlitePoolOptions::new()
                .max_connections(CFG.database.max_connections)
                .acquire_timeout(Duration::from_secs(3))
                .connect_with(options)
                .await
            {
                Ok(pool) => {
                    tracing::info!("🔥 Successfully connected to SQLite");
                    pool
                }
                Err(e) => {
                    tracing::error!("🪨 Failed to connect to SQLite: {:?}", e);
                    std::process::exit(1);
                }
            };
            // 启动时执行 init.sql，保证 local.db 的表结构已初始化
            if let Err(e) = sqlx::raw_sql(INIT_SQL).execute(&pool).await {
                tracing::error!("🪨 Failed to run init.sql: {:?}", e);
                std::process::exit(1);
            }
            tracing::info!("🔥 Successfully initialized local.db schema");
            pool
        })
        .await
        .clone()
}

pub async fn get_fast_reply_list() -> Result<Vec<(String, String)>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, content
        FROM fast_reply
        "#
    )
    .fetch_all(&get_db_pool().await)
    .await?;

    let replies = rows
        .into_iter()
        .filter_map(|row| row.id.map(|id| (id, row.content)))
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
        ON CONFLICT(id) DO UPDATE SET content = ?
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
    let feedback_id = sqlx::query!(
        r#"
        SELECT feedback_id
        FROM feedbacks
        WHERE qqbot_msg_id = ?
        "#,
        msg_id
    )
    .fetch_optional(&get_db_pool().await)
    .await?;
    Ok(feedback_id.and_then(|row| row.feedback_id.map(|id| id as u32)))
}

pub async fn update_feedback_msg_id(feedback_id: u32, msg_id: i32) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO feedbacks (feedback_id, qqbot_msg_id)
        VALUES (?, ?)
        "#,
        feedback_id,
        msg_id
    )
    .execute(&get_db_pool().await)
    .await?;
    Ok(())
}

/// 将群加入监听列表，重复添加不会报错。
pub async fn add_monitored_group(group_id: i64) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO monitored_groups (group_id)
        VALUES (?)
        ON CONFLICT(group_id) DO NOTHING
        "#,
        group_id
    )
    .execute(&get_db_pool().await)
    .await?;
    Ok(())
}

/// 将群从监听列表中移除，返回是否确实删除了记录。
pub async fn remove_monitored_group(group_id: i64) -> Result<bool> {
    let affected = sqlx::query!(
        r#"
        DELETE FROM monitored_groups
        WHERE group_id = ?
        "#,
        group_id
    )
    .execute(&get_db_pool().await)
    .await?
    .rows_affected();
    Ok(affected > 0)
}

/// 查询某个群是否正在被监听。
pub async fn is_group_monitored(group_id: i64) -> Result<bool> {
    let row = sqlx::query!(
        r#"
        SELECT 1 AS exist
        FROM monitored_groups
        WHERE group_id = ?
        "#,
        group_id
    )
    .fetch_optional(&get_db_pool().await)
    .await?;
    Ok(row.is_some())
}

/// 统计每个被监听群下已收集的聊天记录数量。
///
/// 返回 `(群号, 记录数)` 列表，包含尚无记录的群（记录数为 0）。
pub async fn get_monitored_group_counts() -> Result<Vec<(i64, i64)>> {
    let rows = sqlx::query!(
        r#"
        SELECT m.group_id AS "group_id!: i64", COUNT(c.id) AS "count!: i64"
        FROM monitored_groups m
        LEFT JOIN chat_records c ON c.group_id = m.group_id
        GROUP BY m.group_id
        ORDER BY m.group_id
        "#
    )
    .fetch_all(&get_db_pool().await)
    .await?;
    Ok(rows.into_iter().map(|r| (r.group_id, r.count)).collect())
}

/// 插入一条群聊天记录。
pub async fn insert_chat_record(record: &crate::entities::NewChatRecord) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO chat_records (
            message_id, group_id, user_id, nickname, card, role,
            message_type, sub_type, raw_message, plain_text, human_text,
            images, files, message_json, original_json, self_id, font, msg_time
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        record.message_id,
        record.group_id,
        record.user_id,
        record.nickname,
        record.card,
        record.role,
        record.message_type,
        record.sub_type,
        record.raw_message,
        record.plain_text,
        record.human_text,
        record.images,
        record.files,
        record.message_json,
        record.original_json,
        record.self_id,
        record.font,
        record.msg_time,
    )
    .execute(&get_db_pool().await)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;

    /// 创建测试用的数据库连接池
    async fn create_test_pool() -> SqlitePool {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .expect("Failed to create test pool");

        // 创建表结构
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS fast_reply (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create fast_reply table");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS feedbacks (
                feedback_id INTEGER UNSIGNED,
                qqbot_msg_id INTEGER UNSIGNED
            );
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create feedbacks table");

        pool
    }

    /// 为测试注入临时的数据库池
    async fn with_test_pool<F, Fut>(test: F)
    where
        F: FnOnce(SqlitePool) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let pool = create_test_pool().await;
        test(pool).await;
    }

    #[tokio::test]
    async fn test_update_and_get_fast_reply() {
        with_test_pool(|pool| async move {
            // 测试插入新的 fast_reply
            update_fast_reply_with_pool(&pool, "test_id", "test_content")
                .await
                .expect("Failed to insert fast_reply");

            // 测试获取内容
            let content = get_fast_reply_content_with_pool(&pool, "test_id")
                .await
                .expect("Failed to get fast_reply");
            assert_eq!(content, Some("test_content".to_string()));

            // 测试更新已存在的 fast_reply
            update_fast_reply_with_pool(&pool, "test_id", "updated_content")
                .await
                .expect("Failed to update fast_reply");

            let content = get_fast_reply_content_with_pool(&pool, "test_id")
                .await
                .expect("Failed to get updated fast_reply");
            assert_eq!(content, Some("updated_content".to_string()));

            // 测试获取不存在的 id
            let content = get_fast_reply_content_with_pool(&pool, "non_existent")
                .await
                .expect("Failed to query");
            assert_eq!(content, None);
        })
        .await;
    }

    #[tokio::test]
    async fn test_get_fast_reply_list() {
        with_test_pool(|pool| async move {
            // 插入多条数据
            update_fast_reply_with_pool(&pool, "id1", "content1")
                .await
                .expect("Failed to insert");
            update_fast_reply_with_pool(&pool, "id2", "content2")
                .await
                .expect("Failed to insert");
            update_fast_reply_with_pool(&pool, "id3", "content3")
                .await
                .expect("Failed to insert");

            // 获取列表
            let list = get_fast_reply_list_with_pool(&pool)
                .await
                .expect("Failed to get list");

            assert_eq!(list.len(), 3);
            assert!(list.contains(&(String::from("id1"), String::from("content1"))));
            assert!(list.contains(&(String::from("id2"), String::from("content2"))));
            assert!(list.contains(&(String::from("id3"), String::from("content3"))));
        })
        .await;
    }

    #[tokio::test]
    async fn test_delete_fast_reply() {
        with_test_pool(|pool| async move {
            // 插入数据
            update_fast_reply_with_pool(&pool, "to_delete", "content")
                .await
                .expect("Failed to insert");

            // 确认数据存在
            let content = get_fast_reply_content_with_pool(&pool, "to_delete")
                .await
                .expect("Failed to get");
            assert_eq!(content, Some("content".to_string()));

            // 删除数据
            delete_fast_reply_with_pool(&pool, "to_delete")
                .await
                .expect("Failed to delete");

            // 确认数据已删除
            let content = get_fast_reply_content_with_pool(&pool, "to_delete")
                .await
                .expect("Failed to get");
            assert_eq!(content, None);

            // 删除不存在的数据不应该报错
            delete_fast_reply_with_pool(&pool, "non_existent")
                .await
                .expect("Deleting non-existent should not error");
        })
        .await;
    }

    #[tokio::test]
    async fn test_feedback_operations() {
        with_test_pool(|pool| async move {
            // 测试插入 feedback
            update_feedback_msg_id_with_pool(&pool, 1, 12345)
                .await
                .expect("Failed to insert feedback");

            // 测试获取 feedback_id
            let feedback_id = get_feedback_id_by_msg_with_pool(&pool, 12345)
                .await
                .expect("Failed to get feedback_id");
            assert_eq!(feedback_id, Some(1));

            // 测试获取不存在的 feedback
            let feedback_id = get_feedback_id_by_msg_with_pool(&pool, 99999)
                .await
                .expect("Failed to query");
            assert_eq!(feedback_id, None);

            // 测试插入多个 feedback
            update_feedback_msg_id_with_pool(&pool, 2, 12346)
                .await
                .expect("Failed to insert");
            update_feedback_msg_id_with_pool(&pool, 3, 12347)
                .await
                .expect("Failed to insert");

            let feedback_id = get_feedback_id_by_msg_with_pool(&pool, 12346)
                .await
                .expect("Failed to get");
            assert_eq!(feedback_id, Some(2));

            let feedback_id = get_feedback_id_by_msg_with_pool(&pool, 12347)
                .await
                .expect("Failed to get");
            assert_eq!(feedback_id, Some(3));
        })
        .await;
    }

    // 辅助函数：使用自定义池的版本
    async fn get_fast_reply_list_with_pool(pool: &SqlitePool) -> Result<Vec<(String, String)>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, content
            FROM fast_reply
            "#
        )
        .fetch_all(pool)
        .await?;

        let replies = rows
            .into_iter()
            .filter_map(|row| row.id.map(|id| (id, row.content)))
            .collect();
        Ok(replies)
    }

    async fn get_fast_reply_content_with_pool(
        pool: &SqlitePool,
        id: &str,
    ) -> Result<Option<String>> {
        let content = sqlx::query_scalar!(
            r#"
            SELECT content
            FROM fast_reply
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(pool)
        .await?;
        Ok(content)
    }

    async fn update_fast_reply_with_pool(pool: &SqlitePool, id: &str, content: &str) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO fast_reply
            (id, content)
            VALUES (?, ?)
            ON CONFLICT(id) DO UPDATE SET content = ?
            "#,
            id,
            content,
            content
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn delete_fast_reply_with_pool(pool: &SqlitePool, id: &str) -> Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM fast_reply
            WHERE id = ?
            "#,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn get_feedback_id_by_msg_with_pool(
        pool: &SqlitePool,
        msg_id: i64,
    ) -> Result<Option<u32>> {
        let feedback_id = sqlx::query!(
            r#"
            SELECT feedback_id
            FROM feedbacks
            WHERE qqbot_msg_id = ?
            "#,
            msg_id
        )
        .fetch_optional(pool)
        .await?;
        Ok(feedback_id.and_then(|row| row.feedback_id.map(|id| id as u32)))
    }

    async fn update_feedback_msg_id_with_pool(
        pool: &SqlitePool,
        feedback_id: u32,
        msg_id: i64,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO feedbacks (feedback_id, qqbot_msg_id)
            VALUES (?, ?)
            "#,
            feedback_id,
            msg_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
