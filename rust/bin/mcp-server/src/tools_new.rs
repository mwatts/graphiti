use std::collections::HashMap;
use std::sync::Arc;

use graphiti_core::{
    nodes::{EpisodeType, EpisodicNode},
    edges::EntityEdge,
    search::{SearchConfig, SearchResults},
    Graphiti,
    utils::maintenance::graph_data_operations::clear_data,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::config::GraphitiConfig;
use crate::entities;

/// Response for successful operations
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub message: String,
}

/// Response for error operations
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Node result for search responses
#[derive(Debug, Serialize)]
pub struct NodeResult {
    pub uuid: String,
    pub name: String,
    pub summary: String,
    pub labels: Vec<String>,
    pub group_id: String,
    pub created_at: String,
    pub attributes: HashMap<String, Value>,
}

/// Fact search response
#[derive(Debug, Serialize)]
pub struct FactSearchResponse {
    pub message: String,
    pub facts: Vec<HashMap<String, Value>>,
}

/// Episode search response
#[derive(Debug, Serialize)]
pub struct EpisodeSearchResponse {
    pub message: String,
    pub episodes: Vec<HashMap<String, Value>>,
}

/// Node search response
#[derive(Debug, Serialize)]
pub struct NodeSearchResponse {
    pub message: String,
    pub nodes: Vec<NodeResult>,
}

/// Status response
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub message: String,
}

/// Container for all MCP tools
pub struct GraphitiTools {
    graphiti: Option<Arc<Graphiti>>,
    config: Arc<GraphitiConfig>,
    episode_queues: Arc<Mutex<HashMap<String, Vec<Box<dyn Fn() + Send + Sync>>>>>,
}

