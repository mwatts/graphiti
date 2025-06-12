use serde::{Deserialize, Serialize};
use std::env;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// OpenAI API key
    pub openai_api_key: String,

    /// OpenAI base URL (optional)
    pub openai_base_url: Option<String>,

    /// Model name for LLM operations
    pub model_name: Option<String>,

    /// Embedding model name
    pub embedding_model_name: Option<String>,

    /// Neo4j database URI
    pub neo4j_uri: String,

    /// Neo4j username
    pub neo4j_user: String,

    /// Neo4j password
    pub neo4j_password: String,

    /// Server host
    #[serde(default = "default_host")]
    pub host: String,

    /// Server port
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8000
}

impl Settings {
    /// Load settings from environment variables
    pub fn load() -> Result<Self, anyhow::Error> {
        dotenvy::dotenv().ok(); // Load .env file if it exists

        let settings = Settings {
            openai_api_key: env::var("OPENAI_API_KEY")
                .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY environment variable is required"))?,
            openai_base_url: env::var("OPENAI_BASE_URL").ok(),
            model_name: env::var("MODEL_NAME").ok(),
            embedding_model_name: env::var("EMBEDDING_MODEL_NAME").ok(),
            neo4j_uri: env::var("NEO4J_URI")
                .map_err(|_| anyhow::anyhow!("NEO4J_URI environment variable is required"))?,
            neo4j_user: env::var("NEO4J_USER")
                .map_err(|_| anyhow::anyhow!("NEO4J_USER environment variable is required"))?,
            neo4j_password: env::var("NEO4J_PASSWORD")
                .map_err(|_| anyhow::anyhow!("NEO4J_PASSWORD environment variable is required"))?,
            host: env::var("HOST").unwrap_or_else(|_| default_host()),
            port: env::var("PORT")
                .map(|p| p.parse().unwrap_or(default_port()))
                .unwrap_or(default_port()),
        };

        Ok(settings)
    }

    /// Get the server address as a string
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
