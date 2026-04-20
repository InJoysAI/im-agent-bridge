//! Bridge 回写 HTTP 客户端（feat-runtime-reply-bridge）
//!
//! 对齐 SSoT: `SSoT/api/main.tsp` `ReplyRequest` 模型 + `POST /bridge/reply` 端点。
//! 响应码集合：200（成功）/ 400（Bad Request，不可重试）/ 401（Unauthorized，不可重试）/
//! 409（重复 reply_id，幂等成功）。HTTP 5xx / 429 / 网络错误 / 超时 视为可重试错误。
//!
//! 指数退避：初始调用 + 3 次重试，延时序列 1s → 2s → 4s（总计最多 4 次尝试）。
//! 安全脱敏：日志 MUST NOT 包含 Authorization 头值或 BRIDGE_BEARER_TOKEN 字面值（RISK-006）。

use std::time::Duration;

use crate::observability::metrics::Metrics;
use serde::Serialize;
use tokio::time::sleep;

/// Gateway 内部回写 payload（保持 SSoT `ReplyRequest` 字段命名，用于日志/幂等追踪）。
///
/// 注意：发送到 Matterbridge 时会由 [`to_matterbridge_message`] 映射为 Matterbridge 原生
/// `config.Message` JSON 结构（字段：`gateway`/`channel`/`text`/`username`），因为当前架构没有
/// 独立的 Bridge 代理服务；`/api/message` 是 Matterbridge 1.26 的消息投递端点。
#[derive(Debug, Clone, Serialize)]
pub struct BridgeReplyPayload {
    pub reply_id: String,
    pub chat_id: String,
    pub platform: String,
    pub text: String,
    pub bridge_gateway_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_channel_name: Option<String>,
}

/// Matterbridge 1.26 `POST /api/message` payload（`config.Message` 的子集）。
///
/// Matterbridge 在 handlePostMessage 内会覆盖 `Channel`/`Account`/`Protocol`/`ID`，
/// 其中 `channel` 仍应显式提供用于定向回写到单个 chat，避免同 gateway 下多 inout 广播。
#[derive(Debug, Serialize)]
struct MatterbridgeMessage<'a> {
    gateway: &'a str,
    channel: &'a str,
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<&'a str>,
}

fn to_matterbridge_message(payload: &BridgeReplyPayload) -> MatterbridgeMessage<'_> {
    MatterbridgeMessage {
        gateway: &payload.bridge_gateway_name,
        channel: &payload.chat_id,
        text: &payload.text,
        username: None,
    }
}

/// Bridge 回写错误。
///
/// - `NonRetryable`：HTTP 400 / 401，**不进入**重试循环；调用方应立即标记 `reply_failed`
///   并记录需人工介入的错误日志。
/// - `RetriesExhausted`：初始调用 + 3 次重试（共 4 次）均失败后的终态错误。
#[derive(Debug)]
pub enum BridgeError {
    /// 不可重试错误：记录 HTTP 状态码。
    NonRetryable { status: u16, body_hint: String },
    /// 重试耗尽：记录最后一次失败原因。
    RetriesExhausted { last_error: String },
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeError::NonRetryable { status, body_hint } => {
                write!(
                    f,
                    "bridge reply non-retryable (status={}, hint={})",
                    status, body_hint
                )
            }
            BridgeError::RetriesExhausted { last_error } => {
                write!(f, "bridge reply retries exhausted: {}", last_error)
            }
        }
    }
}

impl std::error::Error for BridgeError {}

impl BridgeError {
    /// 用于日志与指标标签，便于运维区分失败类别。
    pub fn kind(&self) -> &'static str {
        match self {
            BridgeError::NonRetryable { .. } => "non_retryable",
            BridgeError::RetriesExhausted { .. } => "retries_exhausted",
        }
    }
}

