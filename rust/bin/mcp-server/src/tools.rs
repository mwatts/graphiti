use std::collections::HashMap;
use std::sync::Arc;

use graphiti_core::{
    nodes::{EpisodeType, Node},
    edges::Edge,
    Graphiti,
};
use serde::Serialize;
use serde_json::{json, Value};
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
}

impl GraphitiTools {
    pub fn new(config: GraphitiConfig, graphiti: Graphiti) -> Self {
        Self {
            graphiti: Some(Arc::new(graphiti)),
            config: Arc::new(config),
        }
    }

    pub async fn set_graphiti(&mut self, graphiti: Graphiti) {
        self.graphiti = Some(Arc::new(graphiti));
    }

    #[allow(dead_code)]
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
        let graphiti = match self.get_graphiti() {
            Ok(g) => g,
            Err(e) => return json!({"error": e}),
        };

        let query = arguments.get("query").and_then(|v| v.as_str()).unwrap_or("");
        
        if query.is_empty() {
            return json!({"error": "Query parameter is required"});
        }

        info!("Search nodes query: {}", query);

        match graphiti.search(query, None, None).await {
            Ok(results) => {
                let nodes: Vec<NodeResult> = results.nodes.into_iter().map(|search_result| {
                    let node = &search_result.item;
                    let mut attributes = HashMap::new();
                    attributes.insert("score".to_string(), json!(search_result.score));
                    
                    NodeResult {
                        uuid: node.uuid().to_string(),
                        name: node.name().to_string(),
                        summary: node.summary.clone(),
                        labels: node.labels().to_vec(),
                        group_id: node.group_id().to_string(),
                        created_at: node.created_at().to_rfc3339(),
                        attributes,
                    }
                }).collect();

                json!({
                    "message": format!("Found {} nodes", nodes.len()),
                    "nodes": nodes
                })
            }
            Err(e) => {
                error!("Search error: {:?}", e);
                json!({"error": format!("Search failed: {:?}", e)})
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
        
        if query.is_empty() {
            return json!({"error": "Query parameter is required"});
        }

        info!("Search facts query: {}", query);

        match graphiti.search(query, None, None).await {
            Ok(results) => {
                let facts: Vec<HashMap<String, Value>> = results.edges.into_iter().map(|search_result| {
                    let edge = &search_result.item;
                    let mut fact = HashMap::new();
                    fact.insert("uuid".to_string(), json!(edge.uuid()));
                    fact.insert("fact".to_string(), json!(edge.fact));
                    fact.insert("source_node_uuid".to_string(), json!(edge.source_node_uuid()));
                    fact.insert("target_node_uuid".to_string(), json!(edge.target_node_uuid()));
                    fact.insert("group_id".to_string(), json!(edge.group_id()));
                    fact.insert("created_at".to_string(), json!(edge.created_at().to_rfc3339()));
                    fact.insert("score".to_string(), json!(search_result.score));
                    fact
                }).collect();

                json!({
                    "message": format!("Found {} facts", facts.len()),
                    "facts": facts
                })
            }
            Err(e) => {
                error!("Search error: {:?}", e);
                json!({"error": format!("Search failed: {:?}", e)})
            }
        }
    }

    /// Clear the graph
    pub async fn clear_graph(&self) -> Value {
        let graphiti = match self.get_graphiti() {
            Ok(g) => g,
            Err(e) => return json!({"error": e}),
        };
        
        info!("Clearing graph for group_id: {}", self.config.group_id);
        
        // Use the client graph access to delete by group_id
        let graph = &graphiti.clients().driver;
        
        match graphiti_core::nodes::BaseNode::delete_by_group_id(graph, &self.config.group_id).await {
            Ok(_) => {
                info!("Graph cleared successfully for group_id: {}", self.config.group_id);
                json!({"message": format!("Graph cleared successfully for group_id: {}", self.config.group_id)})
            }
            Err(e) => {
                error!("Error clearing graph: {:?}", e);
                json!({"error": format!("Error clearing graph: {:?}", e)})
            }
        }
    }
}