impl GraphitiTools {
    pub fn new(config: GraphitiConfig, graphiti: Graphiti) -> Self {
        Self {
            graphiti: Some(Arc::new(graphiti)),
            config: Arc::new(config),
            episode_queues: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn set_graphiti(&mut self, graphiti: Graphiti) {
        self.graphiti = Some(Arc::new(graphiti));
    }

    fn get_graphiti(&self) -> Result<&Arc<Graphiti>, String> {
        self.graphiti.as_ref().ok_or_else(|| {
            "Graphiti client not initialized".to_string()
        })
    }

    /// Convert EpisodeType string to enum
    fn parse_episode_type(source: &str) -> EpisodeType {
        match source.to_lowercase().as_str() {
            "message" => EpisodeType::Message,
            "json" => EpisodeType::Json,
            _ => EpisodeType::Text,
        }
    }

    /// Add an episode to memory
    pub async fn add_memory(&self, arguments: Value) -> Value {
        let graphiti = match self.get_graphiti() {
            Ok(g) => g,
            Err(e) => return json!({"error": e}),
        };

        // Parse arguments
        let name = arguments.get("name").and_then(|v| v.as_str()).unwrap_or("Untitled Episode");
        let episode_body = arguments.get("episode_body").and_then(|v| v.as_str()).unwrap_or("");
        let group_id = arguments.get("group_id").and_then(|v| v.as_str())
            .unwrap_or(&self.config.group_id);
        let source = arguments.get("source").and_then(|v| v.as_str()).unwrap_or("text");
        let source_description = arguments.get("source_description").and_then(|v| v.as_str()).unwrap_or("");
        let uuid_str = arguments.get("uuid").and_then(|v| v.as_str());

        let source_type = Self::parse_episode_type(source);

        let uuid = if let Some(uuid_str) = uuid_str {
            match Uuid::parse_str(uuid_str) {
                Ok(uuid) => Some(uuid),
                Err(_) => {
                    return json!({"error": "Invalid UUID format"});
                }
            }
        } else {
            None
        };

        // Use entity types if custom entities are enabled
        let entity_types = if self.config.use_custom_entities {
            entities::get_entity_types()
        } else {
            HashMap::new()
        };

        match graphiti.add_episode(
            name,
            episode_body,
            source_type,
            Some(source_description.to_string()),
            group_id,
            uuid,
            Utc::now(),
            entity_types,
        ).await {
            Ok(_) => {
                info!("Episode '{}' added successfully", name);
                json!({"message": format!("Episode '{}' added successfully", name)})
            }
            Err(e) => {
                error!("Error adding episode: {:?}", e);
                json!({"error": format!("Error adding episode: {:?}", e)})
            }
        }
    }

    /// Search for memory nodes
    pub async fn search_memory_nodes(&self, arguments: Value) -> Value {
        let graphiti = match self.get_graphiti() {
            Ok(g) => g,
            Err(e) => return json!({"error": e}),
        };

        let query = arguments.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let group_ids = arguments.get("group_ids")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(|| vec![self.config.group_id.clone()]);
        let max_nodes = arguments.get("max_nodes").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let center_node_uuid = arguments.get("center_node_uuid").and_then(|v| v.as_str());
        let entity_filter = arguments.get("entity").and_then(|v| v.as_str());

        // TODO: Implement proper node search using Graphiti's search functionality
        // For now, return a placeholder response
        match graphiti.search(&group_ids, query, max_nodes, center_node_uuid.map(|s| s.to_string())).await {
            Ok(edges) => {
                // Convert edges to a node-like response for now
                let facts: Vec<Value> = edges.iter().map(|edge| {
                    json!({
                        "uuid": edge.uuid,
                        "fact": edge.fact,
                        "source_node_uuid": edge.source_node_uuid,
                        "target_node_uuid": edge.target_node_uuid,
                        "created_at": edge.created_at,
                        "valid": edge.valid
                    })
                }).collect();

                json!({
                    "message": "Search completed successfully",
                    "nodes": facts
                })
            }
            Err(e) => {
                error!("Error searching nodes: {:?}", e);
                json!({"error": format!("Error searching nodes: {:?}", e)})
            }
        }
    }

    /// Search for memory facts
    pub async fn search_memory_facts(&self, arguments: Value) -> Value {
        let graphiti = match self.get_graphiti() {
            Ok(g) => g,
            Err(e) => return json!({"error": e}),
        };

        let query = arguments.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let group_ids = arguments.get("group_ids")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(|| vec![self.config.group_id.clone()]);
        let max_facts = arguments.get("max_facts").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let center_node_uuid = arguments.get("center_node_uuid").and_then(|v| v.as_str());

        match graphiti.search(&group_ids, query, max_facts, center_node_uuid.map(|s| s.to_string())).await {
            Ok(edges) => {
                let facts: Vec<Value> = edges.iter().map(|edge| {
                    json!({
                        "uuid": edge.uuid,
                        "fact": edge.fact,
                        "source_node_uuid": edge.source_node_uuid,
                        "target_node_uuid": edge.target_node_uuid,
                        "created_at": edge.created_at,
                        "valid": edge.valid,
                        "source_node_name": edge.source_node_name,
                        "target_node_name": edge.target_node_name
                    })
                }).collect();

                json!({
                    "message": "Facts retrieved successfully",
                    "facts": facts
                })
            }
            Err(e) => {
                error!("Error searching facts: {:?}", e);
                json!({"error": format!("Error searching facts: {:?}", e)})
            }
        }
    }

    /// Clear the graph
    pub async fn clear_graph(&self) -> Value {
        let graphiti = match self.get_graphiti() {
            Ok(g) => g,
            Err(e) => return json!({"error": e}),
        };

        match clear_data(&graphiti.driver).await {
            Ok(_) => {
                match graphiti.build_indices_and_constraints().await {
                    Ok(_) => {
                        info!("Graph cleared successfully and indices rebuilt");
                        json!({"message": "Graph cleared successfully and indices rebuilt"})
                    }
                    Err(e) => {
                        error!("Error rebuilding indices: {:?}", e);
                        json!({"error": format!("Error rebuilding indices: {:?}", e)})
                    }
                }
            }
            Err(e) => {
                error!("Error clearing graph: {:?}", e);
                json!({"error": format!("Error clearing graph: {:?}", e)})
            }
        }
    }
}
