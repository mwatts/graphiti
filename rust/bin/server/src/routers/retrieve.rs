use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    dto::{FactResult, GetMemoryRequest, GetMemoryResponse, SearchQuery, SearchResults},
    service::GraphitiService,
};

/// Query parameters for getting episodes
#[derive(Deserialize)]
pub struct GetEpisodesQuery {
    pub last_n: usize,
}

/// Create retrieve router
pub fn create_router() -> Router {
    Router::new()
        .route("/search", post(search))
        .route("/entity-edge/:uuid", get(get_entity_edge))
        .route("/episodes/:group_id", get(get_episodes))
        .route("/get-memory", post(get_memory))
}

/// Search for facts
async fn search(
    Extension(service): Extension<Arc<GraphitiService>>,
    Json(query): Json<SearchQuery>,
) -> Result<Json<SearchResults>, StatusCode> {
    match service.search(query.query, Some(query.group_ids), Some(query.max_facts)).await {
        Ok(results) => {
            // Convert search results to facts
            let facts: Vec<FactResult> = results.edges.iter().map(|edge| {
                FactResult {
                    fact: edge.fact.clone(),
                    uuid: edge.base.uuid.to_string(),
                    valid_at: edge.valid_at.map(|dt| dt.to_rfc3339()),
                    invalid_at: edge.invalid_at.map(|dt| dt.to_rfc3339()),
                    source_description: "".to_string(), // TODO: Add source description to edges
                    episodes: Vec::new(), // TODO: Add episode references
                }
            }).collect();
            
            Ok(Json(SearchResults { facts }))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Get entity edge by UUID
async fn get_entity_edge(
    Extension(service): Extension<Arc<GraphitiService>>,
    Path(uuid): Path<String>,
) -> Result<Json<FactResult>, StatusCode> {
    let uuid = Uuid::parse_str(&uuid).map_err(|_| StatusCode::BAD_REQUEST)?;
    
    match service.get_entity_edge(uuid).await {
        Ok(Some(edge)) => {
            let fact = FactResult {
                fact: edge.fact,
                uuid: edge.base.uuid.to_string(),
                valid_at: edge.valid_at.map(|dt| dt.to_rfc3339()),
                invalid_at: edge.invalid_at.map(|dt| dt.to_rfc3339()),
                source_description: "".to_string(), // TODO: Add source description
                episodes: Vec::new(), // TODO: Add episode references
            };
            Ok(Json(fact))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Get episodes for a group
async fn get_episodes(
    Extension(service): Extension<Arc<GraphitiService>>,
    Path(group_id): Path<String>,
    Query(params): Query<GetEpisodesQuery>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let reference_time = Utc::now();
    
    match service.retrieve_episodes(vec![group_id], params.last_n, reference_time).await {
        Ok(episodes) => {
            // Convert episodes to JSON
            let episodes_json: Vec<serde_json::Value> = episodes
                .iter()
                .map(|episode| serde_json::to_value(episode).unwrap_or_default())
                .collect();
            Ok(Json(episodes_json))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Get memory from messages
async fn get_memory(
    Extension(service): Extension<Arc<GraphitiService>>,
    Json(request): Json<GetMemoryRequest>,
) -> Result<Json<GetMemoryResponse>, StatusCode> {
    // Compose query from messages
    let combined_query = request.messages
        .iter()
        .map(|msg| format!("{}({}): {}", msg.role_type.as_deref().unwrap_or(""), msg.role.as_deref().unwrap_or(""), msg.content))
        .collect::<Vec<_>>()
        .join("\n");

    match service.search(combined_query, Some(vec![request.group_id]), Some(request.max_facts)).await {
        Ok(results) => {
            let facts: Vec<FactResult> = results.edges.iter().map(|edge| {
                FactResult {
                    fact: edge.fact.clone(),
                    uuid: edge.base.uuid.to_string(),
                    valid_at: edge.valid_at.map(|dt| dt.to_rfc3339()),
                    invalid_at: edge.invalid_at.map(|dt| dt.to_rfc3339()),
                    source_description: "".to_string(),
                    episodes: Vec::new(),
                }
            }).collect();

            Ok(Json(GetMemoryResponse { facts }))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
