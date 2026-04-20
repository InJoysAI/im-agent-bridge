use sqlx::PgPool;
use uuid::Uuid;

/// Upsert a session record (BR-032, criterion.md §3.7).
///
/// Conflict key: `(bot_id, session_id)` — established by migration `00003`.
/// On conflict: updates `updated_at` and `last_user_id` (RISK-B006 mitigation).
///
/// `id` is application-generated (`Uuid::new_v4()`) per project convention.
/// `bot_id` is mandatory for multi-Bot isolation (BR-032).
pub async fn upsert_session(
    pool: &PgPool,
    session_id: &str,
    bot_id: Uuid,
    platform: &str,
    chat_id: &str,
    chat_type: &str,
    last_user_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO sessions \
             (id, session_id, bot_id, platform, chat_id, chat_type, last_user_id, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW()) \
         ON CONFLICT (bot_id, session_id) \
         DO UPDATE SET updated_at = NOW(), last_user_id = EXCLUDED.last_user_id",
    )
    .bind(Uuid::new_v4())
    .bind(session_id)
    .bind(bot_id)
    .bind(platform)
    .bind(chat_id)
    .bind(chat_type)
    .bind(last_user_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Clear `runtime_session_key` for a session, triggering session rebuild on next Runtime call.
///
/// Called when NanoBotAdapter returns `RuntimeError::SessionNotFound` (criterion.md §3.5).
pub async fn clear_runtime_session_key(
    pool: &PgPool,
    session_id: &str,
    bot_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE sessions \
         SET runtime_session_key = NULL, updated_at = NOW() \
         WHERE session_id = $1 AND bot_id = $2",
    )
    .bind(session_id)
    .bind(bot_id)
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // 6.3.1 — first insert creates the record
    // Uses the seeded bot_id (11111111-...) to satisfy sessions_bot_id_fkey FK.
    #[tokio::test]
    #[ignore = "requires DATABASE_URL and migrated schema (goose up 00003) with seed_db.sh"]
    async fn first_insert_creates_record() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let pool = sqlx::PgPool::connect(&url).await.unwrap();
        // Use seeded bot_id so FK constraint sessions_bot_id_fkey is satisfied
        let bot_id = Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap();

        // Clean up any prior run before inserting
        sqlx::query("DELETE FROM sessions WHERE session_id = $1 AND bot_id = $2")
            .bind("telegram:private:test001")
            .bind(bot_id)
            .execute(&pool)
            .await
            .unwrap();

        upsert_session(
            &pool,
            "telegram:private:test001",
            bot_id,
            "telegram",
            "test001",
            "private",
            "user-test",
        )
        .await
        .expect("first insert should succeed");

        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM sessions WHERE session_id = $1 AND bot_id = $2")
                .bind("telegram:private:test001")
                .bind(bot_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, 1, "record should exist after first insert");

        sqlx::query("DELETE FROM sessions WHERE session_id = $1 AND bot_id = $2")
            .bind("telegram:private:test001")
            .bind(bot_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    // 6.3.2 — duplicate upsert is idempotent; updated_at refreshed; last_user_id updated
    // Uses the seeded bot_id (11111111-...) to satisfy sessions_bot_id_fkey FK.
    #[tokio::test]
    #[ignore = "requires DATABASE_URL and migrated schema (goose up 00003) with seed_db.sh"]
    async fn duplicate_upsert_is_idempotent() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let pool = sqlx::PgPool::connect(&url).await.unwrap();
        // Use seeded bot_id so FK constraint sessions_bot_id_fkey is satisfied
        let bot_id = Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap();

        // Clean up any prior run
        sqlx::query("DELETE FROM sessions WHERE session_id = $1 AND bot_id = $2")
            .bind("telegram:private:test002")
            .bind(bot_id)
            .execute(&pool)
            .await
            .unwrap();

        upsert_session(
            &pool,
            "telegram:private:test002",
            bot_id,
            "telegram",
            "test002",
            "private",
            "user-a",
        )
        .await
        .expect("first insert should succeed");

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        upsert_session(
            &pool,
            "telegram:private:test002",
            bot_id,
            "telegram",
            "test002",
            "private",
            "user-b",
        )
        .await
        .expect("second upsert should not error");

        let row: (i64, String) = sqlx::query_as(
            "SELECT COUNT(*), last_user_id FROM sessions \
             WHERE session_id = $1 AND bot_id = $2 \
             GROUP BY last_user_id",
        )
        .bind("telegram:private:test002")
        .bind(bot_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, 1, "should still be 1 row after upsert");
        assert_eq!(row.1, "user-b", "last_user_id should be updated to latest");

        sqlx::query("DELETE FROM sessions WHERE session_id = $1 AND bot_id = $2")
            .bind("telegram:private:test002")
            .bind(bot_id)
            .execute(&pool)
            .await
            .unwrap();
    }
}
