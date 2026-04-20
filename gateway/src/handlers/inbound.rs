use std::sync::Arc;

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::adapters::nanobot::NanoBotAdapter;
use crate::adapters::runtime::RuntimeError;
use crate::bridge_client::{self, BridgeReplyPayload};
use crate::db;
use crate::middleware::auth::BearerAuth;
use crate::middleware::rate_limit::RateLimiter;
use crate::models::inbound::{
    ChatType, ErrorResponse, InboundResponse, InboundStatus, MessageType, ValidatedJson,
};
use crate::models::session::generate_session_id;
use crate::models::standard_message::StandardMessage;
use crate::observability::metrics::Metrics;
use prometheus_client::registry::Registry;

pub use crate::models::inbound::InboundRequest;

fn truncate_input_text(text: &str) -> String {
    text.chars().take(512).collect::<String>()
}

fn missing_or_unmatched_mention(
    platform: &str,
    chat_type: ChatType,
    require_mention: bool,
    telegram_username: Option<&str>,
    text: &str,
) -> bool {
    if !platform.eq_ignore_ascii_case("telegram")
        || chat_type != ChatType::Group
        || !require_mention
    {
        return false;
    }

    let Some(username) = telegram_username else {
        return true;
    };
    let username = username.trim();
    if username.is_empty() {
        return true;
    }

    let mention = format!("@{}", username.to_ascii_lowercase());
    !text.to_ascii_lowercase().contains(&mention)
}

