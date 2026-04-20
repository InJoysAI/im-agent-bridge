use sqlx::PgPool;
use uuid::Uuid;

use crate::models::standard_message::StandardMessage;

pub async fn insert_pending(
    pool: &PgPool,
    msg: &StandardMessage,
    input_text: &str,
    reply_id: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        "INSERT INTO message_events \
            (id, event_id, bot_id, session_id, platform, bridge_gateway_name, bridge_channel_name, bridge_message_id, reply_id, chat_id, chat_type, user_id, input_text, status, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 'pending', NOW()) \
         ON CONFLICT (platform, bridge_gateway_name, COALESCE(bridge_channel_name, ''), bridge_message_id) \
         DO NOTHING \
         RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(&msg.event_id)
    .bind(msg.bot_id)
    .bind(&msg.session_id)
    .bind(&msg.platform)
    .bind(&msg.bridge_gateway_name)
    .bind(msg.bridge_channel_name.as_deref())
    .bind(&msg.bridge_message_id)
    .bind(reply_id)
    .bind(&msg.chat_id)
    .bind(&msg.chat_type)
    .bind(&msg.user_id)
    .bind(input_text)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id,)| id))
}

pub async fn mark_processing(
    pool: &PgPool,
    event_id: &str,
    bot_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE message_events \
         SET status = 'processing' \
         WHERE event_id = $1 AND bot_id = $2",
    )
    .bind(event_id)
    .bind(bot_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn mark_done(
    pool: &PgPool,
    event_id: &str,
    bot_id: Uuid,
    output_text: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE message_events \
         SET status = 'done', output_text = $3 \
         WHERE event_id = $1 AND bot_id = $2",
    )
    .bind(event_id)
    .bind(bot_id)
    .bind(output_text)
    .execute(pool)
    .await?;

    Ok(())
}

/// 更新 `message_events.reply_status` 字段（`"success"` 或 `"reply_failed"`）。
///
/// 对应 feat-runtime-reply-bridge Bridge 回写链路：
/// - HTTP 200 / 409 → `"success"`
/// - 不可重试错误 (400/401) 立即失败 或 4 次尝试耗尽 → `"reply_failed"`
pub async fn mark_reply_status(
    pool: &PgPool,
    event_id: &str,
    bot_id: Uuid,
    reply_status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE message_events \
         SET reply_status = $3 \
         WHERE event_id = $1 AND bot_id = $2",
    )
    .bind(event_id)
    .bind(bot_id)
    .bind(reply_status)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn mark_error(
    pool: &PgPool,
    event_id: &str,
    bot_id: Uuid,
    error_code: &str,
    error_message: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE message_events \
         SET status = 'error', error_code = $3, error_message = $4 \
         WHERE event_id = $1 AND bot_id = $2",
    )
    .bind(event_id)
    .bind(bot_id)
    .bind(error_code)
    .bind(error_message)
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_message(bridge_message_id: &str, text: &str) -> StandardMessage {
        StandardMessage {
            event_id: Uuid::new_v4().to_string(),
            platform: "telegram".to_string(),
            bridge_gateway_name: "default".to_string(),
            bridge_channel_name: Some("general".to_string()),
            bridge_message_id: bridge_message_id.to_string(),
            chat_id: "chat-test".to_string(),
            chat_type: "private".to_string(),
            user_id: "user-test".to_string(),
            session_id: "telegram:private:chat-test".to_string(),
            text: text.to_string(),
            timestamp: "2026-04-16T00:00:00Z".to_string(),
            bot_id: Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap(),
        }
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL and migrated schema with seed data"]
    async fn insert_pending_first_time_returns_some_and_pending() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let pool = PgPool::connect(&url).await.unwrap();
        let msg = sample_message("msg-insert-first", "hello");
        let reply_id = Uuid::new_v4().to_string();

        sqlx::query("DELETE FROM message_events WHERE event_id = $1")
            .bind(&msg.event_id)
            .execute(&pool)
            .await
            .unwrap();

        let inserted = insert_pending(&pool, &msg, "hello", &reply_id)
            .await
            .unwrap();
        assert!(inserted.is_some());

        let row: (String,) =
            sqlx::query_as("SELECT status FROM message_events WHERE event_id = $1 AND bot_id = $2")
                .bind(&msg.event_id)
                .bind(msg.bot_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, "pending");

        sqlx::query("DELETE FROM message_events WHERE event_id = $1")
            .bind(&msg.event_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL and migrated schema with seed data"]
    async fn insert_pending_duplicate_returns_none_and_only_one_row() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let pool = PgPool::connect(&url).await.unwrap();

        let dedup_key = "msg-dedup-1";
        let msg1 = sample_message(dedup_key, "first");
        let msg2 = sample_message(dedup_key, "second");

        sqlx::query(
            "DELETE FROM message_events \
             WHERE platform = 'telegram' AND bridge_gateway_name = 'default' \
               AND COALESCE(bridge_channel_name, '') = 'general' \
               AND bridge_message_id = $1",
        )
        .bind(dedup_key)
        .execute(&pool)
        .await
        .unwrap();

        let first = insert_pending(&pool, &msg1, "first", &Uuid::new_v4().to_string())
            .await
            .unwrap();
        assert!(first.is_some());

        let second = insert_pending(&pool, &msg2, "second", &Uuid::new_v4().to_string())
            .await
            .unwrap();
        assert!(second.is_none());

        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM message_events \
             WHERE platform = 'telegram' AND bridge_gateway_name = 'default' \
               AND COALESCE(bridge_channel_name, '') = 'general' \
               AND bridge_message_id = $1",
        )
        .bind(dedup_key)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, 1);

        sqlx::query(
            "DELETE FROM message_events \
             WHERE platform = 'telegram' AND bridge_gateway_name = 'default' \
               AND COALESCE(bridge_channel_name, '') = 'general' \
               AND bridge_message_id = $1",
        )
        .bind(dedup_key)
        .execute(&pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL and migrated schema with seed data"]
    async fn mark_processing_updates_status() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let pool = PgPool::connect(&url).await.unwrap();
        let msg = sample_message("msg-mark-processing", "hello");

        let _ = insert_pending(&pool, &msg, "hello", &Uuid::new_v4().to_string())
            .await
            .unwrap();

        mark_processing(&pool, &msg.event_id, msg.bot_id)
            .await
            .unwrap();

        let row: (String,) =
            sqlx::query_as("SELECT status FROM message_events WHERE event_id = $1 AND bot_id = $2")
                .bind(&msg.event_id)
                .bind(msg.bot_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, "processing");

        sqlx::query("DELETE FROM message_events WHERE event_id = $1")
            .bind(&msg.event_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL and migrated schema with seed data"]
    async fn insert_pending_persists_input_text() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let pool = PgPool::connect(&url).await.unwrap();
        let msg = sample_message("msg-input-text", "ignored");
        let truncated = "中".repeat(512);

        let _ = insert_pending(&pool, &msg, &truncated, &Uuid::new_v4().to_string())
            .await
            .unwrap();

        let row: (String,) = sqlx::query_as(
            "SELECT input_text FROM message_events WHERE event_id = $1 AND bot_id = $2",
        )
        .bind(&msg.event_id)
        .bind(msg.bot_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, truncated);

        sqlx::query("DELETE FROM message_events WHERE event_id = $1")
            .bind(&msg.event_id)
            .execute(&pool)
            .await
            .unwrap();
    }
}
