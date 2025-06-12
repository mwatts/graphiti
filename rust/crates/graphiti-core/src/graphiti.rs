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

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    cache::{Cache, CacheConfig},
    cross_encoder::{CrossEncoderClient, OpenAIRerankerClient},
    database::{create_database, DatabaseConfig, GraphDatabase},
    edges::{EntityEdge, EpisodicEdge},
    embedder::{EmbedderClient, OpenAiEmbedder},
    errors::GraphitiError,
    llm_client::{openai_client::OpenAiClient, LlmClient},
    nodes::{BaseNode, EntityNode, EpisodeType, EpisodicNode},
    search::{GraphitiSearch, SearchConfig, SearchFilters, SearchResults},
    types::GraphitiClients,
    utils::{bulk_utils::RawEpisode, datetime_utils::utc_now},
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
    pub database_config: DatabaseConfig,
    pub store_raw_episode_content: bool,
    pub cache_config: Option<CacheConfig>,
}

impl Default for GraphitiConfig {
    fn default() -> Self {
        Self {
            database_config: DatabaseConfig::default(),
            store_raw_episode_content: true,
            cache_config: Some(CacheConfig::default()),
        }
    }
}

/// Main Graphiti orchestrator for temporal graph operations
pub struct Graphiti {
    clients: GraphitiClients,
    database: Arc<dyn GraphDatabase + Send + Sync>,
    store_raw_episode_content: bool,
}

impl Graphiti {
    /// Initialize a new Graphiti instance
    pub async fn new(config: GraphitiConfig) -> Result<Self, GraphitiError> {
        // Initialize database using the abstraction layer
        let database = create_database(config.database_config).await?;

        // Initialize cache if configured
        let cache: Option<Arc<dyn Cache + Send + Sync>> =
            if let Some(cache_config) = config.cache_config {
                Some(Arc::new(crate::cache::memory_cache::MemoryCache::new(
                    cache_config,
                )))
            } else {
                None
            };

        // Initialize default clients
        let llm_client: Arc<dyn LlmClient> =
            Arc::new(OpenAiClient::new(Default::default(), false)?);
        let embedder: Arc<dyn EmbedderClient> = Arc::new(OpenAiEmbedder::new(Default::default())?);
        let cross_encoder: Arc<dyn CrossEncoderClient> =
            Arc::new(OpenAIRerankerClient::new(Default::default())?);

        // Wrap with cache if available
        let cached_llm_client = if let Some(cache) = &cache {
            crate::llm_client::CachedLlmClient::new(llm_client, cache.clone())
        } else {
            crate::llm_client::CachedLlmClient::new(
                llm_client,
                Arc::new(crate::cache::memory_cache::MemoryCache::new(
                    CacheConfig::default(),
                )),
            )
        };

        let cached_embedder = if let Some(cache) = &cache {
            crate::embedder::CachedEmbedderClient::new(embedder, cache.clone())
        } else {
            crate::embedder::CachedEmbedderClient::new(
                embedder,
                Arc::new(crate::cache::memory_cache::MemoryCache::new(
                    CacheConfig::default(),
                )),
            )
        };

        let database_arc: Arc<dyn GraphDatabase + Send + Sync> = Arc::from(database);

        let clients = GraphitiClients {
            database: database_arc.clone(),
            llm_client: Arc::new(cached_llm_client),
            embedder: Arc::new(cached_embedder),
            cross_encoder,
            cache: cache.unwrap_or_else(|| {
                Arc::new(crate::cache::memory_cache::MemoryCache::new(
                    CacheConfig::default(),
                ))
            }),
        };

        Ok(Self {
            clients,
            database: database_arc,
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
        // Initialize database using the abstraction layer
        let database = create_database(config.database_config).await?;

        // Initialize cache if configured
        let cache: Option<Arc<dyn Cache + Send + Sync>> =
            if let Some(cache_config) = config.cache_config {
                Some(Arc::new(crate::cache::memory_cache::MemoryCache::new(
                    cache_config,
                )))
            } else {
                None
            };

        let database_arc = database;

        let clients = GraphitiClients {
            database: database_arc.clone(),
            llm_client,
            embedder,
            cross_encoder,
            cache: cache.unwrap_or_else(|| {
                Arc::new(crate::cache::memory_cache::MemoryCache::new(
                    CacheConfig::default(),
                ))
            }),
        };

        Ok(Self {
            clients,
            database: database_arc,
            store_raw_episode_content: config.store_raw_episode_content,
        })
    }

    /// Close the database connections
    pub async fn close(&self) -> Result<(), GraphitiError> {
        self.database.close().await?;
        Ok(())
    }

    /// Build database indices and constraints
    pub async fn build_indices_and_constraints(
        &self,
        delete_existing: bool,
    ) -> Result<(), GraphitiError> {
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
        let base_node = BaseNode::new(name, group_id.clone()).with_created_at(reference_time);

        let episode = EpisodicNode {
            base: base_node,
            source,
            source_description,
            content: if self.store_raw_episode_content {
                content.clone()
            } else {
                String::new()
            },
            valid_at: reference_time,
            entity_edges: Vec::new(),
        };

        // Get previous episodes for context (temporarily disabled)
        let _previous_episodes: Vec<EpisodicNode> = Vec::new(); // TODO: Implement using database abstraction
                                                                // retrieve_episodes(
                                                                //     &self.clients.database,
                                                                //     reference_time,
                                                                //     EPISODE_WINDOW_LEN,
                                                                //     &[group_id],
                                                                // ).await?;

        // Extract nodes and edges (temporarily disabled)
        let nodes: Vec<EntityNode> = Vec::new(); // TODO: Implement using database abstraction
        let edges: Vec<EntityEdge> = Vec::new(); // TODO: Implement using database abstraction
        let _episodic_edges: Vec<EpisodicEdge> = Vec::new(); // TODO: Implement using database abstraction
                                                             // let (mut nodes, mut edges, episodic_edges) = extract_nodes_and_edges_bulk(
                                                             //     &self.clients,
                                                             //     vec![(episode.clone(), previous_episodes)],
                                                             // ).await?;

        // Deduplicate nodes (temporarily disabled)
        // let (deduped_nodes, uuid_map) = dedupe_nodes_bulk(
        //     &self.clients.database,
        //     self.clients.llm_client.as_ref(),
        //     nodes,
        // ).await?;
        // nodes = deduped_nodes;

        // Update edge pointers with deduplicated node UUIDs (temporarily disabled)
        // resolve_edge_pointers(&mut edges, &uuid_map);

        // Deduplicate edges (temporarily disabled)
        // edges = dedupe_edges_bulk(
        //     &self.clients.database,
        //     self.clients.llm_client.as_ref(),
        //     edges,
        // ).await?;

        // Extract temporal information for edges (temporarily disabled)
        // edges = extract_edge_dates_bulk(
        //     self.clients.llm_client.as_ref(),
        //     edges,
        //     vec![(episode.clone(), Vec::new())], // Previous episodes for this single episode
        // ).await?;

        // Save to database (temporarily disabled)
        // add_nodes_and_edges_bulk(
        //     &self.clients.database,
        //     vec![episode.clone()],
        //     episodic_edges,
        //     nodes.clone(),
        //     edges.clone(),
        //     self.clients.embedder.as_ref(),
        // ).await?;

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
                    content: if self.store_raw_episode_content {
                        raw.content
                    } else {
                        String::new()
                    },
                    valid_at: raw.reference_time,
                    entity_edges: Vec::new(),
                }
            })
            .collect();

