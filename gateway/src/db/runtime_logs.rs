use regex::Regex;
use serde_json::{Map, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::observability::metrics::Metrics;

const ERROR_MESSAGE_MAX_CHARS: usize = 512;
const TRUNCATED_SUFFIX: &str = "...[truncated]";

pub async fn insert_runtime_log(
    pool: &PgPool,
    event_id: &str,
    bot_id: Uuid,
    runtime_type: &str,
    status: &str,
    error_code: Option<&str>,
    error_message: Option<&str>,
    latency_ms: i32,
    request_payload: Option<Value>,
    response_payload: Option<Value>,
    metrics: Option<&Metrics>,
) {
    if status != "error" {
        return;
    }

    let result = sqlx::query(
        "INSERT INTO runtime_logs \
            (id, event_id, bot_id, runtime_type, request_payload, response_payload, status, error_code, error_message, latency_ms, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, 'error', $7, $8, $9, NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(event_id)
    .bind(bot_id)
    .bind(runtime_type)
    .bind(request_payload)
    .bind(response_payload)
    .bind(error_code)
    .bind(error_message.map(sanitize_error_message))
    .bind(latency_ms.max(0))
    .execute(pool)
    .await;

    if let Err(err) = result {
        crate::db::increment_runtime_log_write_failures_total();
        if let Some(m) = metrics {
            m.runtime_log_write_failures_total.inc();
        }
        tracing::warn!(
            event_id,
            %bot_id,
            runtime_type,
            error = %err,
            "failed to insert runtime_logs row"
        );
    }
}

pub fn sanitize_request_payload(payload: Value) -> Value {
    sanitize_payload_by_whitelist(
        payload,
        &["session_id", "event_id", "runtime_type", "model"],
    )
}

pub fn sanitize_response_payload(payload: Value) -> Value {
    let mut sanitized =
        sanitize_payload_by_whitelist(payload, &["error_type", "error_message", "status_code"]);
    if let Some(obj) = sanitized.as_object_mut() {
        if let Some(message) = obj.get("error_message").and_then(Value::as_str) {
            obj.insert(
                "error_message".to_string(),
                Value::String(sanitize_error_message(message)),
            );
        }
    }
    sanitized
}

fn sanitize_payload_by_whitelist(payload: Value, allowed_keys: &[&str]) -> Value {
    let Some(obj) = payload.as_object() else {
        return Value::Object(Map::new());
    };

    let mut safe = Map::new();
    for key in allowed_keys {
        if let Some(value) = obj.get(*key) {
            safe.insert((*key).to_string(), value.clone());
        }
    }
    Value::Object(safe)
}

fn sanitize_error_message(raw: &str) -> String {
    let mut text = raw.to_string();
    text = redact_bearer_token(&text);
    text = redact_shopify_secret(&text);

    if text.chars().count() <= ERROR_MESSAGE_MAX_CHARS {
        return text;
    }

    let mut clipped: String = text.chars().take(ERROR_MESSAGE_MAX_CHARS).collect();
    clipped.push_str(TRUNCATED_SUFFIX);
    clipped
}

fn redact_bearer_token(input: &str) -> String {
    let re = Regex::new(r"Bearer\s+[A-Za-z0-9._\-]+").expect("valid regex");
    re.replace_all(input, "[REDACTED]").to_string()
}

fn redact_shopify_secret(input: &str) -> String {
    let re = Regex::new(r"shp[a-zA-Z]+_[0-9a-fA-F]{32,64}").expect("valid regex");
    re.replace_all(input, "[REDACTED]").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_request_removes_user_id_and_input_text() {
        let payload = serde_json::json!({
            "session_id": "telegram:private:123",
            "event_id": "evt-1",
            "runtime_type": "nanobot",
            "model": "deepseek-chat",
            "user_id": "u-1",
            "input_text": "hello",
            "extra": "drop-me"
        });

        let out = sanitize_request_payload(payload);
        assert_eq!(out["session_id"], "telegram:private:123");
        assert_eq!(out["event_id"], "evt-1");
        assert_eq!(out["runtime_type"], "nanobot");
        assert_eq!(out["model"], "deepseek-chat");
        assert!(out.get("user_id").is_none());
        assert!(out.get("input_text").is_none());
        assert!(out.get("extra").is_none());
    }

    #[test]
    fn sanitize_response_truncates_and_redacts_error_message() {
        let secret = "shpat_0123456789abcdef0123456789abcdef";
        let bearer = "Bearer token.with-dots_123";
        let long_text = format!("{} {} {}", "x".repeat(500), bearer, secret);
        let payload = serde_json::json!({
            "error_type": "runtime_error",
            "error_message": long_text,
            "status_code": 500,
            "input_text": "should-be-removed"
        });

        let out = sanitize_response_payload(payload);
        assert_eq!(out["error_type"], "runtime_error");
        assert_eq!(out["status_code"], 500);
        assert!(out.get("input_text").is_none());

        let msg = out["error_message"].as_str().unwrap();
        assert!(msg.ends_with(TRUNCATED_SUFFIX));
        assert!(!msg.contains("Bearer "));
        assert!(!msg.contains("shpat_"));
        assert!(msg.contains("[REDACTED]"));
    }

    #[test]
    fn sanitize_payload_handles_non_object_without_panic() {
        let req = sanitize_request_payload(Value::Null);
        let resp = sanitize_response_payload(Value::Bool(true));
        assert_eq!(req, Value::Object(Map::new()));
        assert_eq!(resp, Value::Object(Map::new()));
    }

    #[tokio::test]
    async fn insert_runtime_log_skips_write_when_status_success() {
        crate::db::reset_runtime_log_write_failures_total_for_test();

        let pool = PgPool::connect_lazy("postgres://postgres:postgres@127.0.0.1:1/im_agent_bridge")
            .expect("lazy pool should build");
        insert_runtime_log(
            &pool,
            "evt-no-write",
            Uuid::nil(),
            "nanobot",
            "success",
            None,
            None,
            1,
            Some(serde_json::json!({"session_id":"s"})),
            Some(serde_json::json!({"error_type":"e"})),
            None,
        )
        .await;

        assert_eq!(crate::db::runtime_log_write_failures_total(), 0);
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL and migrated schema with seed data"]
    async fn insert_runtime_log_writes_row_for_error() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let pool = PgPool::connect(&url).await.unwrap();

        let bot_id = Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap();
        let event_id = format!("evt-runtime-log-{}", Uuid::new_v4());

        sqlx::query(
            "INSERT INTO message_events \
                (id, event_id, bot_id, session_id, platform, bridge_gateway_name, bridge_message_id, reply_id, chat_id, chat_type, status, created_at) \
             VALUES ($1, $2, $3, 'telegram:private:test', 'telegram', 'default', $4, $5, 'chat-test', 'private', 'processing', NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(&event_id)
        .bind(bot_id)
        .bind(format!("msg-{}", Uuid::new_v4()))
        .bind(format!("reply-{}", Uuid::new_v4()))
        .execute(&pool)
        .await
        .unwrap();

        insert_runtime_log(
            &pool,
            &event_id,
            bot_id,
            "nanobot",
            "error",
            Some("RUNTIME_TIMEOUT"),
            Some("Bearer test-token-123"),
            42,
            Some(serde_json::json!({"session_id":"s-1","user_id":"drop-me"})),
            Some(serde_json::json!({"error_type":"timeout","error_message":"shpat_0123456789abcdef0123456789abcdef"})),
            None,
        )
        .await;

        let row: (String, Option<Value>, i32) = sqlx::query_as(
            "SELECT status, request_payload, latency_ms FROM runtime_logs WHERE event_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(&event_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "error");
        assert_eq!(row.2, 42);
        let req_payload = row.1.unwrap();
        assert!(req_payload.get("user_id").is_none());
        assert_eq!(req_payload["session_id"], "s-1");

        sqlx::query("DELETE FROM runtime_logs WHERE event_id = $1")
            .bind(&event_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM message_events WHERE event_id = $1")
            .bind(&event_id)
            .execute(&pool)
            .await
            .unwrap();
    }
}
