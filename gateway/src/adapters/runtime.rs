use async_trait::async_trait;
use uuid::Uuid;

use crate::models::standard_message::StandardMessage;

/// Bot configuration required by RuntimeAdapter implementations.
#[derive(Debug, Clone)]
pub struct BotConfig {
    pub id: Uuid,
    pub runtime_type: String,
    pub runtime_endpoint: String,
    pub runtime_model: String,
}

/// Internal reply produced by a RuntimeAdapter.
#[derive(Debug, Clone)]
pub struct StandardReply {
    pub text: String,
    pub status: String,
}

/// Errors that can occur during runtime processing.
#[derive(Debug)]
pub enum RuntimeError {
    Timeout,
    Unavailable,
    BadResponse(String),
    SessionNotFound,
}

impl RuntimeError {
    pub fn error_code(&self) -> &'static str {
        match self {
            RuntimeError::Timeout => "RUNTIME_TIMEOUT",
            RuntimeError::Unavailable => "RUNTIME_UNAVAILABLE",
            RuntimeError::BadResponse(_) => "RUNTIME_BAD_RESPONSE",
            RuntimeError::SessionNotFound => "RUNTIME_SESSION_NOT_FOUND",
        }
    }

    pub fn user_message(&self) -> &'static str {
        match self {
            RuntimeError::Timeout | RuntimeError::Unavailable => "工具暂不可用，请稍后重试。",
            RuntimeError::BadResponse(_) | RuntimeError::SessionNotFound => {
                "抱歉，当前无法处理您的请求，请稍后再试。"
            }
        }
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::Timeout => write!(f, "runtime timeout"),
            RuntimeError::Unavailable => write!(f, "runtime unavailable"),
            RuntimeError::BadResponse(msg) => write!(f, "bad response: {}", msg),
            RuntimeError::SessionNotFound => write!(f, "session not found"),
        }
    }
}

/// Strategy trait for Runtime Adapters (criterion.md §3.5).
/// Implementations convert StandardMessage → Runtime HTTP request
/// and Runtime output → StandardReply.
#[async_trait]
pub trait RuntimeAdapter: Send + Sync {
    async fn process(
        &self,
        msg: &StandardMessage,
        bot: &BotConfig,
    ) -> Result<StandardReply, RuntimeError>;
}