        // Get previous episodes for each episode (temporarily disabled)
        let _episode_tuples: Vec<(EpisodicNode, Vec<EpisodicNode>)> = Vec::new(); // TODO: Implement using database abstraction
                                                                                  // retrieve_previous_episodes_bulk(
                                                                                  //     &self.clients.database,
                                                                                  //     &episodes,
                                                                                  //     EPISODE_WINDOW_LEN,
                                                                                  // ).await?;

        // Extract nodes and edges in bulk (temporarily disabled)
        let nodes: Vec<EntityNode> = Vec::new(); // TODO: Implement using database abstraction
        let edges: Vec<EntityEdge> = Vec::new(); // TODO: Implement using database abstraction
        let _episodic_edges: Vec<EpisodicEdge> = Vec::new(); // TODO: Implement using database abstraction
                                                             // let (mut nodes, mut edges, episodic_edges) = extract_nodes_and_edges_bulk(
                                                             //     &self.clients,
                                                             //     episode_tuples.clone(),
                                                             // ).await?;

        // Deduplicate nodes (temporarily disabled)
        // let (deduped_nodes, uuid_map) = dedupe_nodes_bulk(
        //     &self.clients.database,
        //     self.clients.llm_client.as_ref(),
        //     nodes,
        // ).await?;
        // nodes = deduped_nodes;

        // Update edge pointers (temporarily disabled)
        // resolve_edge_pointers(&mut edges, &uuid_map);

        // Deduplicate edges (temporarily disabled)
        // edges = dedupe_edges_bulk(
        //     &self.clients.database,
        //     self.clients.llm_client.as_ref(),
        //     edges,
        // ).await?;

        // Extract temporal information (temporarily disabled)
        // edges = extract_edge_dates_bulk(
        //     self.clients.llm_client.as_ref(),
        //     edges,
        //     episode_tuples.clone(),
        // ).await?;

        // Save to database (temporarily disabled)
        // add_nodes_and_edges_bulk(
        //     &self.clients.database,
        //     episodes.clone(),
        //     episodic_edges,
        //     nodes.clone(),
        //     edges.clone(),
        //     self.clients.embedder.as_ref(),
        // ).await?;

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
        let search = GraphitiSearch::new(self.clients.clone());
        let default_config = SearchConfig::default();
        let default_filters = SearchFilters::default();
        search
            .search(
                query,
                config.as_ref().unwrap_or(&default_config),
                filters.as_ref().unwrap_or(&default_filters),
                None,
            )
            .await
    }

    /// Get access to the clients for advanced operations
    pub fn clients(&self) -> &GraphitiClients {
        &self.clients
    }

    /// Get the underlying database driver for backward compatibility
    /// This is temporary until all utilities are migrated to use the database abstraction
    pub fn get_neo4j_driver(&self) -> Option<&neo4rs::Graph> {
        if let Some(neo4j_db) = self
            .database
            .as_any()
            .downcast_ref::<crate::database::neo4j::Neo4jDatabase>()
        {
            Some(neo4j_db.get_graph())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graphiti_config_default() {
        let config = GraphitiConfig::default();
        assert_eq!(config.database_config.uri, "bolt://localhost:7687");
        assert_eq!(config.database_config.username, Some("neo4j".to_string()));
        assert_eq!(
            config.database_config.password,
            Some("password".to_string())
        );
        assert!(config.store_raw_episode_content);
    }

    #[tokio::test]
    #[ignore] // Requires database connection
    async fn test_graphiti_initialization() {
        // This test requires a Neo4j database to be running
        // and proper credentials to be configured
    }
}
