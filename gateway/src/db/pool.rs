use crate::{config::AppConfig, errors::AppError};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

const REQUIRED_TABLES: &[&str] = &[
    "bots",
    "channel_bindings",
    "sessions",
    "message_events",
    "runtime_logs",
];

const REQUIRED_INDEXES: &[&str] = &[
    "idx_sessions_bot_platform_chat",
    "uq_message_events_inbound_dedup",
    "uq_message_events_reply_id",
    "idx_message_events_session_created",
    "idx_message_events_bot",
    "idx_channel_bindings_bot_platform",
    "idx_channel_bindings_lookup",
    "idx_runtime_logs_event",
    "idx_runtime_logs_bot_created",
    "uq_channel_bindings_source",
];

pub async fn init_pool(config: &AppConfig) -> PgPool {
    let pool = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await;

    match pool {
        Ok(pool) => {
            tracing::info!(
                max_connections = config.db_max_connections,
                "db pool initialized"
            );
            pool
        }
        Err(err) => {
            tracing::error!(
                error = %err,
                missing_or_invalid_field = "DATABASE_URL",
                "failed to initialize db pool"
            );
            std::process::exit(1);
        }
    }
}

pub async fn health_check(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
        .map(|_| ())
        .map_err(Into::into)
}

pub async fn validate_required_schema(pool: &PgPool) -> Result<(), AppError> {
    let mut missing_tables = Vec::new();
    for table in REQUIRED_TABLES {
        let exists = sqlx::query_scalar::<_, i32>(
            "SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = $1 LIMIT 1",
        )
        .bind(*table)
        .fetch_optional(pool)
        .await?;

        if exists.is_none() {
            missing_tables.push(*table);
        }
    }

    let mut missing_indexes = Vec::new();
    for index in REQUIRED_INDEXES {
        let exists = sqlx::query_scalar::<_, i32>(
            "SELECT 1 FROM pg_indexes WHERE schemaname = 'public' AND indexname = $1 LIMIT 1",
        )
        .bind(*index)
        .fetch_optional(pool)
        .await?;

        if exists.is_none() {
            missing_indexes.push(*index);
        }
    }

    if !missing_tables.is_empty() || !missing_indexes.is_empty() {
        return Err(anyhow::anyhow!(
            "database migration verification failed; missing tables: [{}], missing indexes: [{}]",
            missing_tables.join(", "),
            missing_indexes.join(", ")
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_check_returns_err_when_pg_unreachable() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@127.0.0.1:1/im_agent_bridge")
            .expect("lazy pool should build");

        let result = health_check(&pool).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL_TEST to point to a reachable postgres"]
    async fn health_check_returns_ok_when_pg_reachable() {
        let database_url =
            std::env::var("DATABASE_URL_TEST").expect("missing env: DATABASE_URL_TEST");

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("DATABASE_URL_TEST should point to a reachable postgres");

        let result = health_check(&pool).await;
        assert!(result.is_ok());
    }
}
