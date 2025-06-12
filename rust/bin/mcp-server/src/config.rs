use clap::Parser;
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;

/// Model Context Protocol transport type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Transport {
    Stdio,
    Sse,
}

impl std::str::FromStr for Transport {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "stdio" => Ok(Transport::Stdio),
            "sse" => Ok(Transport::Sse),
            _ => Err(format!("Invalid transport: {}", s)),
        }
    }
}

/// Command line arguments
#[derive(Parser, Debug)]
#[clap(name = "graphiti-mcp-server")]
#[clap(about = "Graphiti Model Context Protocol Server")]
pub struct Args {
    /// Group ID for organizing related data
    #[clap(long, help = "Namespace for the graph")]
    pub group_id: Option<String>,

    /// Transport protocol to use
    #[clap(long, default_value = "sse", help = "Transport to use (stdio or sse)")]
    pub transport: Transport,

    /// LLM model name
    #[clap(long, help = "Model name for LLM operations")]
    pub model: Option<String>,

    /// Small LLM model name
    #[clap(long, help = "Small model name for LLM operations")]
    pub small_model: Option<String>,

    /// Temperature setting for LLM
    #[clap(long, help = "Temperature for LLM (0.0-2.0)")]
    pub temperature: Option<f32>,

    /// Destroy all graphs
    #[clap(long, help = "Destroy all Graphiti graphs")]
    pub destroy_graph: bool,

    /// Enable custom entity extraction
    #[clap(long, help = "Enable entity extraction using predefined types")]
    pub use_custom_entities: bool,
}

/// Graphiti LLM Configuration
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub api_key: Option<String>,
    pub model: String,
    pub small_model: String,
    pub temperature: f32,
    pub azure_openai_endpoint: Option<String>,
    pub azure_openai_deployment_name: Option<String>,
    pub azure_openai_api_version: Option<String>,
    pub azure_openai_use_managed_identity: bool,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: "gpt-4o-mini".to_string(),
            small_model: "gpt-4o-mini".to_string(),
            temperature: 0.0,
            azure_openai_endpoint: None,
            azure_openai_deployment_name: None,
            azure_openai_api_version: None,
            azure_openai_use_managed_identity: false,
        }
    }
}

impl LlmConfig {
    /// Create LLM configuration from environment variables
    pub fn from_env() -> Self {
        let model = env::var("MODEL_NAME").unwrap_or_else(|_| "gpt-4o-mini".to_string());

        let small_model =
            env::var("SMALL_MODEL_NAME").unwrap_or_else(|_| "gpt-4o-mini".to_string());

        let azure_endpoint = env::var("AZURE_OPENAI_ENDPOINT").ok();
        let azure_use_managed_identity = env::var("AZURE_OPENAI_USE_MANAGED_IDENTITY")
            .unwrap_or_else(|_| "false".to_string())
            .to_lowercase()
            == "true";

        Self {
            api_key: env::var("OPENAI_API_KEY").ok(),
            model,
            small_model,
            temperature: env::var("LLM_TEMPERATURE")
                .unwrap_or_else(|_| "0.0".to_string())
                .parse()
                .unwrap_or(0.0),
            azure_openai_endpoint: azure_endpoint,
            azure_openai_deployment_name: env::var("AZURE_OPENAI_DEPLOYMENT_NAME").ok(),
            azure_openai_api_version: env::var("AZURE_OPENAI_API_VERSION").ok(),
            azure_openai_use_managed_identity: azure_use_managed_identity,
        }
    }

    /// Create LLM config from CLI args and environment
    pub fn from_cli_and_env(args: &Args) -> Self {
        let mut config = Self::from_env();

        // CLI overrides environment
        if let Some(model) = &args.model {
            if !model.trim().is_empty() {
                config.model = model.clone();
            }
        }

        if let Some(small_model) = &args.small_model {
            if !small_model.trim().is_empty() {
                config.small_model = small_model.clone();
            }
        }

        if let Some(temperature) = args.temperature {
            config.temperature = temperature;
        }

        config
    }
}

/// Graphiti Embedder Configuration
#[derive(Debug, Clone)]
pub struct EmbedderConfig {
    pub api_key: Option<String>,
    pub model: String,
    pub azure_openai_endpoint: Option<String>,
    pub azure_openai_deployment_name: Option<String>,
    pub azure_openai_api_version: Option<String>,
    pub azure_openai_use_managed_identity: bool,
}

impl Default for EmbedderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: "text-embedding-3-small".to_string(),
            azure_openai_endpoint: None,
            azure_openai_deployment_name: None,
            azure_openai_api_version: None,
            azure_openai_use_managed_identity: false,
        }
    }
}

impl EmbedderConfig {
    pub fn from_env() -> Self {
        let model = env::var("EMBEDDER_MODEL_NAME")
            .unwrap_or_else(|_| "text-embedding-3-small".to_string());

        Self {
            api_key: env::var("OPENAI_API_KEY").ok(),
            model,
            azure_openai_endpoint: env::var("AZURE_OPENAI_ENDPOINT").ok(),
            azure_openai_deployment_name: env::var("AZURE_OPENAI_DEPLOYMENT_NAME").ok(),
            azure_openai_api_version: env::var("AZURE_OPENAI_API_VERSION").ok(),
            azure_openai_use_managed_identity: env::var("AZURE_OPENAI_USE_MANAGED_IDENTITY")
                .unwrap_or_else(|_| "false".to_string())
                .to_lowercase()
                == "true",
        }
    }
}

/// Neo4j Configuration
#[derive(Debug, Clone)]
pub struct Neo4jConfig {
    pub uri: String,
    pub user: String,
    pub password: String,
    pub database: Option<String>,
}

impl Default for Neo4jConfig {
    fn default() -> Self {
        Self {
            uri: "bolt://localhost:7687".to_string(),
            user: "neo4j".to_string(),
            password: "password".to_string(),
            database: None,
        }
    }
}

impl Neo4jConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            uri: env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string()),
            user: env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string()),
            password: env::var("NEO4J_PASSWORD")
                .map_err(|_| anyhow::anyhow!("NEO4J_PASSWORD environment variable is required"))?,
            database: env::var("NEO4J_DATABASE").ok(),
        })
    }
}

/// Main Graphiti Configuration
#[derive(Debug, Clone)]
pub struct GraphitiConfig {
    pub group_id: String,
    pub transport: Transport,
    pub use_custom_entities: bool,
    pub destroy_graph: bool,
    pub llm: LlmConfig,
    pub embedder: EmbedderConfig,
    pub neo4j: Neo4jConfig,
}

impl GraphitiConfig {
    /// Create configuration from CLI arguments and environment
    pub fn from_cli_and_env(mut args: Args) -> anyhow::Result<Self> {
        let group_id = args
            .group_id
            .take()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let transport = args.transport;
        let use_custom_entities = args.use_custom_entities;
        let destroy_graph = args.destroy_graph;
        let llm_config = LlmConfig::from_cli_and_env(&args);

        Ok(Self {
            group_id,
            transport,
            use_custom_entities,
            destroy_graph,
            llm: llm_config,
            embedder: EmbedderConfig::from_env(),
            neo4j: Neo4jConfig::from_env()?,
        })
    }
}
