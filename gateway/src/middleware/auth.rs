use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    Json,
};
use constant_time_eq::constant_time_eq;
use uuid::Uuid;

use crate::models::inbound::ErrorResponse;

/// Newtype injected via `.layer(Extension(BearerTokenConfig(token)))` in main.rs.
#[derive(Clone)]
pub struct BearerTokenConfig(pub String);

/// Axum extractor — validates `Authorization: Bearer <token>` via constant-time comparison.
/// Reads expected token from `Extension<BearerTokenConfig>` inserted by main.rs.
/// Returns HTTP 401 + `{"error":"Unauthorized"}` when validation fails.
/// Token value is never emitted to tracing/logs.
pub struct BearerAuth {
    pub event_id: String,
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for BearerAuth {
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let event_id = Uuid::new_v4().to_string();
        let expected = parts
            .extensions
            .get::<BearerTokenConfig>()
            .map(|c| c.0.as_str())
            .unwrap_or("");

        let provided = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .unwrap_or("");

        if expected.is_empty() || !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
            tracing::warn!(
                event_id = %event_id,
                "bearer auth failed"
            );
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Unauthorized".to_string(),
                }),
            ));
        }

        tracing::info!(
            event_id = %event_id,
            "bearer auth succeeded"
        );
        Ok(BearerAuth { event_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn dummy_handler(_auth: BearerAuth) -> &'static str {
        "ok"
    }

    fn make_app(token: &str) -> Router {
        Router::new()
            .route("/test", get(dummy_handler))
            .layer(axum::Extension(BearerTokenConfig(token.to_string())))
    }

    #[tokio::test]
    async fn no_auth_header_returns_401() {
        let app = make_app("secret");
        let resp = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn invalid_token_returns_401() {
        let app = make_app("secret");
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .header("authorization", "Bearer wrong")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn valid_token_passes() {
        let app = make_app("secret");
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .header("authorization", "Bearer secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
