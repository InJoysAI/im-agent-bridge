// SSoT: SSoT/api/main.tsp — POST /gateway/inbound
// 字段定义与 openapi.yaml components/schemas 保持同步，变更时先修改 SSoT 再更新此文件。

use serde::{Deserialize, Serialize};

// ── 入站请求 ──────────────────────────────────────────────────────────────────

/// Bridge → Gateway 入站请求
#[derive(Debug, Deserialize)]
pub struct InboundRequest {
    pub platform: String,
    pub bridge_gateway_name: String,
    pub bridge_channel_name: Option<String>,
    pub raw_message: RawMessage,
}

/// Bridge 入站消息中的原始消息体
#[derive(Debug, Deserialize)]
pub struct RawMessage {
    pub chat_id: String,
    pub chat_type: ChatType,
    pub user_id: String,
    pub message_type: MessageType,
    pub text: Option<String>,
    pub timestamp: String,
    pub message_id: String,
    pub sender_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatType {
    Private,
    Group,
}

impl ChatType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChatType::Private => "private",
            ChatType::Group => "group",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Text,
    Image,
    Audio,
    Video,
    File,
    Other,
}

// ── 入站响应 ──────────────────────────────────────────────────────────────────

/// 入站响应
#[derive(Debug, Serialize)]
pub struct InboundResponse {
    pub status: InboundStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InboundStatus {
    Accepted,
    IgnoredDuplicate,
    IgnoredNoMention,
}

// ── 统一错误响应（Gateway 自有类型，不由 TypeSpec 生成）──────────────────────

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

// ── ValidatedJson extractor — 将 axum Json 的 422 改为 400 ──────────────────

pub struct ValidatedJson<T>(pub T);

#[async_trait::async_trait]
impl<T, S> axum::extract::FromRequest<S> for ValidatedJson<T>
where
    T: serde::de::DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = (axum::http::StatusCode, axum::Json<ErrorResponse>);

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(val)) => Ok(ValidatedJson(val)),
            Err(err) => Err((
                axum::http::StatusCode::BAD_REQUEST,
                axum::Json(ErrorResponse {
                    error: err.to_string(),
                }),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_response_serializes_correctly() {
        let r = ErrorResponse {
            error: "bad input".to_string(),
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"error\""));
        assert!(json.contains("\"bad input\""));
    }

    #[test]
    fn message_type_deserializes_from_snake_case() {
        let mt: MessageType = serde_json::from_str("\"text\"").unwrap();
        assert_eq!(mt, MessageType::Text);
        let mt: MessageType = serde_json::from_str("\"image\"").unwrap();
        assert_eq!(mt, MessageType::Image);
    }

    #[test]
    fn inbound_status_serializes_to_snake_case() {
        let s = serde_json::to_string(&InboundStatus::Accepted).unwrap();
        assert_eq!(s, "\"accepted\"");
        let s = serde_json::to_string(&InboundStatus::IgnoredDuplicate).unwrap();
        assert_eq!(s, "\"ignored_duplicate\"");
        let s = serde_json::to_string(&InboundStatus::IgnoredNoMention).unwrap();
        assert_eq!(s, "\"ignored_no_mention\"");
    }

    #[test]
    fn inbound_request_deserializes_full_payload() {
        let json = serde_json::json!({
            "platform": "telegram",
            "bridge_gateway_name": "default",
            "raw_message": {
                "chat_id": "c1",
                "chat_type": "private",
                "user_id": "u1",
                "message_type": "text",
                "text": "hello",
                "timestamp": "2026-04-15T00:00:00Z",
                "message_id": "m1"
            }
        });
        let req: InboundRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.platform, "telegram");
        assert_eq!(req.raw_message.message_type, MessageType::Text);
        assert_eq!(req.raw_message.text.as_deref(), Some("hello"));
    }
}
