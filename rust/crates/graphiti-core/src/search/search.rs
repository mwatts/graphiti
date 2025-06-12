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

use crate::{
    edges::{Edge, EntityEdge},
    errors::GraphitiError,
    nodes::{CommunityNode, EntityNode, EpisodicNode, Node},
    search::{
        utils::*, CommunitySearchMethod, EdgeSearchMethod, EpisodeSearchMethod, NodeSearchMethod,
        SearchConfig, SearchFilters, SearchResult, SearchResults,
    },
    types::GraphitiClients,
};

/// Main search interface for Graphiti
pub struct GraphitiSearch {
    clients: GraphitiClients,
}

impl GraphitiSearch {
    pub fn new(clients: GraphitiClients) -> Self {
        Self { clients }
    }

    /// Perform a comprehensive search across all entity types
    pub async fn search(
        &self,
        query: &str,
        config: &SearchConfig,
        filters: &SearchFilters,
        group_ids: Option<&[String]>,
    ) -> Result<SearchResults, GraphitiError> {
        // Create a cache key for the entire search operation
        let cache_key = format!(
            "search:{}:{}:{:?}:{:?}",
            query,
            serde_json::to_string(config).unwrap_or_default(),
            serde_json::to_string(filters).unwrap_or_default(),
            group_ids
        );

        // Try to get cached results first
        if let Ok(Some(cached_bytes)) = self.clients.cache.get(&cache_key).await {
            if let Ok(cached_results) = serde_json::from_slice::<SearchResults>(&cached_bytes) {
                return Ok(cached_results);
            }
        }

        let mut results = SearchResults::new();

        // Search nodes
        let node_results = self
            .search_nodes(
                query,
                &config.node_search_config.search_methods,
                filters,
                group_ids,
                config.limit,
            )
            .await?;
        results.nodes = node_results;

        // Search edges
        let edge_results = self
            .search_edges(
                query,
                &config.edge_search_config.search_methods,
                filters,
                group_ids,
                config.limit,
            )
            .await?;
        results.edges = edge_results;

        // Search episodes
        let episode_results = self
            .search_episodes(
                query,
                &config.episode_search_config.search_methods,
                filters,
                group_ids,
                config.limit,
            )
            .await?;
        results.episodes = episode_results;

        // Search communities
        let community_results = self
            .search_communities(
                query,
                &config.community_search_config.search_methods,
                filters,
                group_ids,
                config.limit,
            )
            .await?;
        results.communities = community_results;

        // Cache the results for future use
        if let Ok(serialized) = serde_json::to_vec(&results) {
            let _ = self.clients.cache.set(&cache_key, serialized).await;
        }

        Ok(results)
    }

    /// Search for entity nodes
    pub async fn search_nodes(
        &self,
        query: &str,
        search_methods: &[NodeSearchMethod],
        filters: &SearchFilters,
        group_ids: Option<&[String]>,
        limit: usize,
    ) -> Result<Vec<SearchResult<EntityNode>>, GraphitiError> {
        if search_methods.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_results = Vec::new();

        // Get query embedding if needed for similarity search, with cache support
        let query_vector = if search_methods.contains(&NodeSearchMethod::CosimeSimilarity) {
            let cache_key = format!("embedding:{}", query);

            // Try to get from cache first
            if let Ok(Some(cached_bytes)) = self.clients.cache.get(&cache_key).await {
                if let Ok(cached_vector) = serde_json::from_slice::<Vec<f32>>(&cached_bytes) {
                    Some(cached_vector)
                } else {
                    // Cache miss or invalid data, compute new embedding
                    let vector = self.clients.embedder.embed_query(query).await?;

                    // Cache the result
                    if let Ok(serialized) = serde_json::to_vec(&vector) {
                        let _ = self.clients.cache.set(&cache_key, serialized).await;
                    }

                    Some(vector)
                }
            } else {
                // Cache miss, compute new embedding
                let vector = self.clients.embedder.embed_query(query).await?;

                // Cache the result
                if let Ok(serialized) = serde_json::to_vec(&vector) {
                    let _ = self.clients.cache.set(&cache_key, serialized).await;
                }

                Some(vector)
            }
        } else {
            None
        };

        // Execute different search methods
        for method in search_methods {
            match method {
                NodeSearchMethod::CosimeSimilarity => {
                    if let Some(ref vector) = query_vector {
                        let results = node_similarity_search(
                            &self.clients,
                            vector,
                            filters,
                            group_ids,
                            limit * 2, // Get more results for reranking
                        )
                        .await?;
                        all_results.extend(results);
                    }
                }
                NodeSearchMethod::Bm25 => {
                    let results =
                        node_fulltext_search(&self.clients, query, filters, group_ids, limit * 2)
                            .await?;
                    all_results.extend(results);
                }
                NodeSearchMethod::Bfs => {
                    // BFS search requires additional parameters
                    // For now, we'll skip it but could be implemented later
                    continue;
                }
            }
        }

        // Remove duplicates and limit results
        all_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all_results.dedup_by(|a, b| a.item.uuid() == b.item.uuid());
        all_results.truncate(limit);

        Ok(all_results)
    }