/// 可重试错误分类（内部枚举），不对外暴露。
enum AttemptOutcome {
    Success,
    /// 幂等成功（HTTP 409），立即返回 Ok。
    IdempotentSuccess,
    /// 不可重试错误（HTTP 400 / 401），立即返回 Err。
    NonRetryable {
        status: u16,
        body_hint: String,
    },
    /// 可重试错误，进入下一轮退避。
    Retryable {
        reason: String,
    },
}

/// 退避延时序列（仅用于重试，不含初始调用）。生产使用 1s / 2s / 4s。
const RETRY_DELAYS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
];

/// `POST {BRIDGE_URL}/bridge/reply` + `Authorization: Bearer <token>`。
///
/// # 重试策略
/// - 初始调用 + 最多 3 次重试（延时序列 1s/2s/4s），总计最多 4 次尝试。
/// - HTTP 200 → `Ok(())`。
/// - HTTP 409 → `Ok(())`（幂等成功，不进入重试）。
/// - HTTP 400 / 401 → `Err(NonRetryable)`，**立即失败，不进入重试**。
/// - HTTP 5xx / 429 / 网络错误 / 超时 → 进入重试；4 次尝试全部失败 → `Err(RetriesExhausted)`。
///
/// # 安全脱敏
/// 函数内部日志不得输出 `bearer_token`；调用方自检相同规则。
pub async fn post_reply(
    client: &reqwest::Client,
    bridge_url: &str,
    bearer_token: &str,
    payload: &BridgeReplyPayload,
    metrics: &Metrics,
) -> Result<(), BridgeError> {
    post_reply_with_delays(
        client,
        bridge_url,
        bearer_token,
        payload,
        metrics,
        &RETRY_DELAYS,
    )
    .await
}

/// 与 [`post_reply`] 一致，但允许调用方自定义重试延时序列。
///
/// 仅用于测试场景（传入零延时序列以避免真实等待）。生产代码应始终调用 [`post_reply`]。
pub(crate) async fn post_reply_with_delays(
    client: &reqwest::Client,
    bridge_url: &str,
    bearer_token: &str,
    payload: &BridgeReplyPayload,
    metrics: &Metrics,
    retry_delays: &[Duration],
) -> Result<(), BridgeError> {
    // Matterbridge 1.26 原生消息投递端点。如果未来引入 Bridge 代理层，仅需调整此处路径。
    let url = format!("{}/api/message", bridge_url.trim_end_matches('/'));

    let mut last_retryable_reason: String = String::from("no attempt made");

    // 总尝试 = 初始调用（attempt=0） + 重试（attempt=1..=retry_delays.len()）
    for attempt in 0..=retry_delays.len() {
        let outcome = attempt_once(client, &url, bearer_token, payload).await;
        match outcome {
            AttemptOutcome::Success | AttemptOutcome::IdempotentSuccess => {
                if attempt > 0 {
                    tracing::info!(
                        reply_id = %payload.reply_id,
                        attempt = attempt + 1,
                        "bridge reply succeeded after retry"
                    );
                }
                metrics.reply_write_success_total.inc();
                metrics.messages_replied_total.inc();
                return Ok(());
            }
            AttemptOutcome::NonRetryable { status, body_hint } => {
                tracing::error!(
                    reply_id = %payload.reply_id,
                    http_status = status,
                    "bridge reply non-retryable error; manual intervention required"
                );
                metrics.reply_write_error_total.inc();
                return Err(BridgeError::NonRetryable { status, body_hint });
            }
            AttemptOutcome::Retryable { reason } => {
                last_retryable_reason = reason;
                // 若还剩重试轮次，sleep 相应延时后继续；否则跳出返回耗尽。
                if attempt < retry_delays.len() {
                    let delay = retry_delays[attempt];
                    tracing::warn!(
                        reply_id = %payload.reply_id,
                        attempt = attempt + 1,
                        next_delay_ms = delay.as_millis() as u64,
                        reason = %last_retryable_reason,
                        "bridge reply retryable error, will retry"
                    );
                    sleep(delay).await;
                }
            }
        }
    }

    tracing::error!(
        reply_id = %payload.reply_id,
        last_error = %last_retryable_reason,
        "bridge reply retries exhausted after 4 attempts"
    );
    metrics.reply_write_error_total.inc();
    Err(BridgeError::RetriesExhausted {
        last_error: last_retryable_reason,
    })
}

