use sqlx::PgPool;
use uuid::Uuid;

use crate::adapters::runtime::BotConfig;

#[derive(Debug, Clone)]
pub struct Bot {
    pub id: Uuid,
    pub runtime_type: String,
    pub runtime_endpoint: String,
    pub runtime_model: String,
    pub telegram_username: Option<String>,
    pub require_mention: bool,
}

impl Bot {
    pub fn runtime_config(&self) -> BotConfig {
        BotConfig {
            id: self.id,
            runtime_type: self.runtime_type.clone(),
            runtime_endpoint: self.runtime_endpoint.clone(),
            runtime_model: self.runtime_model.clone(),
        }
    }
}

/// Fetch bot configuration by id.
///
/// Returns runtime_type, runtime_endpoint, and runtime_model for use
/// by RuntimeAdapter dispatch logic (criterion.md §3.5).
///
/// Returns `Ok(None)` when no enabled bot with that id exists.
pub async fn get_by_id(pool: &PgPool, bot_id: Uuid) -> Result<Option<Bot>, sqlx::Error> {
    let row: Option<(Uuid, String, String, String, Option<String>, bool)> = sqlx::query_as(
        "SELECT id, runtime_type, runtime_endpoint, runtime_model, telegram_username, require_mention \
         FROM bots \
         WHERE id = $1 AND is_enabled = true",
    )
    .bind(bot_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(
        |(
            id,
            runtime_type,
            runtime_endpoint,
            runtime_model,
            telegram_username,
            require_mention,
        )| {
            Bot {
                id,
                runtime_type,
                runtime_endpoint,
                runtime_model,
                telegram_username,
                require_mention,
            }
        },
    ))
}

pub async fn find_bot_config(
    pool: &PgPool,
    bot_id: Uuid,
) -> Result<Option<BotConfig>, sqlx::Error> {
    let bot = get_by_id(pool, bot_id).await?;
    Ok(bot.map(|b| b.runtime_config()))
}
