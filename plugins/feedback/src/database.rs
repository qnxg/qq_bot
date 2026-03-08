use crate::config::CFG;
use anyhow::Result;
use kovi::tokio::sync::OnceCell;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::time::Duration;

static DB_POOL: OnceCell<SqlitePool> = OnceCell::const_new();

/// # Performance
/// 参见 [`sqlx::pool::Pool`] 文档：
/// > Cloning `Pool` is cheap as it is simply a
/// > reference-counted handle to the inner pool state.
///
/// 因此实际上没有必要将[`SqlitePool`]用[`std::sync::Arc`]等包裹。
/// 可以直接调用此函数获得全局数据库池。
///
/// # Side Effects
/// 数据库连接异常时，这个函数可能会结束进程。
pub async fn get_db_pool() -> SqlitePool {
    DB_POOL
        .get_or_init(|| async {
            match SqlitePoolOptions::new()
                .max_connections(CFG.database.max_connections)
                .acquire_timeout(Duration::from_secs(3))
                .connect(&CFG.database.database_url)
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
            }
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
        SELECT id
        FROM feedbacks
        WHERE qqbot_msg_id = ?
        "#,
        msg_id
    )
    .fetch_optional(&get_db_pool().await)
    .await?;
    Ok(feedback_id.and_then(|row| row.id.map(|id| id as u32)))
}

pub async fn update_feedback_msg_id(msg_id: i64) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO feedbacks (qqbot_msg_id)
        VALUES (?)
        "#,
        msg_id
    )
    .execute(&get_db_pool().await)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kovi::tokio;
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
            "#
        )
        .execute(&pool)
        .await
        .expect("Failed to create fast_reply table");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS feedbacks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                qqbot_msg_id INTEGER
            );
            "#
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
        }).await;
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
        }).await;
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
        }).await;
    }

    #[tokio::test]
    async fn test_feedback_operations() {
        with_test_pool(|pool| async move {
            // 测试插入 feedback
            update_feedback_msg_id_with_pool(&pool, 12345)
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
            update_feedback_msg_id_with_pool(&pool, 12346)
                .await
                .expect("Failed to insert");
            update_feedback_msg_id_with_pool(&pool, 12347)
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
        }).await;
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

    async fn get_fast_reply_content_with_pool(pool: &SqlitePool, id: &str) -> Result<Option<String>> {
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

    async fn get_feedback_id_by_msg_with_pool(pool: &SqlitePool, msg_id: i64) -> Result<Option<u32>> {
        let feedback_id = sqlx::query!(
            r#"
            SELECT id
            FROM feedbacks
            WHERE qqbot_msg_id = ?
            "#,
            msg_id
        )
        .fetch_optional(pool)
        .await?;
        Ok(feedback_id.and_then(|row| row.id.map(|id| id as u32)))
    }

    async fn update_feedback_msg_id_with_pool(pool: &SqlitePool, msg_id: i64) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO feedbacks (qqbot_msg_id)
            VALUES (?)
            "#,
            msg_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
