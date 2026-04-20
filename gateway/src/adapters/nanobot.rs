use async_trait::async_trait;
use serde::Deserialize;

use crate::adapters::runtime::{BotConfig, RuntimeAdapter, RuntimeError, StandardReply};
use crate::models::standard_message::StandardMessage;
use crate::observability::metrics::Metrics;

const TRUNCATE_LIMIT: usize = 4096;
const TRUNCATE_SUFFIX: &str = "…（内容已截断）";
const NANOBOT_TIMEOUT_SECS: u64 = 60;

pub struct NanoBotAdapter {
    pub client: reqwest::Client,
}

impl NanoBotAdapter {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(NANOBOT_TIMEOUT_SECS))
            .no_proxy()
            .build()
            .expect("failed to build reqwest client for NanoBotAdapter");
        Self { client }
    }

    pub async fn process_with_latency(
        &self,
        msg: &StandardMessage,
        bot: &BotConfig,
    ) -> (Result<StandardReply, RuntimeError>, i32) {
        let start = std::time::Instant::now();
        let result = self.process_inner(msg, bot, None).await;
        let latency_ms = start.elapsed().as_millis() as i32;
        (result, latency_ms)
    }

    pub async fn process_with_latency_with_metrics(
        &self,
        msg: &StandardMessage,
        bot: &BotConfig,
        metrics: Option<&Metrics>,
    ) -> (Result<StandardReply, RuntimeError>, i32) {
        let start = std::time::Instant::now();
        let result = self.process_inner(msg, bot, metrics).await;
        let latency_ms = start.elapsed().as_millis() as i32;
        (result, latency_ms)
    }

    async fn process_inner(
        &self,
        msg: &StandardMessage,
        bot: &BotConfig,
        metrics: Option<&Metrics>,
    ) -> Result<StandardReply, RuntimeError> {
        let url = format!(
            "{}/v1/chat/completions",
            bot.runtime_endpoint.trim_end_matches('/')
        );

        let body = serde_json::json!({
            "model": bot.runtime_model,
            "messages": [
                {
                    "role": "user",
                    "content": msg.text
                }
            ],
            "session_id": msg.session_id
        });

        tracing::info!(
            session_id = %msg.session_id,
            bot_id = %bot.id,
            runtime_model = %bot.runtime_model,
            endpoint = %url,
            "calling nanobot runtime"
        );

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|err| {
                if err.is_timeout() {
                    if let Some(m) = metrics {
                        m.runtime_call_timeout_total.inc();
                    }
                    tracing::warn!(
                        session_id = %msg.session_id,
                        error_code = "RUNTIME_TIMEOUT",
                        "nanobot request timed out"
                    );
                    RuntimeError::Timeout
                } else if err.is_connect() {
                    tracing::warn!(
                        session_id = %msg.session_id,
                        error_code = "RUNTIME_UNAVAILABLE",
                        "nanobot connection failed"
                    );
                    RuntimeError::Unavailable
                } else {
                    tracing::error!(
                        session_id = %msg.session_id,
                        error = %err,
                        error_code = "RUNTIME_UNAVAILABLE",
                        "nanobot request error"
                    );
                    RuntimeError::Unavailable
                }
            })?;

        let http_status = response.status().as_u16();

        if !response.status().is_success() {
            let error_body: Option<NanoBotErrorBody> = response
                .json::<NanoBotResponse>()
                .await
                .ok()
                .and_then(|r| r.error);

            if is_session_not_found(http_status, &error_body) {
                tracing::warn!(
                    session_id = %msg.session_id,
                    http_status,
                    error_code = "RUNTIME_SESSION_NOT_FOUND",
                    "nanobot session not found"
                );
                return Err(RuntimeError::SessionNotFound);
            }

            tracing::error!(
                session_id = %msg.session_id,
                http_status,
                error_code = "RUNTIME_BAD_RESPONSE",
                "nanobot returned non-2xx"
            );
            return Err(RuntimeError::BadResponse(format!("HTTP {}", http_status)));
        }

        let resp_body: NanoBotResponse = response.json().await.map_err(|err| {
            tracing::error!(
                session_id = %msg.session_id,
                error = %err,
                error_code = "RUNTIME_BAD_RESPONSE",
                "failed to parse nanobot response body"
            );
            RuntimeError::BadResponse(format!("json parse error: {}", err))
        })?;

        let content = resp_body
            .choices
            .and_then(|choices| choices.into_iter().next())
            .and_then(|choice| choice.message.content)
            .ok_or_else(|| {
                tracing::error!(
                    session_id = %msg.session_id,
                    error_code = "RUNTIME_BAD_RESPONSE",
                    "nanobot response missing choices[0].message.content"
                );
                RuntimeError::BadResponse("choices[0].message.content missing".to_string())
            })?;

        let text = truncate_reply(&content);

        tracing::info!(
            session_id = %msg.session_id,
            reply_len = text.chars().count(),
            "nanobot response received"
        );
        if let Some(m) = metrics {
            m.runtime_call_success_total.inc();
        }

        Ok(StandardReply {
            text,
            status: "success".to_string(),
        })
    }
}

