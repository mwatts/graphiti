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

use serde::{Deserialize, Serialize};

use crate::{edges::EntityEdge, nodes::{CommunityNode, EntityNode, EpisodicNode}};

pub const DEFAULT_SEARCH_LIMIT: usize = 10;
pub const DEFAULT_MIN_SCORE: f64 = 0.0;
pub const DEFAULT_MMR_LAMBDA: f64 = 0.5;
pub const MAX_SEARCH_DEPTH: i32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EdgeSearchMethod {
    CosimeSimilarity,
    Bm25,
    Bfs,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeSearchMethod {
    CosimeSimilarity,
    Bm25,
    Bfs,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EpisodeSearchMethod {
    Bm25,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommunitySearchMethod {
    CosimeSimilarity,
    Bm25,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeReranker {
    Rrf,
    NodeDistance,
    EpisodeMentions,
    Mmr,
    CrossEncoder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeReranker {
    Rrf,
    NodeDistance,
    EpisodeMentions,
    Mmr,
    CrossEncoder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EpisodeReranker {
    Rrf,
    CrossEncoder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommunityReranker {
    Rrf,
    Mmr,
    CrossEncoder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeSearchConfig {
    pub search_methods: Vec<EdgeSearchMethod>,
    pub reranker: EdgeReranker,
    pub sim_min_score: f64,
    pub mmr_lambda: f64,
    pub bfs_max_depth: i32,
}

impl Default for EdgeSearchConfig {
    fn default() -> Self {
        Self {
            search_methods: vec![EdgeSearchMethod::CosimeSimilarity],
            reranker: EdgeReranker::Rrf,
            sim_min_score: DEFAULT_MIN_SCORE,
            mmr_lambda: DEFAULT_MMR_LAMBDA,
            bfs_max_depth: MAX_SEARCH_DEPTH,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSearchConfig {
    pub search_methods: Vec<NodeSearchMethod>,
    pub reranker: NodeReranker,
    pub sim_min_score: f64,
    pub mmr_lambda: f64,
    pub bfs_max_depth: i32,
}

impl Default for NodeSearchConfig {
    fn default() -> Self {
        Self {
            search_methods: vec![NodeSearchMethod::CosimeSimilarity],
            reranker: NodeReranker::Rrf,
            sim_min_score: DEFAULT_MIN_SCORE,
            mmr_lambda: DEFAULT_MMR_LAMBDA,
            bfs_max_depth: MAX_SEARCH_DEPTH,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeSearchConfig {
    pub search_methods: Vec<EpisodeSearchMethod>,
    pub reranker: EpisodeReranker,
    pub sim_min_score: f64,
    pub mmr_lambda: f64,
}

impl Default for EpisodeSearchConfig {
    fn default() -> Self {
        Self {
            search_methods: vec![EpisodeSearchMethod::Bm25],
            reranker: EpisodeReranker::Rrf,
            sim_min_score: DEFAULT_MIN_SCORE,
            mmr_lambda: DEFAULT_MMR_LAMBDA,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunitySearchConfig {
    pub search_methods: Vec<CommunitySearchMethod>,
    pub reranker: CommunityReranker,
    pub sim_min_score: f64,
    pub mmr_lambda: f64,
}

impl Default for CommunitySearchConfig {
    fn default() -> Self {
        Self {
            search_methods: vec![CommunitySearchMethod::CosimeSimilarity],
            reranker: CommunityReranker::Rrf,
            sim_min_score: DEFAULT_MIN_SCORE,
            mmr_lambda: DEFAULT_MMR_LAMBDA,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub node_search_config: NodeSearchConfig,
    pub edge_search_config: EdgeSearchConfig,
    pub episode_search_config: EpisodeSearchConfig,
    pub community_search_config: CommunitySearchConfig,
    pub limit: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            node_search_config: NodeSearchConfig::default(),
            edge_search_config: EdgeSearchConfig::default(),
            episode_search_config: EpisodeSearchConfig::default(),
            community_search_config: CommunitySearchConfig::default(),
            limit: DEFAULT_SEARCH_LIMIT,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult<T> {
    pub item: T,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub nodes: Vec<SearchResult<EntityNode>>,
    pub edges: Vec<SearchResult<EntityEdge>>,
    pub episodes: Vec<SearchResult<EpisodicNode>>,
    pub communities: Vec<SearchResult<CommunityNode>>,
}

impl SearchResults {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            episodes: Vec::new(),
            communities: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.edges.is_empty() && self.episodes.is_empty() && self.communities.is_empty()
    }
}

impl Default for SearchResults {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_config_default() {
        let config = SearchConfig::default();
        assert_eq!(config.limit, DEFAULT_SEARCH_LIMIT);
        assert_eq!(config.node_search_config.sim_min_score, DEFAULT_MIN_SCORE);
        assert_eq!(config.edge_search_config.mmr_lambda, DEFAULT_MMR_LAMBDA);
    }

    #[test]
    fn test_search_results_empty() {
        let results = SearchResults::new();
        assert!(results.is_empty());
    }
}
