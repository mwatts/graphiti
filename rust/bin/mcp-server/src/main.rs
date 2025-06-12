use std::collections::HashMap;
use std::sync::Arc;

use clap::Parser;
use graphiti_core::{
    cross_encoder::openai_reranker_client::OpenAIRerankerClient,
    embedder::openai::{OpenAiEmbedder, OpenAiEmbedderConfig},
    llm_client::{config::LlmConfig, openai_client::OpenAiClient},
    Graphiti, GraphitiConfig as CoreGraphitiConfig,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use tracing::{error, info, warn, Level};
use tracing_subscriber;

mod config;
mod entities;
mod tools;

use config::{Args, GraphitiConfig, Transport as ConfigTransport};
use tools::GraphitiTools;

const MCP_INSTRUCTIONS: &str = r#"
Graphiti is a memory service for AI agents built on a knowledge graph. Graphiti performs well
with dynamic data such as user interactions, changing enterprise data, and external information.

Graphiti transforms information into a richly connected knowledge network, allowing you to
capture relationships between concepts, entities, and information. The system organizes data as episodes
(content snippets), nodes (entities), and facts (relationships between entities), creating a dynamic,
queryable memory store that evolves with new information. Graphiti supports multiple data formats, including
structured JSON data, enabling seamless integration with existing data pipelines and systems.

Facts contain temporal metadata, allowing you to track the time of creation and whether a fact is invalid
(superseded by new information).

Key capabilities:
1. Add episodes (text, messages, or JSON) to the knowledge graph with the add_memory tool
2. Search for nodes (entities) in the graph using natural language queries with search_memory_nodes
3. Find relevant facts (relationships between entities) with search_memory_facts
4. Retrieve specific entity edges or episodes by UUID
5. Manage the knowledge graph with tools like delete_episode, delete_entity_edge, and clear_graph

The server connects to a database for persistent storage and uses language models for certain operations.
Each piece of information is organized by group_id, allowing you to maintain separate knowledge domains.

When adding information, provide descriptive names and detailed content to improve search quality.
When searching, use specific queries and consider filtering by group_id for more relevant results.

For optimal performance, ensure the database is properly configured and accessible, and valid
API keys are provided for any language model operations.
"#;

#[derive(Debug, Serialize, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct McpResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

#[derive(Debug, Serialize)]
struct McpError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

#[derive(Debug, Serialize)]
struct ServerInfo {
    name: String,
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    instructions: Option<String>,
}

#[derive(Debug, Serialize)]
struct ToolInfo {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: Value,
}

async fn initialize_graphiti(config: &GraphitiConfig) -> anyhow::Result<Graphiti> {
    info!("Initializing Graphiti with group_id: {}", config.group_id);

    // Create core Graphiti config
    let core_config = CoreGraphitiConfig {
        database_config: graphiti_core::database::config::DatabaseConfig {
            database_type: graphiti_core::database::config::DatabaseType::Neo4j,
            uri: config.neo4j.uri.clone(),
            username: Some(config.neo4j.user.clone()),
            password: Some(config.neo4j.password.clone()),
            database: config.neo4j.database.clone(),
            pool_size: Some(10),
            timeout_seconds: Some(30),
            additional_config: std::collections::HashMap::new(),
        },
        store_raw_episode_content: true,
        cache_config: None,
    };

    // Create LLM client
    let llm_config = LlmConfig {
        api_key: config.llm.api_key.clone(),
        model: Some(config.llm.model.clone()),
        base_url: config.llm.azure_openai_endpoint.clone(),
        temperature: config.llm.temperature,
        max_tokens: 8192,
        small_model: Some(config.llm.small_model.clone()),
    };

    let llm_client = Arc::new(
        OpenAiClient::new(llm_config, false)
            .map_err(|e| anyhow::anyhow!("Failed to create LLM client: {:?}", e))?,
    );

    // Create embedder
    let embedder_config = OpenAiEmbedderConfig {
        api_key: config.embedder.api_key.clone(),
        embedding_model: config.embedder.model.clone(),
        base_url: config.embedder.azure_openai_endpoint.clone(),
        ..Default::default()
    };

    let embedder = Arc::new(
        OpenAiEmbedder::new(embedder_config)
            .map_err(|e| anyhow::anyhow!("Failed to create embedder: {:?}", e))?,
    );

    // Create cross encoder
    let cross_encoder = Arc::new(
        OpenAIRerankerClient::new(Default::default())
            .map_err(|e| anyhow::anyhow!("Failed to create cross encoder: {:?}", e))?,
    );

    // Create Graphiti instance
    let graphiti = Graphiti::with_clients(core_config, llm_client, embedder, cross_encoder)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create Graphiti: {:?}", e))?;

    info!("Graphiti initialized successfully");
    Ok(graphiti)
}

fn create_error_response(id: Option<Value>, code: i32, message: String) -> McpResponse {
    McpResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(McpError {
            code,
            message,
            data: None,
        }),
    }
}

fn create_success_response(id: Option<Value>, result: Value) -> McpResponse {
    McpResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(result),
        error: None,
    }
}

