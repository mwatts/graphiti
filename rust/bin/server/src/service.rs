use anyhow::Result;
use graphiti_core::{
    embedder::openai::{OpenAiEmbedder, OpenAiEmbedderConfig},
    llm_client::{openai_client::OpenAiClient, config::LlmConfig},
    cross_encoder::openai_reranker_client::OpenAIRerankerClient,
    nodes::{EpisodeType, EpisodicNode, EntityNode},
    edges::EntityEdge,
    search::{SearchConfig, SearchFilters, SearchResults},
    Graphiti, GraphitiConfig,
};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::config::Settings;

/// Service layer that manages Graphiti instances and provides high-level operations
pub struct GraphitiService {
    graphiti: Graphiti,
}

impl GraphitiService {
    /// Create a new GraphitiService
    pub async fn new(settings: Settings) -> Result<Self> {
        let config = GraphitiConfig {
            database_config: graphiti_core::database::config::DatabaseConfig {
                database_type: graphiti_core::database::config::DatabaseType::Neo4j,
                uri: settings.neo4j_uri.clone(),
                username: Some(settings.neo4j_user.clone()),
                password: Some(settings.neo4j_password.clone()),
                database: None,
                pool_size: Some(10),
                timeout_seconds: Some(30),
                additional_config: std::collections::HashMap::new(),
            },
            store_raw_episode_content: true,
            cache_config: None,
        };

        // Create LLM client
        let llm_config = LlmConfig {
            api_key: Some(settings.openai_api_key.clone()),
            model: settings.model_name.clone(),
            base_url: settings.openai_base_url.clone(),
            temperature: 0.0,
            max_tokens: 8192,
            small_model: None,
        };
        let llm_client = Arc::new(OpenAiClient::new(llm_config, false)
            .map_err(|e| anyhow::anyhow!("Failed to create LLM client: {:?}", e))?);

        // Create embedder
        let embedder_config = OpenAiEmbedderConfig {
            api_key: Some(settings.openai_api_key.clone()),
            embedding_model: settings.embedding_model_name.clone()
                .unwrap_or_else(|| "text-embedding-ada-002".to_string()),
            base_url: settings.openai_base_url.clone(),
            ..Default::default()
        };
        let embedder = Arc::new(OpenAiEmbedder::new(embedder_config)
            .map_err(|e| anyhow::anyhow!("Failed to create embedder: {:?}", e))?);

        // Create cross encoder (reranker)
        let cross_encoder = Arc::new(OpenAIRerankerClient::new(Default::default())
            .map_err(|e| anyhow::anyhow!("Failed to create cross encoder: {:?}", e))?);

        // Create Graphiti instance
        let graphiti = Graphiti::with_clients(config, llm_client, embedder, cross_encoder)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create Graphiti: {:?}", e))?;

        Ok(Self { graphiti })
    }

    /// Add an episode to the graph
    pub async fn add_episode(
        &self,
        name: String,
        content: String,
        source: EpisodeType,
        source_description: String,
        group_id: String,
        reference_time: Option<DateTime<Utc>>,
    ) -> Result<EpisodicNode> {
        let result = self.graphiti.add_episode(
            name,
            content,
            source,
            source_description,
            group_id,
            reference_time,
        ).await
        .map_err(|e| anyhow::anyhow!("Failed to add episode: {:?}", e))?;

        Ok(result.episode)
    }

    /// Search for relevant edges
    pub async fn search(
        &self,
        query: String,
        _group_ids: Option<Vec<String>>, // TODO: Implement group filtering when available in SearchFilters
        num_results: Option<usize>,
    ) -> Result<SearchResults> {
        let filters = SearchFilters::default();
        // TODO: Add group_ids filtering when the field is available
        // if let Some(groups) = group_ids {
        //     filters.group_ids = Some(groups);
        // }

        let mut config = SearchConfig::default();
        if let Some(limit) = num_results {
            config.limit = limit;
        }

        self.graphiti.search(&query, Some(config), Some(filters))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to search: {:?}", e))
    }

    /// Save an entity node (stub - this would need to be implemented in graphiti-core)
    pub async fn save_entity_node(
        &self,
        _uuid: Uuid,
        _group_id: String,
        _name: String,
        _summary: String,
    ) -> Result<EntityNode> {
        // This is a stub implementation since this method doesn't exist in the Rust version yet
        // In the Python version, this creates an EntityNode and saves it directly
        // For now, we'll return an error indicating this needs to be implemented
        Err(anyhow::anyhow!("save_entity_node not yet implemented in Rust version"))
    }

    /// Get an entity edge by UUID (stub)
    pub async fn get_entity_edge(&self, _uuid: Uuid) -> Result<Option<EntityEdge>> {
        // This is a stub implementation since this method doesn't exist in the Rust version yet
        Err(anyhow::anyhow!("get_entity_edge not yet implemented in Rust version"))
    }

    /// Delete an entity edge (stub)
    pub async fn delete_entity_edge(&self, _uuid: Uuid) -> Result<()> {
        // This is a stub implementation since this method doesn't exist in the Rust version yet
        Err(anyhow::anyhow!("delete_entity_edge not yet implemented in Rust version"))
    }

    /// Delete a group (stub)
    pub async fn delete_group(&self, _group_id: String) -> Result<()> {
        // This is a stub implementation since this method doesn't exist in the Rust version yet
        Err(anyhow::anyhow!("delete_group not yet implemented in Rust version"))
    }

    /// Delete an episode (stub)
    pub async fn delete_episode(&self, _uuid: Uuid) -> Result<()> {
        // This is a stub implementation since this method doesn't exist in the Rust version yet
        Err(anyhow::anyhow!("delete_episode not yet implemented in Rust version"))
    }

    /// Retrieve episodes (stub)
    pub async fn retrieve_episodes(
        &self,
        _group_ids: Vec<String>,
        _last_n: usize,
        _reference_time: DateTime<Utc>,
    ) -> Result<Vec<EpisodicNode>> {
        // This is a stub implementation since this method doesn't exist in the Rust version yet
        Err(anyhow::anyhow!("retrieve_episodes not yet implemented in Rust version"))
    }

    /// Clear all data (stub)
    pub async fn clear_data(&self) -> Result<()> {
        // This is a stub implementation since this method doesn't exist in the Rust version yet
        Err(anyhow::anyhow!("clear_data not yet implemented in Rust version"))
    }
}
