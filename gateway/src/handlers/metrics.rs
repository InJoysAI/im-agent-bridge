use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};

use crate::handlers::inbound::InboundHandlerState;
use crate::observability::metrics::encode_metrics;

pub async fn metrics_handler(State(state): State<InboundHandlerState>) -> Response {
    let body = encode_metrics(state.metrics_registry.as_ref());
    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use axum::{Extension, Router};
    use prometheus_client::registry::Registry;
    use tower::ServiceExt;

    use crate::adapters::nanobot::NanoBotAdapter;
    use crate::middleware::rate_limit::RateLimiter;
    use crate::observability::metrics::Metrics;

    use super::*;

    #[tokio::test]
    async fn metrics_handler_returns_200_and_text_plain() {
        let mut registry = Registry::default();
        let metrics = Arc::new(Metrics::new(&mut registry));
        let app = Router::new()
            .route("/metrics", get(metrics_handler))
            .with_state(InboundHandlerState {
                bridge_url: "http://127.0.0.1:1".to_string(),
                bridge_bearer_token: "bridge-token".to_string(),
                rate_limiter: Arc::new(RateLimiter::new()),
                nanobot_adapter: Arc::new(NanoBotAdapter::new()),
                bridge_http_client: Arc::new(reqwest::Client::new()),
                metrics_registry: Arc::new(registry),
                metrics,
            })
            .layer(Extension(
                sqlx::postgres::PgPoolOptions::new()
                    .connect_lazy("postgres://postgres:postgres@127.0.0.1:1/im_agent_bridge")
                    .expect("lazy pool should build"),
            ));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            content_type.starts_with("text/plain; version=0.0.4"),
            "unexpected content-type: {content_type}"
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("metrics response body");
        let text = String::from_utf8(body.to_vec()).expect("utf8 metrics body");
        assert!(
            text.contains("# HELP"),
            "metrics output should include HELP"
        );
        assert!(
            text.contains("# TYPE"),
            "metrics output should include TYPE"
        );
    }
}
