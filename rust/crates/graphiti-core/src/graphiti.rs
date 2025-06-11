/*
Copyright 2024, Zep Software, Inc.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

//! Main Graphiti orchestrator - equivalent to Python's graphiti.py

use std::sync::Arc;
use neo4rs::{Graph, ConfigBuilder};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::{
    types::GraphitiClients,
    nodes::{EntityNode, EpisodicNode, BaseNode, EpisodeType},
    edges::EntityEdge,
    llm_client::{LlmClient, openai_client::OpenAiClient},
    embedder::{EmbedderClient, OpenAiEmbedder},
    cross_encoder::{CrossEncoderClient, OpenAIRerankerClient},
    cache::{Cache, CacheConfig},
    search::{SearchConfig, SearchResults, SearchFilters, GraphitiSearch},
    utils::{
        bulk_utils::{
            RawEpisode, add_nodes_and_edges_bulk, dedupe_edges_bulk, dedupe_nodes_bulk,
            extract_edge_dates_bulk, extract_nodes_and_edges_bulk, resolve_edge_pointers,
            retrieve_previous_episodes_bulk,
        },
        datetime_utils::utc_now,
        maintenance::{
            EPISODE_WINDOW_LEN,
            // build_indices_and_constraints,
            retrieve_episodes,
        },
    },
    errors::GraphitiError,
};

/// Results from adding an episode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddEpisodeResults {
    pub episode: EpisodicNode,
    pub nodes: Vec<EntityNode>,
    pub edges: Vec<EntityEdge>,
}

/// Configuration for Graphiti
#[derive(Debug, Clone)]
pub struct GraphitiConfig {
    pub neo4j_uri: String,
    pub neo4j_user: String,
    pub neo4j_password: String,
    pub database: Option<String>,
    pub store_raw_episode_content: bool,
    pub cache_config: Option<CacheConfig>,
}

impl Default for GraphitiConfig {
    fn default() -> Self {
        Self {
            neo4j_uri: "bolt://localhost:7687".to_string(),
            neo4j_user: "neo4j".to_string(),
            neo4j_password: "password".to_string(),
            database: None,
            store_raw_episode_content: true,
            cache_config: Some(CacheConfig::default()),
        }
    }
}

/// Main Graphiti orchestrator for temporal graph operations
pub struct Graphiti {
    clients: GraphitiClients,
    database: Option<String>,
    store_raw_episode_content: bool,
}

impl Graphiti {
    /// Initialize a new Graphiti instance
    pub async fn new(config: GraphitiConfig) -> Result<Self, GraphitiError> {
        // Initialize Neo4j connection
        let graph_config = ConfigBuilder::default()
            .uri(&config.neo4j_uri)
            .user(&config.neo4j_user)
            .password(&config.neo4j_password)
            .db(config.database.as_deref().unwrap_or("neo4j"))
            .build()?;

        let driver = Graph::connect(graph_config).await?;

        // Initialize cache if configured
        let cache: Option<Arc<dyn Cache>> = if let Some(cache_config) = config.cache_config {
            Some(cache_config.build()?)
        } else {
            None
        };

        // Initialize default clients
        let llm_client: Arc<dyn LlmClient> = Arc::new(OpenAiClient::new(Default::default())?);
        let embedder: Arc<dyn EmbedderClient> = Arc::new(OpenAiEmbedder::new(Default::default())?);
        let cross_encoder: Arc<dyn CrossEncoderClient> = Arc::new(OpenAIRerankerClient::new(Default::default())?);

        // Wrap with cache if available
        let cached_llm_client = if let Some(cache) = &cache {
            crate::llm_client::CachedLlmClient::new(llm_client, cache.clone())
        } else {
            crate::llm_client::CachedLlmClient::new(llm_client, Arc::new(crate::cache::memory_cache::MemoryCache::new()))
        };

        let cached_embedder = if let Some(cache) = &cache {
            crate::embedder::CachedEmbedderClient::new(embedder, cache.clone())
        } else {
            crate::embedder::CachedEmbedderClient::new(embedder, Arc::new(crate::cache::memory_cache::MemoryCache::new()))
        };

        let clients = GraphitiClients {
            driver,
            llm_client: Arc::new(cached_llm_client),
            embedder: Arc::new(cached_embedder),
            cross_encoder,
            cache,
        };

        Ok(Self {
            clients,
            database: config.database,
            store_raw_episode_content: config.store_raw_episode_content,
        })
    }

    /// Initialize with custom clients
    pub async fn with_clients(
        config: GraphitiConfig,
        llm_client: Arc<dyn LlmClient>,
        embedder: Arc<dyn EmbedderClient>,
        cross_encoder: Arc<dyn CrossEncoderClient>,
    ) -> Result<Self, GraphitiError> {
        // Initialize Neo4j connection
        let graph_config = ConfigBuilder::default()
            .uri(&config.neo4j_uri)
            .user(&config.neo4j_user)
            .password(&config.neo4j_password)
            .db(config.database.as_deref().unwrap_or("neo4j"))
            .build()?;

        let driver = Graph::connect(graph_config).await?;

        // Initialize cache if configured
        let cache: Option<Arc<dyn Cache>> = if let Some(cache_config) = config.cache_config {
            Some(cache_config.build()?)
        } else {
            None
        };

        let clients = GraphitiClients {
            driver,
            llm_client,
            embedder,
            cross_encoder,
            cache,
        };

        Ok(Self {
            clients,
            database: config.database,
            store_raw_episode_content: config.store_raw_episode_content,
        })
    }

    /// Close the database connections
    pub async fn close(&self) -> Result<(), GraphitiError> {
        // Neo4j driver cleanup would happen here
        // The Graph type should handle this automatically on drop
        Ok(())
    }

    /// Build database indices and constraints
    pub async fn build_indices_and_constraints(&self, delete_existing: bool) -> Result<(), GraphitiError> {
        // Stub implementation - would create database indices and constraints
        let _delete_existing = delete_existing; // Use parameter to avoid warning

        // TODO: Implement index and constraint creation
        // This would involve running Cypher queries to create:
        // - Unique constraints on node UUIDs
        // - Indices on commonly queried fields
        // - Vector indices for embeddings

        Ok(())
    }

    /// Add a single episode to the graph
    pub async fn add_episode(
        &self,
        name: String,
        content: String,
        source: EpisodeType,
        source_description: String,
        group_id: String,
        reference_time: Option<DateTime<Utc>>,
    ) -> Result<AddEpisodeResults, GraphitiError> {
        let reference_time = reference_time.unwrap_or_else(utc_now);

        // Create the episodic node
        let base_node = BaseNode::new(name, group_id.clone())
            .with_created_at(reference_time);

        let episode = EpisodicNode {
            base: base_node,
            source,
            source_description,
            content: if self.store_raw_episode_content { content.clone() } else { String::new() },
            valid_at: reference_time,
            entity_edges: Vec::new(),
        };

        // Get previous episodes for context
        let previous_episodes = retrieve_episodes(
            &self.clients.driver,
            reference_time,
            EPISODE_WINDOW_LEN,
            &[group_id],
        ).await?;

        // Extract nodes and edges
        let (mut nodes, mut edges, episodic_edges) = extract_nodes_and_edges_bulk(
            &self.clients,
            vec![(episode.clone(), previous_episodes)],
        ).await?;

        // Deduplicate nodes
        let (deduped_nodes, uuid_map) = dedupe_nodes_bulk(
            &self.clients.driver,
            self.clients.llm_client.as_ref(),
            nodes,
        ).await?;
        nodes = deduped_nodes;

        // Update edge pointers with deduplicated node UUIDs
        resolve_edge_pointers(&mut edges, &uuid_map);

        // Deduplicate edges
        edges = dedupe_edges_bulk(
            &self.clients.driver,
            self.clients.llm_client.as_ref(),
            edges,
        ).await?;

        // Extract temporal information for edges
        edges = extract_edge_dates_bulk(
            self.clients.llm_client.as_ref(),
            edges,
            vec![(episode.clone(), Vec::new())], // Previous episodes for this single episode
        ).await?;

        // Save to database
        add_nodes_and_edges_bulk(
            &self.clients.driver,
            vec![episode.clone()],
            episodic_edges,
            nodes.clone(),
            edges.clone(),
            self.clients.embedder.as_ref(),
        ).await?;

        Ok(AddEpisodeResults {
            episode,
            nodes,
            edges,
        })
    }

    /// Add multiple episodes in bulk
    pub async fn add_episodes_bulk(
        &self,
        raw_episodes: Vec<RawEpisode>,
    ) -> Result<Vec<AddEpisodeResults>, GraphitiError> {
        // Convert raw episodes to episodic nodes
        let episodes: Vec<EpisodicNode> = raw_episodes
            .into_iter()
            .map(|raw| {
                let base_node = BaseNode::new(raw.name, "default".to_string()) // TODO: Handle group_id properly
                    .with_created_at(raw.reference_time);

                EpisodicNode {
                    base: base_node,
                    source: raw.source,
                    source_description: raw.source_description,
                    content: if self.store_raw_episode_content { raw.content } else { String::new() },
                    valid_at: raw.reference_time,
                    entity_edges: Vec::new(),
                }
            })
            .collect();

        // Get previous episodes for each episode
        let episode_tuples = retrieve_previous_episodes_bulk(
            &self.clients.driver,
            &episodes,
            EPISODE_WINDOW_LEN,
        ).await?;

        // Extract nodes and edges in bulk
        let (mut nodes, mut edges, episodic_edges) = extract_nodes_and_edges_bulk(
            &self.clients,
            episode_tuples.clone(),
        ).await?;

        // Deduplicate nodes
        let (deduped_nodes, uuid_map) = dedupe_nodes_bulk(
            &self.clients.driver,
            self.clients.llm_client.as_ref(),
            nodes,
        ).await?;
        nodes = deduped_nodes;

        // Update edge pointers
        resolve_edge_pointers(&mut edges, &uuid_map);

        // Deduplicate edges
        edges = dedupe_edges_bulk(
            &self.clients.driver,
            self.clients.llm_client.as_ref(),
            edges,
        ).await?;

        // Extract temporal information
        edges = extract_edge_dates_bulk(
            self.clients.llm_client.as_ref(),
            edges,
            episode_tuples.clone(),
        ).await?;

        // Save to database
        add_nodes_and_edges_bulk(
            &self.clients.driver,
            episodes.clone(),
            episodic_edges,
            nodes.clone(),
            edges.clone(),
            self.clients.embedder.as_ref(),
        ).await?;

        // Return results for each episode
        let results: Vec<AddEpisodeResults> = episodes
            .into_iter()
            .map(|episode| AddEpisodeResults {
                episode,
                nodes: nodes.clone(), // For simplicity, returning all nodes for each episode
                edges: edges.clone(), // For simplicity, returning all edges for each episode
            })
            .collect();

        Ok(results)
    }

    /// Search the graph
    pub async fn search(
        &self,
        query: &str,
        config: Option<SearchConfig>,
        filters: Option<SearchFilters>,
    ) -> Result<SearchResults, GraphitiError> {
        let search = GraphitiSearch::new(&self.clients);
        search.search(query, config, filters).await
    }

    /// Get access to the clients for advanced operations
    pub fn clients(&self) -> &GraphitiClients {
        &self.clients
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graphiti_config_default() {
        let config = GraphitiConfig::default();
        assert_eq!(config.neo4j_uri, "bolt://localhost:7687");
        assert_eq!(config.neo4j_user, "neo4j");
        assert_eq!(config.neo4j_password, "password");
        assert!(config.store_raw_episode_content);
    }

    #[tokio::test]
    #[ignore] // Requires database connection
    async fn test_graphiti_initialization() {
        // This test requires a Neo4j database to be running
        // and proper credentials to be configured
    }
}
