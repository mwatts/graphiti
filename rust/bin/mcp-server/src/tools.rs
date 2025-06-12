use std::collections::HashMap;
use std::sync::Arc;

use graphiti_core::{
    nodes::EpisodeType,
    Graphiti,
};
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tracing::{error, info};
use chrono::Utc;

use crate::config::GraphitiConfig;

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

        let source_type = Self::parse_episode_type(source);

        // Note: UUID and entity_types are not currently supported in the Rust implementation
        // but we acknowledge the parameters for API compatibility

        match graphiti.add_episode(
            name.to_string(),
            episode_body.to_string(),
            source_type,
            source_description.to_string(),
            group_id.to_string(),
            Some(Utc::now()),
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
        // For now, return a placeholder since the search API isn't fully implemented
        // This maintains API compatibility while we work on the core implementation
        
        let query = arguments.get("query").and_then(|v| v.as_str()).unwrap_or("");
        
        info!("Search nodes query: {}", query);
        
        // Return empty results for now - this maintains API compatibility
        json!({
            "message": "Search completed successfully (placeholder implementation)",
            "nodes": []
        })
    }

    /// Search for memory facts  
    pub async fn search_memory_facts(&self, arguments: Value) -> Value {
        // For now, return a placeholder since the search API isn't fully implemented
        // This maintains API compatibility while we work on the core implementation
        
        let query = arguments.get("query").and_then(|v| v.as_str()).unwrap_or("");
        
        info!("Search facts query: {}", query);
        
        // Return empty results for now - this maintains API compatibility
        json!({
            "message": "Facts retrieved successfully (placeholder implementation)",
            "facts": []
        })
    }

    /// Clear the graph
    pub async fn clear_graph(&self) -> Value {
        // For now, return a success message since clear functionality isn't implemented yet
        // This maintains API compatibility while we work on the core implementation
        
        info!("Clear graph requested (placeholder implementation)");
        
        json!({"message": "Graph clear functionality not yet implemented in Rust version"})
    }
}