fn strip_bot_mentions_for_runtime(text: &str, telegram_username: &str) -> String {
    let username = telegram_username.trim();
    if username.is_empty() {
        return text.to_string();
    }

    let mention = format!("@{}", username.to_ascii_lowercase());
    let lowered = text.to_ascii_lowercase();

    if !lowered.contains(&mention) {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;

    while let Some(rel_pos) = lowered[cursor..].find(&mention) {
        let pos = cursor + rel_pos;
        out.push_str(&text[cursor..pos]);
        cursor = pos + mention.len();
    }
    out.push_str(&text[cursor..]);

    out.trim().to_string()
}

#[derive(Clone)]
pub struct InboundHandlerState {
    pub bridge_url: String,
    pub bridge_bearer_token: String,
    pub rate_limiter: Arc<RateLimiter>,
    pub nanobot_adapter: Arc<NanoBotAdapter>,
    pub bridge_http_client: Arc<reqwest::Client>,
    pub metrics_registry: Arc<Registry>,
    pub metrics: Arc<Metrics>,
}

/// POST /gateway/inbound
/// Auth is validated by the `BearerAuth` extractor before the handler body runs.
pub async fn inbound_handler(
    auth: BearerAuth,
    State(state): State<InboundHandlerState>,
    Extension(pool): Extension<PgPool>,
    ValidatedJson(req): ValidatedJson<InboundRequest>,
) -> Result<Json<InboundResponse>, (StatusCode, Json<ErrorResponse>)> {
    let event_id = auth.event_id;
    tracing::info!(
        event_id = %event_id,
        platform = %req.platform,
        chat_id = %req.raw_message.chat_id,
        chat_type = req.raw_message.chat_type.as_str(),
        "inbound message arrived"
    );

    // 6.1.4 — Non-text message intercept (BR-001)
    if req.raw_message.message_type != MessageType::Text {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "非文本消息类型，已忽略".to_string(),
            }),
        ));
    }

    // 6.1.5 — text field must be present when message_type=text
    let text = req.raw_message.text.as_deref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "text 字段为必填项".to_string(),
            }),
        )
    })?;

    // 6.1.6 — Empty / whitespace-only text (BR-001, edge_cases.md:17-18)
    if text.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "消息内容不能为空".to_string(),
            }),
        ));
    }

    // 6.1.7 — Inbound text > 4096 chars (BR-002, cross_cutting_concepts.md:106-109)
    if text.chars().count() > 4096 {
        tracing::info!(
            event_id = %event_id,
            chat_id = %req.raw_message.chat_id,
            text_len = text.chars().count(),
            "inbound text exceeds 4096 chars, rejected"
        );
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "消息过长，请缩短后重试".to_string(),
            }),
        ));
    }

    // 6.1.3 — DB health guard (503 if DB unavailable)
    if db::health_guard_with_auth(
        &pool,
        &event_id,
        &req.raw_message.chat_id,
        &req.platform,
        &req.bridge_gateway_name,
        req.bridge_channel_name.as_deref(),
        &state.bridge_url,
        Some(&state.bridge_bearer_token),
    )
    .await
    .is_err()
    {
        state.metrics.db_unavailable_total.inc();
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "service unavailable".to_string(),
            }),
        ));
    }

    // 5.1.1 — channel_bindings lookup: source triple → bot_id (BR-004)
    let bot_id = match db::channel_bindings::find_bot_id_by_channel(
        &pool,
        &req.platform,
        &req.bridge_gateway_name,
        req.bridge_channel_name.as_deref(),
    )
    .await
    {
        Ok(Some(id)) => id,
        Ok(None) => {
            tracing::warn!(
                event_id = %event_id,
                platform = %req.platform,
                gateway = %req.bridge_gateway_name,
                channel = ?req.bridge_channel_name,
                "channel binding not found"
            );
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "channel binding not found".to_string(),
                }),
            ));
        }
        Err(err) => {
            db::increment_db_unavailable_total();
            state.metrics.db_unavailable_total.inc();
            tracing::error!(
                event_id = %event_id,
                platform = %req.platform,
                gateway = %req.bridge_gateway_name,
                error = %err,
                "db error during channel_bindings lookup"
            );
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: "service unavailable".to_string(),
                }),
            ));
        }
    };
    tracing::info!(
        event_id = %event_id,
        bot_id = %bot_id,
        "channel resolved to bot_id"
    );

    // 5.1.2 — generate session_id (BR-010, BR-011)
    let session_id = generate_session_id(
        &req.platform,
        req.raw_message.chat_type.as_str(),
        &req.raw_message.chat_id,
    );
    tracing::info!(
        event_id = %event_id,
        session_id = %session_id,
        "session_id generated_or_hit"
    );

    // 5.1.x — Bot config lookup by bot_id (BR-032).
    // Must remain bot_id-based lookup; no username reverse-lookup is allowed.
    let bot = match db::bots::get_by_id(&pool, bot_id).await {
        Ok(Some(bot)) => bot,
        Ok(None) => {
            tracing::error!(
                event_id = %event_id,
                bot_id = %bot_id,
                "bot config not found"
            );
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: "service unavailable".to_string(),
                }),
            ));
        }
        Err(err) => {
            db::increment_db_unavailable_total();
            state.metrics.db_unavailable_total.inc();
            tracing::error!(
                event_id = %event_id,
                bot_id = %bot_id,
                error = %err,
                "db error during bot config lookup"
            );
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: "service unavailable".to_string(),
                }),
            ));
        }
    };

    if missing_or_unmatched_mention(
        &req.platform,
        req.raw_message.chat_type,
        bot.require_mention,
        bot.telegram_username.as_deref(),
        text,
    ) {
        tracing::info!(
            event_id = %event_id,
            bot_id = %bot_id,
            chat_id = %req.raw_message.chat_id,
            "inbound skipped: group_no_mention"
        );
        return Ok(Json(InboundResponse {
            status: InboundStatus::IgnoredNoMention,
        }));
    }

    // BR-055 ordering: mention filter precedes rate limit; see design.md Decision 6
    if !state.rate_limiter.allow(&req.raw_message.chat_id) {
        state.metrics.rate_limited_total.inc();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        tracing::warn!(
            event_id = %event_id,
            chat_id = %req.raw_message.chat_id,
            timestamp_unix = ts,
            "request rate limited"
        );
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse {
                error: "Too many requests".to_string(),
            }),
        ));
    }

    let runtime_input_text = if req.platform.eq_ignore_ascii_case("telegram")
        && req.raw_message.chat_type == ChatType::Group
        && bot.require_mention
    {
        bot.telegram_username
            .as_deref()
            .map(|username| strip_bot_mentions_for_runtime(text, username))
            .unwrap_or_else(|| text.to_string())
    } else {
        text.to_string()
    };

    // 5.1.3 — sessions upsert (BR-032; 503 熔断 on DB error)
    if let Err(err) = db::sessions::upsert_session(
        &pool,
        &session_id,
        bot_id,
        &req.platform,
        &req.raw_message.chat_id,
        req.raw_message.chat_type.as_str(),
        &req.raw_message.user_id,
    )
    .await
    {
        db::increment_db_unavailable_total();
        state.metrics.db_unavailable_total.inc();
        tracing::error!(
            event_id = %event_id,
            chat_id = %req.raw_message.chat_id,
            session_id = %session_id,
            error = %err,
            "db error during session upsert"
        );
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "service unavailable".to_string(),
            }),
        ));
    }

    let input_text_truncated = truncate_input_text(&runtime_input_text);
    let reply_id = Uuid::new_v4().to_string();
    let mut std_msg = StandardMessage::build(&req, bot_id, &session_id, event_id.clone());
    std_msg.text = runtime_input_text;
    tracing::debug!(
        event_id = %std_msg.event_id,
        session_id = %session_id,
        "message normalization completed"
    );

    match db::message_events::insert_pending(&pool, &std_msg, &input_text_truncated, &reply_id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            tracing::info!(
                event_id = %std_msg.event_id,
                platform = %req.platform,
                gateway = %req.bridge_gateway_name,
                chat_id = %req.raw_message.chat_id,
                bridge_message_id = %req.raw_message.message_id,
                "duplicate inbound ignored"
            );
            return Ok(Json(InboundResponse {
                status: InboundStatus::IgnoredDuplicate,
            }));
        }
        Err(err) => {
            db::increment_db_unavailable_total();
            state.metrics.db_unavailable_total.inc();
            tracing::error!(
                event_id = %std_msg.event_id,
                chat_id = %req.raw_message.chat_id,
                session_id = %session_id,
                error = %err,
                "db error during message_events insert_pending"
            );
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: "service unavailable".to_string(),
                }),
            ));
        }
    }
    state.metrics.messages_received_total.inc();

    if let Err(err) = db::message_events::mark_processing(&pool, &std_msg.event_id, bot_id).await {
        db::increment_db_unavailable_total();
        state.metrics.db_unavailable_total.inc();
        tracing::error!(
            chat_id = %req.raw_message.chat_id,
            session_id = %session_id,
            event_id = %std_msg.event_id,
            error = %err,
            "db error during message_events mark_processing"
        );
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "service unavailable".to_string(),
            }),
        ));
    }

    tracing::info!(
        event_id = %std_msg.event_id,
        platform = %req.platform,
        gateway = %req.bridge_gateway_name,
        chat_id = %req.raw_message.chat_id,
        user_id = %req.raw_message.user_id,
        bot_id = %bot_id,
        session_id = %session_id,
        "inbound accepted"
    );

    let bot_config = bot.runtime_config();
    tracing::info!(
        event_id = %std_msg.event_id,
        session_id = %session_id,
        runtime_type = %bot_config.runtime_type,
        "runtime call started"
    );
    let runtime_request_payload = serde_json::json!({
        "session_id": std_msg.session_id,
        "event_id": std_msg.event_id,
        "runtime_type": bot_config.runtime_type,
        "model": bot_config.runtime_model,
        "user_id": std_msg.user_id,
        "input_text": std_msg.text,
        "platform": std_msg.platform,
        "chat_id": std_msg.chat_id,
    });

    // 5.1 — Dispatch to NanoBotAdapter based on runtime_type
    let (adapter_result, latency_ms) = match bot_config.runtime_type.as_str() {
        "nanobot" => {
            let (result, first_latency_ms) = state
                .nanobot_adapter
                .process_with_latency_with_metrics(&std_msg, &bot_config, Some(&state.metrics))
                .await;
            // 5.2 — SessionNotFound: clear runtime_session_key and retry once
            match result {
                Err(RuntimeError::SessionNotFound) => {
                    tracing::warn!(
                        event_id = %std_msg.event_id,
                        session_id = %session_id,
                        bot_id = %bot_id,
                        "session not found, clearing runtime_session_key and retrying"
                    );
                    if let Err(err) =
                        db::sessions::clear_runtime_session_key(&pool, &session_id, bot_id).await
                    {
                        tracing::error!(
                            event_id = %std_msg.event_id,
                            session_id = %session_id,
                            error = %err,
                            "failed to clear runtime_session_key"
                        );
                    }
                    let (retry_result, retry_latency_ms) = state
                        .nanobot_adapter
                        .process_with_latency_with_metrics(
                            &std_msg,
                            &bot_config,
                            Some(&state.metrics),
                        )
                        .await;
                    (
                        retry_result,
                        first_latency_ms.saturating_add(retry_latency_ms),
                    )
                }
                other => (other, first_latency_ms),
            }
        }
        unknown => {
            tracing::error!(
                event_id = %std_msg.event_id,
                bot_id = %bot_id,
                runtime_type = %unknown,
                "unknown runtime_type, no adapter available"
            );
            (Err(RuntimeError::Unavailable), 0)
        }
    };

    // Handle adapter result
    match adapter_result {
        Ok(reply) => {
            // BR-070: truncate at 512 chars before persistence (落库最小化)
            let output_for_db = bridge_client::truncate_to_512(&reply.text);
            if let Err(err) = db::message_events::mark_done(
                &pool,
                &std_msg.event_id,
                bot_id,
                Some(&output_for_db),
            )
            .await
            {
                tracing::error!(
                    event_id = %std_msg.event_id,
                    error = %err,
                    "db error during message_events mark_done"
                );
            }
            tracing::info!(
                session_id = %session_id,
                event_id = %std_msg.event_id,
                latency_ms,
                reply_len = reply.text.chars().count(),
                "runtime adapter call succeeded"
            );

            // Bridge 回写链路 (feat-runtime-reply-bridge)
            // BR-003: payload text ≤ 4096 字符（bridge_client 入口兜底截断）
            let reply_text = bridge_client::enforce_bridge_text_limit(&reply.text);
            let payload = BridgeReplyPayload {
                reply_id: reply_id.clone(),
                chat_id: std_msg.chat_id.clone(),
                platform: std_msg.platform.clone(),
                text: reply_text,
                bridge_gateway_name: std_msg.bridge_gateway_name.clone(),
                bridge_channel_name: std_msg.bridge_channel_name.clone(),
            };

            let reply_outcome = bridge_client::post_reply(
                state.bridge_http_client.as_ref(),
                &state.bridge_url,
                &state.bridge_bearer_token,
                &payload,
                &state.metrics,
            )
            .await;

            let reply_status = match &reply_outcome {
                Ok(()) => "success",
                Err(err) => {
                    tracing::error!(
                        reply_id = %reply_id,
                        event_id = %std_msg.event_id,
                        error_kind = err.kind(),
                        error = %err,
                        "bridge reply failed; marking reply_failed"
                    );
                    "reply_failed"
                }
            };
            if reply_status == "success" {
                tracing::info!(
                    event_id = %std_msg.event_id,
                    reply_id = %reply_id,
                    "bridge reply succeeded"
                );
            } else {
                tracing::warn!(
                    event_id = %std_msg.event_id,
                    reply_id = %reply_id,
                    "bridge reply failed"
                );
            }
            if let Err(db_err) = db::message_events::mark_reply_status(
                &pool,
                &std_msg.event_id,
                bot_id,
                reply_status,
            )
            .await
            {
                tracing::error!(
                    event_id = %std_msg.event_id,
                    error = %db_err,
                    "db error during message_events mark_reply_status"
                );
            }
        }
        Err(err) => {
            let runtime_response_payload = serde_json::json!({
                "error_type": err.error_code(),
                "error_message": err.to_string(),
                "status_code": match err {
                    RuntimeError::Timeout => 504,
                    RuntimeError::Unavailable => 503,
                    RuntimeError::BadResponse(_) => 502,
                    RuntimeError::SessionNotFound => 404,
                },
                "reply_text": err.user_message(),
            });
            db::runtime_logs::insert_runtime_log(
                &pool,
                &std_msg.event_id,
                bot_id,
                &bot_config.runtime_type,
                "error",
                Some(err.error_code()),
                Some(&err.to_string()),
                latency_ms,
                Some(db::runtime_logs::sanitize_request_payload(
                    runtime_request_payload.clone(),
                )),
                Some(db::runtime_logs::sanitize_response_payload(
                    runtime_response_payload,
                )),
                Some(&state.metrics),
            )
            .await;

            tracing::error!(
                session_id = %session_id,
                event_id = %std_msg.event_id,
                error_code = err.error_code(),
                error = %err,
                latency_ms,
                "runtime adapter call failed"
            );
            if let Err(db_err) = db::message_events::mark_error(
                &pool,
                &std_msg.event_id,
                bot_id,
                err.error_code(),
                &err.to_string(),
            )
            .await
            {
                tracing::error!(
                    event_id = %std_msg.event_id,
                    error = %db_err,
                    "db error during message_events mark_error"
                );
            }

            // Runtime 异常时也必须尝试回写用户可理解提示，避免请求静默失败。
            let error_reply_payload = BridgeReplyPayload {
                reply_id: reply_id.clone(),
                chat_id: std_msg.chat_id.clone(),
                platform: std_msg.platform.clone(),
                text: bridge_client::enforce_bridge_text_limit(err.user_message()),
                bridge_gateway_name: std_msg.bridge_gateway_name.clone(),
                bridge_channel_name: std_msg.bridge_channel_name.clone(),
            };

            let reply_outcome = bridge_client::post_reply(
                state.bridge_http_client.as_ref(),
                &state.bridge_url,
                &state.bridge_bearer_token,
                &error_reply_payload,
                &state.metrics,
            )
            .await;

            let reply_status = match &reply_outcome {
                Ok(()) => "success",
                Err(reply_err) => {
                    tracing::error!(
                        reply_id = %reply_id,
                        event_id = %std_msg.event_id,
                        error_kind = reply_err.kind(),
                        error = %reply_err,
                        "bridge reply failed after runtime error"
                    );
                    "reply_failed"
                }
            };

            if let Err(db_err) = db::message_events::mark_reply_status(
                &pool,
                &std_msg.event_id,
                bot_id,
                reply_status,
            )
            .await
            {
                tracing::error!(
                    event_id = %std_msg.event_id,
                    error = %db_err,
                    "db error during message_events mark_reply_status after runtime error"
                );
            }
        }
    }

    Ok(Json(InboundResponse {
        status: InboundStatus::Accepted,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::auth::BearerTokenConfig;
    use crate::observability::metrics::Metrics;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::post,
        Router,
    };
    use prometheus_client::registry::Registry;
    use serde_json::Value;
    use sqlx::{postgres::PgPoolOptions, PgPool};
    use std::time::Duration;
    use tower::ServiceExt;
    use uuid::Uuid;

    fn make_app(token: &str) -> Router {
        let dead_pool = PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(500))
            .connect_lazy("postgres://postgres:postgres@127.0.0.1:1/im_agent_bridge")
            .unwrap();
        make_app_with_pool(token, dead_pool)
    }

    fn make_app_with_pool(token: &str, pool: PgPool) -> Router {
        let mut registry = Registry::default();
        let metrics = Arc::new(Metrics::new(&mut registry));
        Router::new()
            .route("/gateway/inbound", post(inbound_handler))
            .with_state(InboundHandlerState {
                bridge_url: "http://127.0.0.1:1".to_string(),
                bridge_bearer_token: "bridge-token".to_string(),
                rate_limiter: Arc::new(RateLimiter::new()),
                nanobot_adapter: Arc::new(NanoBotAdapter::new()),
                bridge_http_client: Arc::new(reqwest::Client::new()),
                metrics_registry: Arc::new(registry),
                metrics,
            })
            .layer(Extension(pool))
            .layer(Extension(BearerTokenConfig(token.to_string())))
    }

    fn valid_body(chat_id: &str) -> String {
        valid_body_with_message(chat_id, "msg-1", "hello")
    }

    fn valid_body_with_message(chat_id: &str, message_id: &str, text: &str) -> String {
        valid_body_with_source(chat_id, message_id, text, "default", None)
    }

    fn valid_body_with_source(
        chat_id: &str,
        message_id: &str,
        text: &str,
        bridge_gateway_name: &str,
        bridge_channel_name: Option<&str>,
    ) -> String {
        valid_body_with_source_and_chat_type(
            chat_id,
            message_id,
            text,
            bridge_gateway_name,
            bridge_channel_name,
            "private",
        )
    }

    fn valid_body_with_source_and_chat_type(
        chat_id: &str,
        message_id: &str,
        text: &str,
        bridge_gateway_name: &str,
        bridge_channel_name: Option<&str>,
        chat_type: &str,
    ) -> String {
        serde_json::json!({
            "platform": "telegram",
            "bridge_gateway_name": bridge_gateway_name,
            "bridge_channel_name": bridge_channel_name,
            "raw_message": {
                "chat_id": chat_id,
                "chat_type": chat_type,
                "user_id": "user-1",
                "message_type": "text",
                "text": text,
                "timestamp": "2026-04-15T00:00:00Z",
                "message_id": message_id
            }
        })
        .to_string()
    }

    async fn send(app: Router, token: Option<&str>, body: String) -> axum::response::Response {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/gateway/inbound")
            .header("content-type", "application/json");
        if let Some(t) = token {
            builder = builder.header("authorization", format!("Bearer {t}"));
        }
        app.oneshot(builder.body(Body::from(body)).unwrap())
            .await
            .unwrap()
    }

    // 8.4.2 — No token → 401
    #[tokio::test]
    async fn no_token_returns_401() {
        let resp = send(make_app("tok"), None, valid_body("chat-1")).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    // 8.1.2 — Invalid token → 401
    #[tokio::test]
    async fn invalid_token_returns_401() {
        let app = make_app("correct-token");
        let resp = send(app, Some("wrong-token"), valid_body("chat-1")).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    // 8.4.4 — Missing required field (platform) → 400
    #[tokio::test]
    async fn missing_platform_returns_400() {
        let body = serde_json::json!({
            "bridge_gateway_name": "default",
            "raw_message": {
                "chat_id": "chat-1",
                "chat_type": "private",
                "user_id": "user-1",
                "message_type": "text",
                "text": "hello",
                "timestamp": "2026-04-15T00:00:00Z",
                "message_id": "msg-1"
            }
        })
        .to_string();
        let resp = send(make_app("tok"), Some("tok"), body).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // 8.4.5 — message_type=image → 400 + ignore tip
    #[tokio::test]
    async fn image_message_returns_400_with_ignore_tip() {
        let body = serde_json::json!({
            "platform": "telegram",
            "bridge_gateway_name": "default",
            "raw_message": {
                "chat_id": "chat-1",
                "chat_type": "private",
                "user_id": "user-1",
                "message_type": "image",
                "timestamp": "2026-04-15T00:00:00Z",
                "message_id": "msg-1"
            }
        })
        .to_string();
        let resp = send(make_app("tok"), Some("tok"), body).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json["error"].as_str().unwrap().contains("已忽略"));
    }

    // Mention filtering now requires DB-loaded bot config first.
    // With DB unavailable, request short-circuits as 503 before rate limit.
    #[tokio::test]
    async fn db_unavailable_is_returned_before_rate_limit() {
        let rl = Arc::new(RateLimiter::new());
        for _ in 0..5 {
            rl.allow("chat-rate-429");
        }
        let mut registry = Registry::default();
        let metrics = Arc::new(Metrics::new(&mut registry));
        let dead_pool = PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(500))
            .connect_lazy("postgres://postgres:postgres@127.0.0.1:1/im_agent_bridge")
            .unwrap();
        let app = axum::Router::new()
            .route("/gateway/inbound", post(inbound_handler))
            .with_state(InboundHandlerState {
                bridge_url: "http://127.0.0.1:1".to_string(),
                bridge_bearer_token: "bridge-token".to_string(),
                rate_limiter: rl,
                nanobot_adapter: Arc::new(NanoBotAdapter::new()),
                bridge_http_client: Arc::new(reqwest::Client::new()),
                metrics_registry: Arc::new(registry),
                metrics,
            })
            .layer(Extension(dead_pool))
            .layer(Extension(BearerTokenConfig("tok".to_string())));
        let resp = send(app, Some("tok"), valid_body("chat-rate-429")).await;
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    // 8.3.3 — Invalid JSON body → 400
    #[tokio::test]
    async fn invalid_json_returns_400() {
        let resp = send(make_app("tok"), Some("tok"), "not-json".to_string()).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // existing: DB unavailable → 503 + counter increment
    #[tokio::test]
    async fn inbound_returns_503_and_increments_counter_when_db_unavailable() {
        let before = crate::db::db_unavailable_total();
        let resp = send(make_app("token-1"), Some("token-1"), valid_body("chat-db")).await;
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["error"], "service unavailable");
        let after = crate::db::db_unavailable_total();
        assert!(
            after > before,
            "db_unavailable_total should have incremented"
        );
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL and migrated schema"]
    async fn duplicate_message_returns_ignored_duplicate() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let pool = PgPool::connect(&url).await.unwrap();

        let bot_id = Uuid::new_v4();
        let binding_id = Uuid::new_v4();
        let bot_name = format!("dup-bot-{}", Uuid::new_v4());
        let gateway_name = format!("dup-gateway-{}", Uuid::new_v4());

        sqlx::query(
            "INSERT INTO bots (id, bot_name, name, runtime_type, runtime_endpoint, is_enabled, created_at, updated_at) \
             VALUES ($1, $2, 'dup test bot', 'nanobot', 'http://127.0.0.1:9999/runtime/process', true, NOW(), NOW())",
        )
        .bind(bot_id)
        .bind(&bot_name)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO channel_bindings (id, bot_id, platform, bridge_gateway_name, bridge_channel_name, is_enabled, created_at, updated_at) \
             VALUES ($1, $2, 'telegram', $3, NULL, true, NOW(), NOW())",
        )
        .bind(binding_id)
        .bind(bot_id)
        .bind(&gateway_name)
        .execute(&pool)
        .await
        .unwrap();

        let app = make_app_with_pool("tok", pool.clone());
        let payload = valid_body_with_source("chat-dup", "dup-msg-1", "hello", &gateway_name, None);

        let first = send(app.clone(), Some("tok"), payload.clone()).await;
        assert_eq!(first.status(), StatusCode::OK);

        let second = send(app, Some("tok"), payload).await;
        assert_eq!(second.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(second.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["status"], "ignored_duplicate");

        sqlx::query("DELETE FROM message_events WHERE bot_id = $1")
            .bind(bot_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM sessions WHERE bot_id = $1")
            .bind(bot_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM channel_bindings WHERE id = $1")
            .bind(binding_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM bots WHERE id = $1")
            .bind(bot_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL and migrated schema"]
    async fn group_mention_accepted_with_real_db() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let pool = PgPool::connect(&url).await.unwrap();

        let bot_id = Uuid::new_v4();
        let binding_id = Uuid::new_v4();
        let bot_name = format!("mention-bot-{}", Uuid::new_v4());
        let gateway_name = format!("mention-gateway-{}", Uuid::new_v4());

        sqlx::query(
            "INSERT INTO bots (id, bot_name, name, runtime_type, runtime_endpoint, runtime_model, telegram_username, require_mention, is_enabled, created_at, updated_at) \
             VALUES ($1, $2, 'mention test bot', 'nanobot', 'http://127.0.0.1:9999/runtime/process', 'nanobot', 'CBECOpsBot', true, true, NOW(), NOW())",
        )
        .bind(bot_id)
        .bind(&bot_name)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO channel_bindings (id, bot_id, platform, bridge_gateway_name, bridge_channel_name, is_enabled, created_at, updated_at) \
             VALUES ($1, $2, 'telegram', $3, NULL, true, NOW(), NOW())",
        )
        .bind(binding_id)
        .bind(bot_id)
        .bind(&gateway_name)
        .execute(&pool)
        .await
        .unwrap();

        let app = make_app_with_pool("tok", pool.clone());
        let payload = valid_body_with_source_and_chat_type(
            "chat-mention-accepted",
            "mention-msg-1",
            "@CBECOpsBot hi",
            &gateway_name,
            None,
            "group",
        );
        let resp = send(app, Some("tok"), payload).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["status"], "accepted");

        sqlx::query("DELETE FROM message_events WHERE bot_id = $1")
            .bind(bot_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM sessions WHERE bot_id = $1")
            .bind(bot_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM channel_bindings WHERE id = $1")
            .bind(binding_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM bots WHERE id = $1")
            .bind(bot_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL and migrated schema"]
    async fn group_without_mention_ignored_with_real_db() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let pool = PgPool::connect(&url).await.unwrap();

        let bot_id = Uuid::new_v4();
        let binding_id = Uuid::new_v4();
        let bot_name = format!("mention-ignore-bot-{}", Uuid::new_v4());
        let gateway_name = format!("mention-ignore-gateway-{}", Uuid::new_v4());

        sqlx::query(
            "INSERT INTO bots (id, bot_name, name, runtime_type, runtime_endpoint, runtime_model, telegram_username, require_mention, is_enabled, created_at, updated_at) \
             VALUES ($1, $2, 'mention ignore bot', 'nanobot', 'http://127.0.0.1:9999/runtime/process', 'nanobot', 'CBECOpsBot', true, true, NOW(), NOW())",
        )
        .bind(bot_id)
        .bind(&bot_name)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO channel_bindings (id, bot_id, platform, bridge_gateway_name, bridge_channel_name, is_enabled, created_at, updated_at) \
             VALUES ($1, $2, 'telegram', $3, NULL, true, NOW(), NOW())",
        )
        .bind(binding_id)
        .bind(bot_id)
        .bind(&gateway_name)
        .execute(&pool)
        .await
        .unwrap();

        let app = make_app_with_pool("tok", pool.clone());
        let payload = valid_body_with_source_and_chat_type(
            "chat-mention-ignored",
            "mention-msg-2",
            "今天天气不错",
            &gateway_name,
            None,
            "group",
        );
        let resp = send(app, Some("tok"), payload).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["status"], "ignored_no_mention");

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM message_events WHERE bot_id = $1")
                .bind(bot_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 0);

        sqlx::query("DELETE FROM sessions WHERE bot_id = $1")
            .bind(bot_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM channel_bindings WHERE id = $1")
            .bind(binding_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM bots WHERE id = $1")
            .bind(bot_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[test]
    fn truncate_input_text_keeps_full_when_at_or_below_limit() {
        let text = "中".repeat(512);
        let truncated = truncate_input_text(&text);
        assert_eq!(truncated.chars().count(), 512);
        assert_eq!(truncated, text);
    }

    #[test]
    fn truncate_input_text_truncates_unicode_by_chars() {
        let text = "中".repeat(600);
        let truncated = truncate_input_text(&text);
        assert_eq!(truncated.chars().count(), 512);
    }

    #[test]
    fn mention_filter_hits_group_require_mention_without_match() {
        let ignored = missing_or_unmatched_mention(
            "telegram",
            ChatType::Group,
            true,
            Some("CBECOpsBot"),
            "今天天气不错",
        );
        assert!(ignored);
    }

    #[test]
    fn mention_filter_matches_case_insensitive() {
        let ignored = missing_or_unmatched_mention(
            "telegram",
            ChatType::Group,
            true,
            Some("CBECOpsBot"),
            "@cbecopsbot hi",
        );
        assert!(!ignored);
    }

    #[test]
    fn mention_filter_bypasses_private_chat() {
        let ignored = missing_or_unmatched_mention(
            "telegram",
            ChatType::Private,
            true,
            Some("CBECOpsBot"),
            "hello",
        );
        assert!(!ignored);
    }

    #[test]
    fn mention_filter_bypasses_when_disabled() {
        let ignored = missing_or_unmatched_mention(
            "telegram",
            ChatType::Group,
            false,
            Some("CBECOpsBot"),
            "hello",
        );
        assert!(!ignored);
    }

    #[test]
    fn mention_filter_ignores_missing_username_when_required() {
        let ignored = missing_or_unmatched_mention(
            "telegram",
            ChatType::Group,
            true,
            None,
            "@cbecopsbot hello",
        );
        assert!(ignored);
    }

    #[test]
    fn mention_filter_requires_at_prefix() {
        let ignored = missing_or_unmatched_mention(
            "telegram",
            ChatType::Group,
            true,
            Some("Ops"),
            "Operations update",
        );
        assert!(ignored);
    }

    #[test]
    fn mention_filter_not_called_for_empty_text_in_flow() {
        let text = "";
        assert!(text.trim().is_empty());
        let ignored = missing_or_unmatched_mention(
            "telegram",
            ChatType::Group,
            true,
            Some("CBECOpsBot"),
            text,
        );
        assert!(ignored);
    }

    #[test]
    fn strip_mention_removes_at_username_prefix() {
        let out =
            strip_bot_mentions_for_runtime("@CBECOpsBot 查看最高价格商品的信息", "CBECOpsBot");
        assert_eq!(out, "查看最高价格商品的信息");
    }

    #[test]
    fn strip_mention_removes_case_insensitive_everywhere() {
        let out = strip_bot_mentions_for_runtime("请帮我 @cbecopsbot 查询订单", "CBECOpsBot");
        assert_eq!(out, "请帮我  查询订单");
    }

    #[test]
    fn strip_mention_keeps_original_when_not_found() {
        let out = strip_bot_mentions_for_runtime("查看最高价格商品的信息", "CBECOpsBot");
        assert_eq!(out, "查看最高价格商品的信息");
    }

    #[test]
    fn strip_mention_removes_multiple_mentions() {
        let out = strip_bot_mentions_for_runtime(
            "@CBECOpsBot 帮我看下 @cbecopsbot 这个订单",
            "CBECOpsBot",
        );
        assert_eq!(out, "帮我看下  这个订单");
    }
}