async fn attempt_once(
    client: &reqwest::Client,
    url: &str,
    bearer_token: &str,
    payload: &BridgeReplyPayload,
) -> AttemptOutcome {
    let mb_message = to_matterbridge_message(payload);
    let send_result = client
        .post(url)
        .bearer_auth(bearer_token)
        .json(&mb_message)
        .send()
        .await;

    match send_result {
        Ok(resp) => {
            let status = resp.status();
            let code = status.as_u16();
            if status.is_success() {
                // 2xx 视为成功（SSoT 只声明 200；其他 2xx 仍视为 Ok 以保持健壮性）
                AttemptOutcome::Success
            } else if code == 409 {
                AttemptOutcome::IdempotentSuccess
            } else if code == 400 || code == 401 {
                // 读取响应体的前若干字节作为提示（便于定位问题），但不包含 Authorization
                let hint = resp
                    .text()
                    .await
                    .unwrap_or_default()
                    .chars()
                    .take(256)
                    .collect::<String>();
                AttemptOutcome::NonRetryable {
                    status: code,
                    body_hint: hint,
                }
            } else {
                // 5xx / 429 / 其他均归入可重试（健壮处理非契约状态码）
                AttemptOutcome::Retryable {
                    reason: format!("http_status={}", code),
                }
            }
        }
        Err(err) => {
            // 网络错误 / 超时 → 可重试
            AttemptOutcome::Retryable {
                reason: format!("transport_error: {}", err),
            }
        }
    }
}

/// 按 UTF-8 字符边界将文本截断至 `limit` 个字符（不是字节），不破坏多字节字符。
///
/// - 超过 `limit` 字符时截断；不足时原样返回。
fn truncate_to_chars(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        text.to_string()
    } else {
        text.chars().take(limit).collect()
    }
}

/// BR-070：落库 `message_events.output_text` 前的 512 字符截断。
///
/// 按 UTF-8 字符边界截断，不破坏多字节字符。
pub fn truncate_to_512(text: &str) -> String {
    truncate_to_chars(text, 512)
}

