//! BR-032 约定：所有后续 db 查询/写入函数签名必须携带 `bot_id: Uuid` 参数，
//! 以保证多 Bot 数据隔离，禁止无 bot_id 过滤的全表 SELECT/UPDATE/DELETE。

pub mod bots;
pub mod channel_bindings;
pub mod message_events;
pub mod pool;
pub mod runtime_logs;
pub mod sessions;

use crate::errors::AppError;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};

pub use pool::{health_check, init_pool, validate_required_schema};

static DB_UNAVAILABLE_TOTAL: AtomicU64 = AtomicU64::new(0);
static RUNTIME_LOG_WRITE_FAILURES_TOTAL: AtomicU64 = AtomicU64::new(0);

pub fn db_unavailable_total() -> u64 {
    DB_UNAVAILABLE_TOTAL.load(Ordering::Relaxed)
}

pub fn increment_db_unavailable_total() {
    DB_UNAVAILABLE_TOTAL.fetch_add(1, Ordering::Relaxed);
}

pub fn runtime_log_write_failures_total() -> u64 {
    RUNTIME_LOG_WRITE_FAILURES_TOTAL.load(Ordering::Relaxed)
}

pub fn increment_runtime_log_write_failures_total() {
    RUNTIME_LOG_WRITE_FAILURES_TOTAL.fetch_add(1, Ordering::Relaxed);
}

pub async fn health_guard(
    pool: &PgPool,
    event_id: &str,
    chat_id: &str,
    platform: &str,
    bridge_gateway_name: &str,
    bridge_channel_name: Option<&str>,
    bridge_url: &str,
) -> Result<(), AppError> {
    health_guard_with_auth(
        pool,
        event_id,
        chat_id,
        platform,
        bridge_gateway_name,
        bridge_channel_name,
        bridge_url,
        None,
    )
    .await
}

pub async fn health_guard_with_auth(
    pool: &PgPool,
    event_id: &str,
    chat_id: &str,
    platform: &str,
    bridge_gateway_name: &str,
    bridge_channel_name: Option<&str>,
    bridge_url: &str,
    bridge_bearer_token: Option<&str>,
) -> Result<(), AppError> {
    if let Err(err) = health_check(pool).await {
        increment_db_unavailable_total();

        tracing::error!(
            event_id = event_id,
            db_unavailable = true,
            chat_id,
            db_unavailable_total = db_unavailable_total(),
            error = %err,
            "database unavailable"
        );

        if let Err(reply_err) = send_db_unavailable_reply(
            chat_id,
            platform,
            bridge_gateway_name,
            bridge_channel_name,
            bridge_url,
            bridge_bearer_token,
        )
        .await
        {
            tracing::error!(
                event_id = event_id,
                db_unavailable = true,
                chat_id,
                error = %reply_err,
                "failed to send bridge reply for db unavailable"
            );
        }

        return Err(anyhow::anyhow!("db unavailable"));
    }

    Ok(())
}

async fn send_db_unavailable_reply(
    chat_id: &str,
    platform: &str,
    bridge_gateway_name: &str,
    bridge_channel_name: Option<&str>,
    bridge_url: &str,
    bridge_bearer_token: Option<&str>,
) -> Result<(), AppError> {
    let _ = (chat_id, platform, bridge_channel_name); // 保留参数便于未来 Bridge 代理层恢复细粒度路由
    let client = reqwest::Client::new();
    // 直连 Matterbridge 1.26 原生消息端点 `/api/message`（同 bridge_client）。
    // 此路径只需 `gateway` + `text`：Matterbridge 根据 api 账号的 gateway 归属分发。
    let mut request = client
        .post(format!("{}/api/message", bridge_url.trim_end_matches('/')))
        .json(&serde_json::json!({
            "gateway": bridge_gateway_name,
            "text": "系统暂时不可用，请稍后重试",
        }));

    if let Some(token) = bridge_bearer_token {
        request = request.bearer_auth(token);
    }

    let response = request.send().await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "bridge reply returned {}",
            response.status()
        ));
    }

    Ok(())
}

#[cfg(test)]
pub fn reset_db_unavailable_total_for_test() {
    DB_UNAVAILABLE_TOTAL.store(0, Ordering::Relaxed);
}

#[cfg(test)]
pub fn reset_runtime_log_write_failures_total_for_test() {
    RUNTIME_LOG_WRITE_FAILURES_TOTAL.store(0, Ordering::Relaxed);
}
