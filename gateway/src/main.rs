mod adapters;
mod bridge_client;
mod config;
mod db;
mod errors;
mod handlers;
mod middleware;
mod models;
mod observability;

use std::sync::Arc;

use adapters::nanobot::NanoBotAdapter;
use axum::{
    routing::{get, post},
    Extension, Router,
};
use handlers::health::health_handler;
use handlers::inbound::{inbound_handler, InboundHandlerState};
use handlers::metrics::metrics_handler;
use middleware::auth::BearerTokenConfig;
use middleware::rate_limit::RateLimiter;
use observability::metrics::Metrics;
use prometheus_client::registry::Registry;

#[tokio::main]
async fn main() {
    observability::init_subscriber();

    dotenvy::dotenv().ok();
    let config = config::AppConfig::from_env();

    let pool = db::init_pool(&config).await;
    if let Err(err) = db::validate_required_schema(&pool).await {
        tracing::error!(
            error = %err,
            "goose migration verification failed; ensure SSoT/schema/migrations has been applied"
        );
        std::process::exit(1);
    }

    tracing::info!("Starting Matterbridge poller → {}", config.bridge_url);
    tokio::spawn(adapters::matterbridge::run_poller(
        config.bridge_url.clone(),
        "http://localhost:8080".to_string(),
        config.gateway_bearer_token.clone(),
    ));

    let rate_limiter = Arc::new(RateLimiter::new());
    let nanobot_adapter = Arc::new(NanoBotAdapter::new());
    let mut metrics_registry = Registry::default();
    let metrics = Arc::new(Metrics::new(&mut metrics_registry));
    let bridge_http_client = Arc::new(
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .no_proxy()
            .build()
            .expect("failed to build reqwest client for bridge_client"),
    );

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .route("/gateway/inbound", post(inbound_handler))
        .with_state(InboundHandlerState {
            bridge_url: config.bridge_url,
            bridge_bearer_token: config.bridge_bearer_token,
            rate_limiter,
            nanobot_adapter,
            bridge_http_client,
            metrics_registry: Arc::new(metrics_registry),
            metrics,
        })
        .layer(Extension(pool))
        .layer(Extension(BearerTokenConfig(config.gateway_bearer_token)));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("failed to bind to port 8080");

    println!("Gateway listening on 0.0.0.0:8080");
    axum::serve(listener, app).await.expect("server error");
}