/// BR-003：回写至 Bridge 的 `text` 字段兜底截断（4096 字符上限）。
///
/// RuntimeAdapter 已承担主截断职责；此函数作为 bridge_client 入口的兜底保护。
pub fn enforce_bridge_text_limit(text: &str) -> String {
    truncate_to_chars(text, 4096)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::metrics::Metrics;
    use prometheus_client::registry::Registry;

    #[test]
    fn truncate_to_512_keeps_text_below_limit() {
        let text = "中".repeat(512);
        let out = truncate_to_512(&text);
        assert_eq!(out.chars().count(), 512);
        assert_eq!(out, text);
    }

    #[test]
    fn truncate_to_512_truncates_over_limit_on_char_boundary() {
        let text = "中".repeat(600);
        let out = truncate_to_512(&text);
        assert_eq!(out.chars().count(), 512);
        // 保证未破坏多字节边界（能从 UTF-8 成功解码；String 本身天然保证）
        assert!(out.is_char_boundary(out.len()));
    }

    #[test]
    fn truncate_to_512_preserves_ascii_short_text() {
        let text = "hello";
        assert_eq!(truncate_to_512(text), "hello");
    }

    #[test]
    fn enforce_bridge_text_limit_caps_at_4096() {
        let text = "a".repeat(5000);
        assert_eq!(enforce_bridge_text_limit(&text).chars().count(), 4096);
    }

    #[test]
    fn enforce_bridge_text_limit_noop_when_within_limit() {
        let text = "a".repeat(4096);
        assert_eq!(enforce_bridge_text_limit(&text).chars().count(), 4096);
    }

    fn sample_payload() -> BridgeReplyPayload {
        BridgeReplyPayload {
            reply_id: "reply-test-1".to_string(),
            chat_id: "chat-1".to_string(),
            platform: "telegram".to_string(),
            text: "hello".to_string(),
            bridge_gateway_name: "default".to_string(),
            bridge_channel_name: None,
        }
    }

    /// 无重试客户端（避免 reqwest 默认 connect_timeout 过长）。
    fn test_client() -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .no_proxy()
            .build()
            .expect("reqwest client")
    }

    /// 零延时重试序列（测试专用），避免真实等待 7 秒。
    /// 生产路径 [`post_reply`] 固定使用 1s / 2s / 4s。
    const ZERO_DELAYS: [Duration; 3] = [
        Duration::from_millis(0),
        Duration::from_millis(0),
        Duration::from_millis(0),
    ];

    async fn post_reply_test(
        client: &reqwest::Client,
        url: &str,
        token: &str,
        payload: &BridgeReplyPayload,
        metrics: &Metrics,
    ) -> Result<(), BridgeError> {
        super::post_reply_with_delays(client, url, token, payload, metrics, &ZERO_DELAYS).await
    }

    #[tokio::test]
    async fn http_200_returns_ok() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/api/message"))
            .and(wiremock::matchers::body_partial_json(serde_json::json!({
                "gateway": "default",
                "channel": "chat-1",
                "text": "hello"
            })))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "status": "ok"
                })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let mut registry = Registry::default();
        let metrics = Metrics::new(&mut registry);
        let result = post_reply_test(
            &test_client(),
            &server.uri(),
            "token-xyz",
            &sample_payload(),
            &metrics,
        )
        .await;
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
    }

    #[tokio::test]
    async fn http_409_returns_ok_without_retry() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/api/message"))
            .respond_with(
                wiremock::ResponseTemplate::new(409).set_body_json(serde_json::json!({
                    "error": "duplicate reply_id"
                })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let mut registry = Registry::default();
        let metrics = Metrics::new(&mut registry);
        let result = post_reply_test(
            &test_client(),
            &server.uri(),
            "token",
            &sample_payload(),
            &metrics,
        )
        .await;
        assert!(
            result.is_ok(),
            "409 should be idempotent success; got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn http_401_returns_non_retryable_immediately() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/api/message"))
            .respond_with(
                wiremock::ResponseTemplate::new(401).set_body_json(serde_json::json!({
                    "error": "unauthorized"
                })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let mut registry = Registry::default();
        let metrics = Metrics::new(&mut registry);
        let result = post_reply_test(
            &test_client(),
            &server.uri(),
            "token",
            &sample_payload(),
            &metrics,
        )
        .await;
        match result {
            Err(BridgeError::NonRetryable { status, .. }) => assert_eq!(status, 401),
            other => panic!("expected NonRetryable(401), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn http_400_returns_non_retryable_immediately() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/api/message"))
            .respond_with(
                wiremock::ResponseTemplate::new(400).set_body_json(serde_json::json!({
                    "error": "bad request"
                })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let mut registry = Registry::default();
        let metrics = Metrics::new(&mut registry);
        let result = post_reply_test(
            &test_client(),
            &server.uri(),
            "token",
            &sample_payload(),
            &metrics,
        )
        .await;
        match result {
            Err(BridgeError::NonRetryable { status, .. }) => assert_eq!(status, 400),
            other => panic!("expected NonRetryable(400), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn http_503_four_attempts_exhausted() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/api/message"))
            .respond_with(wiremock::ResponseTemplate::new(503))
            // 初始调用 + 3 次重试 = 4 次
            .expect(4)
            .mount(&server)
            .await;

        let mut registry = Registry::default();
        let metrics = Metrics::new(&mut registry);
        let result = post_reply_test(
            &test_client(),
            &server.uri(),
            "token",
            &sample_payload(),
            &metrics,
        )
        .await;
        match result {
            Err(BridgeError::RetriesExhausted { .. }) => {}
            other => panic!("expected RetriesExhausted, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn retryable_then_success() {
        let server = wiremock::MockServer::start().await;
        // 第 1 次 503
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/api/message"))
            .respond_with(wiremock::ResponseTemplate::new(503))
            .up_to_n_times(1)
            .expect(1)
            .mount(&server)
            .await;
        // 第 2 次 200
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/api/message"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "status": "ok"
                })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let mut registry = Registry::default();
        let metrics = Metrics::new(&mut registry);
        let result = post_reply_test(
            &test_client(),
            &server.uri(),
            "token",
            &sample_payload(),
            &metrics,
        )
        .await;
        assert!(result.is_ok(), "expected Ok after retry, got {:?}", result);
    }

    /// 日志脱敏硬验证：Debug 格式化 BridgeReplyPayload 不包含 bearer token 字面量。
    ///
    /// （Authorization 头由 reqwest 在内部处理，不会出现在 payload Debug 输出中；
    /// 该测试保证即使业务代码对 payload 执行 `tracing::debug!("{:?}", payload)` 也不泄露 token。）
    #[test]
    fn payload_debug_does_not_leak_bearer_token() {
        let payload = sample_payload();
        let dbg = format!("{:?}", payload);
        assert!(
            !dbg.contains("BEARER"),
            "Debug output must not contain bearer tokens"
        );
        assert!(
            !dbg.contains("Authorization"),
            "Debug output must not contain Authorization header"
        );
    }

    /// 错误 Display 不泄露 token（即使传入一个看似 token 的值，错误仅携带 status/reason）。
    #[test]
    fn bridge_error_display_does_not_leak_token() {
        let err = BridgeError::NonRetryable {
            status: 401,
            body_hint: "unauthorized".to_string(),
        };
        let s = format!("{}", err);
        assert!(!s.to_ascii_lowercase().contains("bearer"));
    }

    /// 群聊 chat_id 产生 channel="-100123"（负数 ID，群聊典型格式）。
    ///
    /// 断言：`to_matterbridge_message` 将 `channel` 绑定为 `BridgeReplyPayload.chat_id`，
    /// 不得使用固定值（如 `"api"`），禁止 gateway 级广播（BR-012，fix-bridge-reply-chat-routing）。
    #[tokio::test]
    async fn channel_in_payload_equals_chat_id_for_group() {
        let group_chat_id = "-100123";
        let payload = BridgeReplyPayload {
            reply_id: "reply-group-1".to_string(),
            chat_id: group_chat_id.to_string(),
            platform: "telegram".to_string(),
            text: "hello group".to_string(),
            bridge_gateway_name: "default".to_string(),
            bridge_channel_name: None,
        };

        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/api/message"))
            .and(wiremock::matchers::body_partial_json(serde_json::json!({
                "channel": group_chat_id
            })))
            .respond_with(wiremock::ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let mut registry = Registry::default();
        let metrics = Metrics::new(&mut registry);
        let result =
            post_reply_test(&test_client(), &server.uri(), "token", &payload, &metrics).await;
        assert!(result.is_ok(), "group chat routing failed: {:?}", result);
    }

    /// 私聊 chat_id 产生 channel="456"（正数 ID，私聊典型格式）。
    ///
    /// 断言同上——`channel` 值必须等于来源 `chat_id`，确保私聊 inout 定向路由。
    #[tokio::test]
    async fn channel_in_payload_equals_chat_id_for_private() {
        let private_chat_id = "456";
        let payload = BridgeReplyPayload {
            reply_id: "reply-private-1".to_string(),
            chat_id: private_chat_id.to_string(),
            platform: "telegram".to_string(),
            text: "hello private".to_string(),
            bridge_gateway_name: "default".to_string(),
            bridge_channel_name: None,
        };

        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/api/message"))
            .and(wiremock::matchers::body_partial_json(serde_json::json!({
                "channel": private_chat_id
            })))
            .respond_with(wiremock::ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let mut registry = Registry::default();
        let metrics = Metrics::new(&mut registry);
        let result =
            post_reply_test(&test_client(), &server.uri(), "token", &payload, &metrics).await;
        assert!(result.is_ok(), "private chat routing failed: {:?}", result);
    }
}
