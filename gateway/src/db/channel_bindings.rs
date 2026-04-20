use sqlx::PgPool;
use uuid::Uuid;

/// Resolve a `bot_id` from the channel_bindings table using the inbound source triple.
///
/// Lookup strategy — two-step, reuses `idx_channel_bindings_lookup`:
///
/// 1. If `bridge_channel_name = Some(name)`:
///    exact-match on `COALESCE(bridge_channel_name, '') = name`.
///    If no row found, fall through to step 2.
///
/// 2. Fallback: `bridge_channel_name IS NULL` (the gateway-level default binding).
///    If `bridge_channel_name = None`, skip step 1 and run step 2 directly.
///
/// Returns `Ok(None)` when no matching enabled binding exists → caller should 404.
///
/// Note: channel_bindings is the source of truth for bot_id resolution; the query
/// predicate is the source triple — no bot_id filter is needed or possible here.
pub async fn find_bot_id_by_channel(
    pool: &PgPool,
    platform: &str,
    bridge_gateway_name: &str,
    bridge_channel_name: Option<&str>,
) -> Result<Option<Uuid>, sqlx::Error> {
    if let Some(channel_name) = bridge_channel_name {
        // Step 1: exact match — index: idx_channel_bindings_lookup
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT bot_id FROM channel_bindings \
             WHERE platform = $1 \
               AND bridge_gateway_name = $2 \
               AND COALESCE(bridge_channel_name, '') = $3 \
               AND is_enabled = true \
             LIMIT 1",
        )
        .bind(platform)
        .bind(bridge_gateway_name)
        .bind(channel_name)
        .fetch_optional(pool)
        .await?;

        if let Some((bot_id,)) = row {
            return Ok(Some(bot_id));
        }
    }

    // Step 2: fallback — NULL / wildcard binding
    let row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT bot_id FROM channel_bindings \
         WHERE platform = $1 \
           AND bridge_gateway_name = $2 \
           AND bridge_channel_name IS NULL \
           AND is_enabled = true \
         LIMIT 1",
    )
    .bind(platform)
    .bind(bridge_gateway_name)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id,)| id))
}

#[cfg(test)]
mod tests {
    use super::*;

    // 6.1.1 — exact match returns bot_id
    #[tokio::test]
    #[ignore = "requires DATABASE_URL and seeded channel_bindings (tasks.md §8.0)"]
    async fn exact_match_returns_bot_id() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let pool = sqlx::PgPool::connect(&url).await.unwrap();
        // MT-1 seed: (telegram, default, general) → 11111111-...
        let result = find_bot_id_by_channel(&pool, "telegram", "default", Some("general"))
            .await
            .unwrap();
        assert!(result.is_some(), "exact match should return a bot_id");
    }

    // 6.1.2 — no exact match, fallback to NULL binding
    #[tokio::test]
    #[ignore = "requires DATABASE_URL and seeded channel_bindings (tasks.md §8.0)"]
    async fn fallback_match_returns_bot_id() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let pool = sqlx::PgPool::connect(&url).await.unwrap();
        // MT-2 scenario: unknown channel falls back to NULL binding
        let result =
            find_bot_id_by_channel(&pool, "telegram", "default", Some("some-unknown-channel"))
                .await
                .unwrap();
        assert!(result.is_some(), "fallback match should return a bot_id");
    }

    // 6.1.3 — no exact, no fallback → None
    #[tokio::test]
    #[ignore = "requires DATABASE_URL (no slack bindings seeded)"]
    async fn no_match_returns_none() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let pool = sqlx::PgPool::connect(&url).await.unwrap();
        // MT-3 scenario: platform=slack has no bindings
        let result = find_bot_id_by_channel(&pool, "slack", "default", None)
            .await
            .unwrap();
        assert!(result.is_none(), "missing binding should return None");
    }
}