    /// Search for entity edges
    pub async fn search_edges(
        &self,
        query: &str,
        search_methods: &[EdgeSearchMethod],
        filters: &SearchFilters,
        group_ids: Option<&[String]>,
        limit: usize,
    ) -> Result<Vec<SearchResult<EntityEdge>>, GraphitiError> {
        if search_methods.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_results = Vec::new();

        // Get query embedding if needed for similarity search
        let query_vector = if search_methods.contains(&EdgeSearchMethod::CosimeSimilarity) {
            Some(self.clients.embedder.embed_query(query).await?)
        } else {
            None
        };

        for method in search_methods {
            let results = match method {
                EdgeSearchMethod::CosimeSimilarity => {
                    if let Some(ref vector) = query_vector {
                        // Convert f32 to f64 for the search function
                        let vector_f64: Vec<f64> = vector.iter().map(|&x| x as f64).collect();
                        edge_similarity_search(
                            &self.clients,
                            &vector_f64,
                            None, // source_node_uuid
                            None, // target_node_uuid
                            filters,
                            group_ids,
                            limit * 2,
                            0.0, // min_score
                        )
                        .await?
                    } else {
                        Vec::new()
                    }
                }
                EdgeSearchMethod::Bm25 => {
                    edge_fulltext_search(&self.clients, query, filters, group_ids, limit * 2)
                        .await?
                }
                EdgeSearchMethod::Bfs => {
                    edge_bfs_search(
                        &self.clients,
                        None, // No origin nodes for general search
                        3,    // max depth
                        filters,
                        limit * 2,
                    )
                    .await?
                }
            };
            all_results.extend(results);
        }

        // Remove duplicates and sort by score
        all_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all_results.dedup_by(|a, b| a.item.uuid() == b.item.uuid());
        all_results.truncate(limit);

        Ok(all_results)
    }

    /// Search for episodic nodes
    pub async fn search_episodes(
        &self,
        query: &str,
        search_methods: &[EpisodeSearchMethod],
        filters: &SearchFilters,
        group_ids: Option<&[String]>,
        limit: usize,
    ) -> Result<Vec<SearchResult<EpisodicNode>>, GraphitiError> {
        if search_methods.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_results = Vec::new();

        for method in search_methods {
            let results = match method {
                EpisodeSearchMethod::Bm25 => {
                    episode_fulltext_search(&self.clients, query, filters, group_ids, limit * 2)
                        .await?
                }
            };
            all_results.extend(results);
        }

        // Remove duplicates and sort by score
        all_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all_results.dedup_by(|a, b| a.item.uuid() == b.item.uuid());
        all_results.truncate(limit);

        Ok(all_results)
    }

    /// Search for community nodes
    pub async fn search_communities(
        &self,
        query: &str,
        search_methods: &[CommunitySearchMethod],
        _filters: &SearchFilters,
        group_ids: Option<&[String]>,
        limit: usize,
    ) -> Result<Vec<SearchResult<CommunityNode>>, GraphitiError> {
        if search_methods.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_results = Vec::new();

        // Get query embedding if needed for similarity search
        let query_vector = if search_methods.contains(&CommunitySearchMethod::CosimeSimilarity) {
            Some(self.clients.embedder.embed_query(query).await?)
        } else {
            None
        };

        for method in search_methods {
            let results = match method {
                CommunitySearchMethod::CosimeSimilarity => {
                    if let Some(ref vector) = query_vector {
                        // Convert f32 to f64 for the search function
                        let vector_f64: Vec<f64> = vector.iter().map(|&x| x as f64).collect();
                        community_similarity_search(&self.clients, &vector_f64, limit * 2).await?
                    } else {
                        Vec::new()
                    }
                }
                CommunitySearchMethod::Bm25 => {
                    community_fulltext_search(&self.clients, query, group_ids, limit * 2).await?
                }
            };
            all_results.extend(results);
        }

        // Remove duplicates and sort by score
        all_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all_results.dedup_by(|a, b| a.item.uuid() == b.item.uuid());
        all_results.truncate(limit);

        Ok(all_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_config_creation() {
        let config = SearchConfig::default();
        assert_eq!(config.limit, 10);
    }

    #[test]
    fn test_search_filters_creation() {
        let filters = SearchFilters::new().with_node_labels(vec!["Entity".to_string()]);
        assert!(filters.node_labels.is_some());
    }
}