async fn handle_request(request: McpRequest, tools: Arc<Mutex<GraphitiTools>>) -> McpResponse {
    match request.method.as_str() {
        "initialize" => {
            let server_info = ServerInfo {
                name: "Graphiti Agent Memory".to_string(),
                version: "0.1.0".to_string(),
                instructions: Some(MCP_INSTRUCTIONS.to_string()),
            };
            create_success_response(
                request.id,
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": server_info
                }),
            )
        }
        "tools/list" => {
            let tool_list = vec![
                ToolInfo {
                    name: "add_memory".to_string(),
                    description: "Add an episode to memory. This is the primary way to add information to the graph.".to_string(),
                    input_schema: json!({
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
                                "description": "Optional group ID for organizing the episode"
                            },
                            "source": {
                                "type": "string",
                                "enum": ["text", "json", "message"],
                                "description": "Source type of the episode"
                            },
                            "source_description": {
                                "type": "string",
                                "description": "Description of the source"
                            },
                            "uuid": {
                                "type": "string",
                                "description": "Optional UUID for the episode"
                            }
                        },
                        "required": ["name", "episode_body"]
                    })
                },
                ToolInfo {
                    name: "search_memory_nodes".to_string(),
                    description: "Search the graph memory for relevant node summaries.".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "The search query"
                            },
                            "group_ids": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "Optional list of group IDs to filter results"
                            },
                            "max_nodes": {
                                "type": "integer",
                                "description": "Maximum number of nodes to return",
                                "default": 10
                            },
                            "center_node_uuid": {
                                "type": "string",
                                "description": "Optional UUID of a node to center the search around"
                            },
                            "entity": {
                                "type": "string",
                                "description": "Optional entity type to filter results"
                            }
                        },
                        "required": ["query"]
                    })
                },
                ToolInfo {
                    name: "search_memory_facts".to_string(),
                    description: "Search the graph memory for relevant facts.".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "The search query"
                            },
                            "group_ids": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "Optional list of group IDs to filter results"
                            },
                            "max_facts": {
                                "type": "integer",
                                "description": "Maximum number of facts to return",
                                "default": 10
                            },
                            "center_node_uuid": {
                                "type": "string",
                                "description": "Optional UUID of a node to center the search around"
                            }
                        },
                        "required": ["query"]
                    })
                },
                ToolInfo {
                    name: "clear_graph".to_string(),
                    description: "Clear all data from the graph memory and rebuild indices.".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {},
                        "additionalProperties": false
                    })
                },
            ];

            create_success_response(request.id, json!({ "tools": tool_list }))
        }
        "tools/call" => {
            if let Some(params) = request.params {
                if let Ok(tool_call) = serde_json::from_value::<HashMap<String, Value>>(params) {
                    if let (Some(name), Some(arguments)) =
                        (tool_call.get("name"), tool_call.get("arguments"))
                    {
                        let tools_guard = tools.lock().await;
                        let result = match name.as_str().unwrap_or("") {
                            "add_memory" => tools_guard.add_memory(arguments.clone()).await,
                            "search_memory_nodes" => {
                                tools_guard.search_memory_nodes(arguments.clone()).await
                            }
                            "search_memory_facts" => {
                                tools_guard.search_memory_facts(arguments.clone()).await
                            }
                            "clear_graph" => tools_guard.clear_graph().await,
                            _ => json!({"error": format!("Unknown tool: {}", name)}),
                        };
                        drop(tools_guard);

                        return create_success_response(
                            request.id,
                            json!({
                                "content": [{
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                                }]
                            }),
                        );
                    }
                }
            }
            create_error_response(
                request.id,
                -32602,
                "Invalid tool call parameters".to_string(),
            )
        }
        _ => create_error_response(
            request.id,
            -32601,
            format!("Method not found: {}", request.method),
        ),
    }
}

async fn run_stdio(tools: Arc<Mutex<GraphitiTools>>) -> anyhow::Result<()> {
    info!("Starting MCP server with stdio transport");

    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                match serde_json::from_str::<McpRequest>(trimmed) {
                    Ok(request) => {
                        let response = handle_request(request, tools.clone()).await;
                        let response_json = serde_json::to_string(&response)?;
                        stdout.write_all(response_json.as_bytes()).await?;
                        stdout.write_all(b"\n").await?;
                        stdout.flush().await?;
                    }
                    Err(e) => {
                        warn!("Failed to parse request: {}", e);
                        let error_response =
                            create_error_response(None, -32700, "Parse error".to_string());
                        let response_json = serde_json::to_string(&error_response)?;
                        stdout.write_all(response_json.as_bytes()).await?;
                        stdout.write_all(b"\n").await?;
                        stdout.flush().await?;
                    }
                }
            }
            Err(e) => {
                error!("Error reading from stdin: {}", e);
                break;
            }
        }
    }

    Ok(())
}

async fn run_server(config: GraphitiConfig) -> anyhow::Result<()> {
    info!("Starting Graphiti MCP Server...");

    // Initialize Graphiti
    let graphiti = initialize_graphiti(&config).await?;

    // Create tools container with Graphiti instance
    let tools = Arc::new(Mutex::new(GraphitiTools::new(config.clone(), graphiti)));

    match config.transport {
        ConfigTransport::Stdio => {
            run_stdio(tools).await?;
        }
        ConfigTransport::Sse => {
            error!("SSE transport not yet implemented");
            return Err(anyhow::anyhow!("SSE transport not yet implemented"));
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    // Parse command line arguments
    let args = Args::parse();

    // Check for destroy graph option
    if args.destroy_graph {
        error!("Graph destruction not yet implemented in Rust version");
        return Err(anyhow::anyhow!("Graph destruction not yet implemented"));
    }

    // Create configuration
    let config = GraphitiConfig::from_cli_and_env(args)?;

    info!("Configuration loaded successfully");
    info!("Group ID: {}", config.group_id);
    info!("Transport: {:?}", config.transport);
    info!("Use custom entities: {}", config.use_custom_entities);

    // Run the server
    match run_server(config).await {
        Ok(_) => {
            info!("Server shutdown gracefully");
            Ok(())
        }
        Err(e) => {
            error!("Server error: {}", e);
            Err(e)
        }
    }
}
