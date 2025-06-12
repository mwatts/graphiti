use std::collections::HashMap;
use std::sync::Arc;

use graphiti_core::{
    nodes::{EpisodeType, EpisodicNode},
    edges::EntityEdge,
    search::{SearchConfig, SearchResults},
    Graphiti,
};
use jsonrpc_core::{Error as JsonRpcError, Result as JsonRpcResult, Params, Value, ErrorCode, IoHandler};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use uuid::Uuid;
use chrono::{DateTime, Utc};

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
pub struct GraphitiMcpServer {
    graphiti: Option<Arc<Graphiti>>,
    config: Arc<GraphitiConfig>,
    episode_queues: Arc<Mutex<HashMap<String, Vec<Box<dyn Fn() + Send + Sync>>>>>,
}

impl GraphitiMcpServer {
    pub fn new(config: GraphitiConfig) -> Self {
        Self {
            graphiti: None,
            config: Arc::new(config),
            episode_queues: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn set_graphiti(&mut self, graphiti: Graphiti) {
        self.graphiti = Some(Arc::new(graphiti));
    }

    fn get_graphiti(&self) -> Result<&Arc<Graphiti>, JsonRpcError> {
        self.graphiti.as_ref().ok_or_else(|| {
            JsonRpcError::new(ErrorCode::InternalError)
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

    /// Register all MCP methods with the JSON-RPC handler
    pub fn register_methods(&self, handler: &mut IoHandler) {
        let server = Arc::new(self.clone());

        // Register add_memory tool
        {
            let server = server.clone();
            handler.add_method("tools/call", move |params| {
                let server = server.clone();
                Box::pin(async move {
                    server.handle_tool_call(params).await
                })
            });
        }

        // Register list tools
        {
            let server = server.clone();
            handler.add_method("tools/list", move |_params| {
                let server = server.clone();
                Box::pin(async move {
                    server.list_tools().await
                })
            });
        }

        // Register initialize
        {
            let server = server.clone();
            handler.add_method("initialize", move |params| {
                let server = server.clone();
                Box::pin(async move {
                    server.initialize(params).await
                })
            });
        }
    }

    async fn initialize(&self, _params: Params) -> JsonRpcResult<Value> {
        Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "graphiti-mcp-server",
                "version": "0.1.0"
            }
        }))
    }

    async fn list_tools(&self) -> JsonRpcResult<Value> {
        Ok(json!({
            "tools": [
                {
                    "name": "add_memory",
                    "description": "Add an episode to memory. This is the primary way to add information to the graph.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Name of the episode"
                            },
                            "episode_body": {
                                "type": "string",
                                "description": "The content of the episode to persist to memory"
                            },
                            "group_id": {
                                "type": "string",
                                "description": "A unique ID for this graph. If not provided, uses the default group_id"
                            },
                            "source": {
                                "type": "string",
                                "enum": ["text", "json", "message"],
                                "default": "text",
                                "description": "Source type"
                            },
                            "source_description": {
                                "type": "string",
                                "default": "",
                                "description": "Description of the source"
                            },
                            "uuid": {
                                "type": "string",
                                "description": "Optional UUID for the episode"
                            }
                        },
                        "required": ["name", "episode_body"]
                    }
                },
                {
                    "name": "search_facts",
                    "description": "Search for facts (edges) in the knowledge graph",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Search query"
                            },
                            "group_id": {
                                "type": "string",
                                "description": "Group ID to filter results"
                            },
                            "limit": {
                                "type": "integer",
                                "default": 10,
                                "description": "Maximum number of results to return"
                            }
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "clear_graph",
                    "description": "Clear all data from the graph. WARNING: This is irreversible!",
                    "inputSchema": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }
            ]
        }))
    }

    async fn handle_tool_call(&self, params: Params) -> JsonRpcResult<Value> {
        let call_params: Value = params.parse()?;

        let tool_name = call_params.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing tool name"))?;

        let arguments = call_params.get("arguments")
            .cloned()
            .unwrap_or(json!({}));

        match tool_name {
            "add_memory" => self.add_memory(arguments).await,
            "search_facts" => self.search_facts(arguments).await,
            "clear_graph" => self.clear_graph(arguments).await,
            _ => Err(JsonRpcError::method_not_found()),
        }
    }

    async fn add_memory(&self, params: Value) -> JsonRpcResult<Value> {
        #[derive(Deserialize)]
        struct AddMemoryParams {
            name: String,
            episode_body: String,
            group_id: Option<String>,
            #[serde(default = "default_source")]
            source: String,
            #[serde(default)]
            source_description: String,
            uuid: Option<String>,
        }

        fn default_source() -> String {
            "text".to_string()
        }

        let params: AddMemoryParams = serde_json::from_value(params)
            .map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        let graphiti = self.get_graphiti()?;

        let source_type = Self::parse_episode_type(&params.source);
        let effective_group_id = params.group_id
            .unwrap_or_else(|| self.config.group_id.clone());

        match graphiti.add_episode(
            params.name.clone(),
            params.episode_body,
            source_type,
            params.source_description,
            effective_group_id.clone(),
            None, // reference_time will use current time
        ).await {
            Ok(_) => {
                info!("Episode '{}' added successfully for group_id: {}", params.name, effective_group_id);
                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Episode '{}' added successfully", params.name)
                    }]
                }))
            },
            Err(e) => {
                error!("Error adding episode '{}': {}", params.name, e);
                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Error adding episode: {}", e)
                    }],
                    "isError": true
                }))
            }
        }
    }

    async fn search_facts(&self, params: Value) -> JsonRpcResult<Value> {
        #[derive(Deserialize)]
        struct SearchFactsParams {
            query: String,
            group_id: Option<String>,
            #[serde(default = "default_limit")]
            limit: usize,
        }

        fn default_limit() -> usize {
            10
        }

        let params: SearchFactsParams = serde_json::from_value(params)
            .map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        let graphiti = self.get_graphiti()?;

        let mut search_config = SearchConfig::default();
        search_config.limit = params.limit;

        let filters = graphiti_core::search::SearchFilters::default();

        match graphiti.search(&params.query, Some(search_config), Some(filters)).await {
            Ok(results) => {
                let facts: Vec<HashMap<String, Value>> = results.edges.iter().map(|edge| {
                    let mut fact = HashMap::new();
                    fact.insert("uuid".to_string(), json!(edge.item.base.uuid.to_string()));
                    fact.insert("fact".to_string(), json!(edge.item.fact));
                    fact.insert("valid_at".to_string(), json!(edge.item.valid_at.to_rfc3339()));
                    if let Some(invalid_at) = edge.item.invalid_at {
                        fact.insert("invalid_at".to_string(), json!(invalid_at.to_rfc3339()));
                    }
                    fact.insert("score".to_string(), json!(edge.score));
                    fact
                }).collect();

                let response = FactSearchResponse {
                    message: format!("Found {} facts", facts.len()),
                    facts,
                };

                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string_pretty(&response).unwrap_or_default()
                    }]
                }))
            },
            Err(e) => {
                error!("Error searching facts: {}", e);
                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Error searching facts: {}", e)
                    }],
                    "isError": true
                }))
            }
        }
    }

    async fn clear_graph(&self, _params: Value) -> JsonRpcResult<Value> {
        warn!("Clear graph requested but not implemented in Rust version");
        Ok(json!({
            "content": [{
                "type": "text",
                "text": "clear_graph not yet implemented in Rust version"
            }],
            "isError": true
        }))
    }
}

// Make it clonable for use with Arc
impl Clone for GraphitiMcpServer {
    fn clone(&self) -> Self {
        Self {
            graphiti: self.graphiti.clone(),
            config: self.config.clone(),
            episode_queues: self.episode_queues.clone(),
        }
    }
}
