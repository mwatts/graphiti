use std::sync::Arc;

use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::Json,
    routing::{delete, post},
    Router,
};
use graphiti_core::nodes::EpisodeType;
use uuid::Uuid;

use crate::{
    dto::{AddEntityNodeRequest, AddMessagesRequest, Result as ApiResult},
    service::GraphitiService,
};

/// Create ingest router
pub fn create_router() -> Router {
    Router::new()
        .route("/messages", post(add_messages))
        .route("/entity-node", post(add_entity_node))
        .route("/entity-edge/:uuid", delete(delete_entity_edge))
        .route("/group/:group_id", delete(delete_group))
        .route("/episode/:uuid", delete(delete_episode))
}

/// Add messages to the graph
async fn add_messages(
    Extension(service): Extension<Arc<GraphitiService>>,
    Json(request): Json<AddMessagesRequest>,
) -> Result<(StatusCode, Json<ApiResult>), StatusCode> {
    // Process each message asynchronously
    // Note: The original Python version uses a queue-based worker for this
    // For now, we'll process them directly

    for message in request.messages {
        let episode_body = format!(
            "{}({}): {}",
            message.role.as_deref().unwrap_or(""),
            message.role_type.as_deref().unwrap_or(""),
            message.content
        );

        match service.add_episode(
            message.name.unwrap_or_else(|| "Message".to_string()),
            episode_body,
            EpisodeType::Message,
            message.source_description,
            request.group_id.clone(),
            Some(message.timestamp),
        ).await {
            Ok(_) => {
                // Episode added successfully
            }
            Err(_) => {
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(ApiResult {
            message: "Messages added to processing queue".to_string(),
            success: true,
        }),
    ))
}

/// Add an entity node
async fn add_entity_node(
    Extension(service): Extension<Arc<GraphitiService>>,
    Json(request): Json<AddEntityNodeRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    match service.save_entity_node(
        request.uuid,
        request.group_id,
        request.name,
        request.summary,
    ).await {
        Ok(node) => {
            let node_json = serde_json::to_value(&node).unwrap_or_default();
            Ok((StatusCode::CREATED, Json(node_json)))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Delete an entity edge
async fn delete_entity_edge(
    Extension(service): Extension<Arc<GraphitiService>>,
    Path(uuid): Path<String>,
) -> Result<Json<ApiResult>, StatusCode> {
    let uuid = Uuid::parse_str(&uuid).map_err(|_| StatusCode::BAD_REQUEST)?;

    match service.delete_entity_edge(uuid).await {
        Ok(_) => Ok(Json(ApiResult {
            message: "Entity Edge deleted".to_string(),
            success: true,
        })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Delete a group
async fn delete_group(
    Extension(service): Extension<Arc<GraphitiService>>,
    Path(group_id): Path<String>,
) -> Result<Json<ApiResult>, StatusCode> {
    match service.delete_group(group_id).await {
        Ok(_) => Ok(Json(ApiResult {
            message: "Group deleted".to_string(),
            success: true,
        })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Delete an episode
async fn delete_episode(
    Extension(service): Extension<Arc<GraphitiService>>,
    Path(uuid): Path<String>,
) -> Result<Json<ApiResult>, StatusCode> {
    let uuid = Uuid::parse_str(&uuid).map_err(|_| StatusCode::BAD_REQUEST)?;

    match service.delete_episode(uuid).await {
        Ok(_) => Ok(Json(ApiResult {
            message: "Episode deleted".to_string(),
            success: true,
        })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