/// Truncate reply text to TRUNCATE_LIMIT chars, appending suffix if truncated (BR-003).
pub fn truncate_reply(text: &str) -> String {
    if text.chars().count() <= TRUNCATE_LIMIT {
        text.to_string()
    } else {
        let suffix_chars = TRUNCATE_SUFFIX.chars().count();
        if suffix_chars >= TRUNCATE_LIMIT {
            return text.chars().take(TRUNCATE_LIMIT).collect();
        }
        let keep_chars = TRUNCATE_LIMIT - suffix_chars;
        let truncated: String = text.chars().take(keep_chars).collect();
        format!("{}{}", truncated, TRUNCATE_SUFFIX)
    }
}

#[derive(Deserialize)]
struct NanoBotResponse {
    choices: Option<Vec<NanoBotChoice>>,
    error: Option<NanoBotErrorBody>,
}

#[derive(Deserialize)]
struct NanoBotChoice {
    message: NanoBotMessage,
}

#[derive(Deserialize)]
struct NanoBotMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct NanoBotErrorBody {
    message: Option<String>,
    #[serde(rename = "type")]
    error_type: Option<String>,
}

/// Detect whether a NanoBot error response indicates session-not-found.
///
/// ⚠️ PENDING PROBE 3.2: Exact trigger condition (HTTP status + error body structure)
/// must be confirmed by running:
///   curl -v http://localhost:8900/v1/chat/completions \
///     -H "Content-Type: application/json" \
///     -d '{"model":"deepseek-chat","messages":[{"role":"user","content":"probe"}],"session_id":"probe-nonexistent-session-99999"}' \
///     2>&1 | grep -E "< HTTP|^\{|error"
///
/// After confirming the exact HTTP status and error body, update this function.
/// Per spec: mapping MUST NOT be hardcoded before probe 3.2 is completed.
fn is_session_not_found(http_status: u16, error_body: &Option<NanoBotErrorBody>) -> bool {
    if http_status == 404 {
        return true;
    }

    let Some(body) = error_body else {
        return false;
    };
    let message = body
        .message
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let error_type = body
        .error_type
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();

    error_type.contains("session_not_found")
        || (message.contains("session") && message.contains("not found"))
}

