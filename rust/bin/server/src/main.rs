use std::sync::Arc;

use axum::{
    extract::Extension,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::{info, instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod dto;
mod routers;
mod service;

use config::Settings;
use service::GraphitiService;

/// Health check endpoint
#[instrument]
async fn healthcheck() -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({"status": "healthy"})))
}

/// Initialize the Axum web server
async fn create_app(settings: Settings) -> Result<Router, anyhow::Error> {
    // Initialize Graphiti service
    let graphiti_service = Arc::new(GraphitiService::new(settings).await?);

    // Build the router with all routes
    let app = Router::new()
        .route("/healthcheck", get(healthcheck))
        .nest("/api", routers::create_router())
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default()))
                .layer(CorsLayer::permissive())
                .layer(Extension(graphiti_service)),
        );

    Ok(app)
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "graphiti_server=debug,tower_http=debug,axum::rejection=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let settings = Settings::load()?;
    info!("Starting Graphiti server with settings: {:?}", settings);

    // Create the app
    let app = create_app(settings.clone()).await?;

    // Start the server
    let listener = tokio::net::TcpListener::bind(&settings.server_address()).await?;
    info!("Server listening on {}", settings.server_address());

    axum::serve(listener, app).await?;

    Ok(())
}
