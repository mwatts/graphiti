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

//! Search functionality for Graphiti
//!
//! This module provides search capabilities including:
//! - Vector similarity search
//! - Full-text search (BM25)
//! - Breadth-first search
//! - Hybrid search with reranking
//! - Search configuration and filters

pub mod config;
pub mod filters;
pub mod search;
pub mod utils;

#[cfg(test)]
mod tests;

pub use config::{
    SearchConfig, SearchResults, SearchResult,
    EdgeSearchConfig, NodeSearchConfig, EpisodeSearchConfig, CommunitySearchConfig,
    EdgeSearchMethod, NodeSearchMethod, EpisodeSearchMethod, CommunitySearchMethod,
    EdgeReranker, NodeReranker, EpisodeReranker, CommunityReranker,
    DEFAULT_SEARCH_LIMIT,
};
pub use filters::{SearchFilters, DateFilter, ComparisonOperator};
pub use search::GraphitiSearch;
pub use utils::{
    lucene_sanitize, fulltext_query,
    get_episodes_by_mentions, get_mentioned_nodes, get_communities_by_nodes,
    get_relevant_nodes, get_relevant_edges,
    edge_fulltext_search, edge_similarity_search, edge_bfs_search,
    RELEVANT_SCHEMA_LIMIT, DEFAULT_MIN_SCORE, DEFAULT_MMR_LAMBDA, MAX_SEARCH_DEPTH, MAX_QUERY_LENGTH,
};