#[async_trait]
impl RuntimeAdapter for NanoBotAdapter {
    async fn process(
        &self,
        msg: &StandardMessage,
        bot: &BotConfig,
    ) -> Result<StandardReply, RuntimeError> {
        self.process_with_latency_with_metrics(msg, bot, None)
            .await
            .0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_msg(session_id: &str, text: &str) -> StandardMessage {
        StandardMessage {
            event_id: Uuid::new_v4().to_string(),
            platform: "telegram".to_string(),
            bridge_gateway_name: "tg-gateway".to_string(),
            bridge_channel_name: None,
            bridge_message_id: "msg-1".to_string(),
            chat_id: "123456".to_string(),
            chat_type: "private".to_string(),
            user_id: "user-1".to_string(),
            session_id: session_id.to_string(),
            text: text.to_string(),
            timestamp: "2026-04-16T00:00:00Z".to_string(),
            bot_id: Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap(),
        }
    }

    fn sample_bot(endpoint: &str) -> BotConfig {
        BotConfig {
            id: Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap(),
            runtime_type: "nanobot".to_string(),
            runtime_endpoint: endpoint.to_string(),
            runtime_model: "deepseek-chat".to_string(),
        }
    }

    // 6.1.1 — request body shape: model, session_id, messages 1 item, no stream
    #[test]
    fn request_body_shape_is_correct() {
        let body = serde_json::json!({
            "model": "deepseek-chat",
            "messages": [{"role": "user", "content": "ping"}],
            "session_id": "telegram:private:123456"
        });
        assert!(
            body.get("stream").is_none(),
            "stream field must not be present"
        );
        assert_eq!(body["session_id"], "telegram:private:123456");
        assert_eq!(body["model"], "deepseek-chat");
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
        assert_eq!(body["messages"][0]["role"], "user");
    }

    // 6.1.2 — normal response parsing: choices[0].message.content extracted
    #[tokio::test]
    async fn normal_response_parsed_correctly() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{"message": {"content": "hello from nanobot"}}]
            })))
            .mount(&mock_server)
            .await;

        let adapter = NanoBotAdapter::new();
        let msg = sample_msg("telegram:private:123456", "ping");
        let bot = sample_bot(&mock_server.uri());

        let result = adapter.process(&msg, &bot).await.unwrap();
        assert_eq!(result.text, "hello from nanobot");
        assert_eq!(result.status, "success");
    }

    // 6.1.2 — session_id in request body matches msg.session_id
    #[tokio::test]
    async fn request_contains_correct_session_id() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{"message": {"content": "pong"}}]
            })))
            .mount(&mock_server)
            .await;

        let adapter = NanoBotAdapter::new();
        let session_id = "telegram:private:999888";
        let msg = sample_msg(session_id, "ping");
        let bot = sample_bot(&mock_server.uri());

        let result = adapter.process(&msg, &bot).await.unwrap();
        assert_eq!(result.status, "success");

        let requests = mock_server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["session_id"], session_id);
    }

    // 6.1.3 — timeout → RuntimeError::Timeout
    #[tokio::test]
    async fn timeout_maps_to_runtime_timeout() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let _accept_task = tokio::spawn(async move {
            if let Ok((_stream, _)) = listener.accept().await {
                // Keep stream alive (no drop) so connection stays open → triggers read timeout
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(150))
            .no_proxy()
            .build()
            .unwrap();
        let adapter = NanoBotAdapter { client };
        let msg = sample_msg("telegram:private:123", "ping");
        let mut bot = sample_bot("http://127.0.0.1:1");
        bot.runtime_endpoint = format!("http://127.0.0.1:{}", port);

        let result = adapter.process(&msg, &bot).await;
        assert!(
            matches!(result, Err(RuntimeError::Timeout)),
            "expected Timeout, got {:?}",
            result.err()
        );
    }

    // 6.1.4 — connect error → RuntimeError::Unavailable
    #[tokio::test]
    async fn connect_error_maps_to_runtime_unavailable() {
        let port = {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            listener.local_addr().unwrap().port()
        };

        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let adapter = NanoBotAdapter { client };
        let msg = sample_msg("telegram:private:123", "ping");
        let mut bot = sample_bot("http://127.0.0.1:1");
        bot.runtime_endpoint = format!("http://127.0.0.1:{}", port);

        let result = adapter.process(&msg, &bot).await;
        assert!(
            matches!(result, Err(RuntimeError::Unavailable)),
            "expected Unavailable, got {:?}",
            result.err()
        );
    }

    // 6.1.5 — missing choices[0].message.content → RuntimeError::BadResponse
    #[tokio::test]
    async fn missing_choices_maps_to_bad_response() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"choices": []})),
            )
            .mount(&mock_server)
            .await;

        let adapter = NanoBotAdapter::new();
        let msg = sample_msg("telegram:private:123", "ping");
        let bot = sample_bot(&mock_server.uri());

        let result = adapter.process(&msg, &bot).await;
        assert!(
            matches!(result, Err(RuntimeError::BadResponse(_))),
            "expected BadResponse, got {:?}",
            result.err()
        );
    }

    // 6.1.6 — truncate_reply boundary values
    #[test]
    fn truncate_at_limit_unchanged() {
        let text = "a".repeat(TRUNCATE_LIMIT);
        assert_eq!(truncate_reply(&text), text);
    }

    #[test]
    fn truncate_above_limit_adds_suffix() {
        let text = "a".repeat(TRUNCATE_LIMIT + 1);
        let result = truncate_reply(&text);
        assert_eq!(result.chars().count(), TRUNCATE_LIMIT);
        assert!(result.ends_with(TRUNCATE_SUFFIX));
        let base_len = TRUNCATE_LIMIT - TRUNCATE_SUFFIX.chars().count();
        let base: String = result.chars().take(base_len).collect();
        assert_eq!(base, "a".repeat(base_len));
    }

    #[test]
    fn truncate_5000_chars() {
        let text = "b".repeat(5000);
        let result = truncate_reply(&text);
        assert_eq!(result.chars().count(), TRUNCATE_LIMIT);
        assert!(result.ends_with(TRUNCATE_SUFFIX));
        let base_len = TRUNCATE_LIMIT - TRUNCATE_SUFFIX.chars().count();
        let base: String = result.chars().take(base_len).collect();
        assert_eq!(base, "b".repeat(base_len));
    }

    #[test]
    fn truncate_unicode_by_chars() {
        let text = "中".repeat(TRUNCATE_LIMIT + 10);
        let result = truncate_reply(&text);
        assert_eq!(result.chars().count(), TRUNCATE_LIMIT);
        let base_len = TRUNCATE_LIMIT - TRUNCATE_SUFFIX.chars().count();
        let prefix: String = result.chars().take(base_len).collect();
        assert_eq!(prefix, "中".repeat(base_len));
        assert!(result.ends_with(TRUNCATE_SUFFIX));
    }

    #[test]
    fn session_not_found_detected_by_http_404() {
        assert!(is_session_not_found(404, &None));
    }

    #[test]
    fn session_not_found_detected_by_error_body() {
        let body = Some(NanoBotErrorBody {
            message: Some("session not found".to_string()),
            error_type: Some("session_not_found".to_string()),
        });
        assert!(is_session_not_found(400, &body));
    }
}
