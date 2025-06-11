use axum::Router;

pub mod ingest;
pub mod retrieve;

/// Create the main API router
pub fn create_router() -> Router {
    Router::new()
        .nest("/ingest", ingest::create_router())
        .nest("/retrieve", retrieve::create_router())
}
