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

use uuid::Uuid;
use neo4rs::{BoltType, BoltList, BoltMap};
use crate::{
    errors::GraphitiError,
    edges::EntityEdge,
    nodes::{EntityNode, EpisodicNode, CommunityNode},
    search::{SearchFilters, SearchResult},
    types::GraphitiClients,
};

pub const RELEVANT_SCHEMA_LIMIT: usize = 10;
pub const DEFAULT_MIN_SCORE: f64 = 0.6;
pub const DEFAULT_MMR_LAMBDA: f64 = 0.5;
pub const MAX_SEARCH_DEPTH: i32 = 3;
pub const MAX_QUERY_LENGTH: usize = 32;

/// Convert a serde_json::Value to a neo4rs::BoltType for Neo4j parameters
fn convert_json_to_bolt(value: serde_json::Value) -> Result<BoltType, GraphitiError> {
    match value {
        serde_json::Value::Null => Ok(BoltType::Null(neo4rs::BoltNull)),
        serde_json::Value::Bool(b) => Ok(BoltType::Boolean(neo4rs::BoltBoolean::new(b))),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(BoltType::Integer(neo4rs::BoltInteger::new(i)))
            } else if let Some(f) = n.as_f64() {
                Ok(BoltType::Float(neo4rs::BoltFloat::new(f)))
            } else {
                Err(GraphitiError::ValueError("Invalid number format".to_string()))
            }
        }
        serde_json::Value::String(s) => Ok(BoltType::String(neo4rs::BoltString::new(&s))),
        serde_json::Value::Array(arr) => {
            let mut bolt_list = BoltList::new();
            for item in arr {
                bolt_list.push(convert_json_to_bolt(item)?);
            }
            Ok(BoltType::List(bolt_list))
        }
        serde_json::Value::Object(obj) => {
            let mut bolt_map = BoltMap::new();
            for (key, val) in obj {
                bolt_map.put(neo4rs::BoltString::new(&key), convert_json_to_bolt(val)?);
            }
            Ok(BoltType::Map(bolt_map))
        }
    }
}

/// Sanitize a query string for Lucene full-text search
pub fn lucene_sanitize(query: &str) -> String {
    query
        .chars()
        .filter_map(|c| {
            match c {
                // Escape special Lucene characters
                '+' | '-' | '&' | '|' | '!' | '(' | ')' | '{' | '}' | '[' | ']' | '^' | '"' | '~' | '*' | '?' | ':' | '\\' => {
                    Some(format!("\\{}", c))
                }
                // Keep alphanumeric and space
                c if c.is_alphanumeric() || c.is_whitespace() => Some(c.to_string()),
                // Remove other characters
                _ => None,
            }
        })
        .collect::<Vec<String>>()
        .join("")
}

/// Build a full-text search query with group filtering
pub fn fulltext_query(query: &str, group_ids: Option<&[String]>) -> String {
    let mut group_ids_filter = String::new();
    if let Some(groups) = group_ids {
        let group_filters: Vec<String> = groups
            .iter()
            .map(|g| format!("group_id:\"{}\"", lucene_sanitize(g)))
            .collect();

        if !group_filters.is_empty() {
            group_ids_filter = group_filters.join(" OR ");
            group_ids_filter.push_str(" AND ");
        }
    }

    let lucene_query = lucene_sanitize(query);

    // If the lucene query is too long, return empty query
    if lucene_query.split_whitespace().count() + group_ids.map_or(0, |g| g.len()) >= MAX_QUERY_LENGTH {
        return String::new();
    }

    format!("{}({})", group_ids_filter, lucene_query)
}

/// Get episodes by node and edge mentions
pub async fn get_episodes_by_mentions(
    clients: &GraphitiClients,
    _nodes: &[EntityNode],
    edges: &[EntityEdge],
    limit: usize,
) -> Result<Vec<EpisodicNode>, GraphitiError> {
    let mut episode_uuids = Vec::new();

    for edge in edges {
        episode_uuids.extend(edge.episodes.iter().cloned());
    }

    // Limit to prevent excessive queries
    episode_uuids.truncate(limit);

    // Convert string UUIDs to Uuid objects for validation
    let _uuids: Result<Vec<Uuid>, _> = episode_uuids
        .iter()
        .map(|s| s.parse::<Uuid>())
        .collect();

    let _uuids = _uuids.map_err(|e| GraphitiError::Validation { message: e.to_string() })?;

    // Build Cypher query to get episodes by UUIDs
    if episode_uuids.is_empty() {
        return Ok(Vec::new());
    }

    let cypher = "MATCH (e:EpisodicNode) WHERE e.uuid IN $uuids RETURN e";

    let graph = &clients.driver;
    let mut results = Vec::new();

    let cypher_query = neo4rs::query(cypher)
        .param("uuids", episode_uuids);

    let mut result = graph.execute(cypher_query).await?;

    while let Some(row) = result.next().await? {
        if let Ok(_node_data) = row.get::<neo4rs::Node>("e") {
            // Create a placeholder EpisodicNode for now
            use crate::nodes::{EpisodicNode, BaseNode, EpisodeType};

            let base_node = BaseNode::new("edge_episode".to_string(), "default".to_string());
            let placeholder_episode = EpisodicNode {
                base: base_node,
                source: EpisodeType::Text,
                source_description: "episode from edge mentions".to_string(),
                content: "episode content".to_string(),
                valid_at: chrono::Utc::now(),
                entity_edges: Vec::new(),
            };

            results.push(placeholder_episode);
        }
    }

    Ok(results)
}

/// Get nodes mentioned in episodes
pub async fn get_mentioned_nodes(
    clients: &GraphitiClients,
    episodes: &[EpisodicNode],
) -> Result<Vec<EntityNode>, GraphitiError> {
    if episodes.is_empty() {
        return Ok(Vec::new());
    }

    // Extract entity edges from episodes
    let mut entity_uuids = std::collections::HashSet::new();
    for episode in episodes {
        for entity_uuid in &episode.entity_edges {
            entity_uuids.insert(entity_uuid.clone());
        }
    }

    if entity_uuids.is_empty() {
        return Ok(Vec::new());
    }

    // Build Cypher query to get entities by UUIDs
    let cypher = "MATCH (n:EntityNode) WHERE n.uuid IN $uuids RETURN n";
    let uuid_list: Vec<String> = entity_uuids.into_iter().collect();

    let graph = &clients.driver;
    let mut results = Vec::new();

    let cypher_query = neo4rs::query(cypher)
        .param("uuids", uuid_list);

    let mut result = graph.execute(cypher_query).await?;

    while let Some(row) = result.next().await? {
        if let Ok(_node_data) = row.get::<neo4rs::Node>("n") {
            // Create a placeholder EntityNode for now
            use crate::nodes::{EntityNode, BaseNode};

            let base_node = BaseNode::new("mentioned_entity".to_string(), "default".to_string());
            let placeholder_node = EntityNode {
                base: base_node,
                summary: "mentioned entity summary".to_string(),
                summary_embedding: None,
            };

            results.push(placeholder_node);
        }
    }

    Ok(results)
}

/// Get communities by their member nodes
pub async fn get_communities_by_nodes(
    _clients: &GraphitiClients,
    nodes: &[EntityNode],
) -> Result<Vec<CommunityNode>, GraphitiError> {
    if nodes.is_empty() {
        return Ok(Vec::new());
    }

    // TODO: Implement proper Neo4j query execution
    Ok(Vec::new())
}

/// Perform full-text search on edges
pub async fn edge_fulltext_search(
    _clients: &GraphitiClients,
    query: &str,
    search_filter: &SearchFilters,
    group_ids: Option<&[String]>,
    _limit: usize,
) -> Result<Vec<SearchResult<EntityEdge>>, GraphitiError> {
    let fuzzy_query = fulltext_query(query, group_ids);
    if fuzzy_query.is_empty() {
        return Ok(Vec::new());
    }

    let (_filter_query, _filter_params) = search_filter.edge_search_filter_query();

    // TODO: Implement proper Neo4j full-text search
    Ok(Vec::new())
}

/// Perform vector similarity search on edges
pub async fn edge_similarity_search(
    _clients: &GraphitiClients,
    _search_vector: &[f64],
    _source_node_uuid: Option<&str>,
    _target_node_uuid: Option<&str>,
    search_filter: &SearchFilters,
    _group_ids: Option<&[String]>,
    _limit: usize,
    _min_score: f64,
) -> Result<Vec<SearchResult<EntityEdge>>, GraphitiError> {
    let (_filter_query, _filter_params) = search_filter.edge_search_filter_query();

    // TODO: Implement proper Neo4j vector similarity search
    Ok(Vec::new())
}

/// Perform breadth-first search on edges
pub async fn edge_bfs_search(
    _clients: &GraphitiClients,
    bfs_origin_node_uuids: Option<&[String]>,
    _bfs_max_depth: i32,
    search_filter: &SearchFilters,
    _limit: usize,
) -> Result<Vec<SearchResult<EntityEdge>>, GraphitiError> {
    let Some(_origin_uuids) = bfs_origin_node_uuids else {
        return Ok(Vec::new());
    };

    if _origin_uuids.is_empty() {
        return Ok(Vec::new());
    }

    let (_filter_query, _filter_params) = search_filter.edge_search_filter_query();

    // TODO: Implement proper Neo4j breadth-first search
    Ok(Vec::new())
}

/// Node vector similarity search using cosine similarity
pub async fn node_similarity_search(
    clients: &GraphitiClients,
    query_vector: &[f32],
    filters: &SearchFilters,
    group_ids: Option<&[String]>,
    limit: usize,
) -> Result<Vec<SearchResult<EntityNode>>, GraphitiError> {
    if limit == 0 || query_vector.is_empty() {
        return Ok(Vec::new());
    }

    // Validate group_ids if provided
    if let Some(groups) = group_ids {
        if groups.is_empty() {
            return Ok(Vec::new());
        }
    }

    // Build the vector search query
    let (query, mut params) = build_node_vector_query(filters, group_ids, limit, DEFAULT_MIN_SCORE);

    // Add the embedding vector parameter
    let embedding_json: Vec<serde_json::Value> = query_vector
        .iter()
        .map(|&x| serde_json::Value::Number(serde_json::Number::from_f64(x as f64).unwrap_or(serde_json::Number::from(0))))
        .collect();
    params.insert("embedding_vector".to_string(), serde_json::Value::Array(embedding_json));

    // Execute the Cypher query against Neo4j
    let graph = &clients.driver;
    let mut results = Vec::new();

    // Create query with parameters
    let mut cypher_query = neo4rs::query(&query);
    for (key, value) in params {
        cypher_query = match convert_json_to_bolt(value) {
            Ok(bolt_value) => cypher_query.param(&key, bolt_value),
            Err(_) => continue, // Skip parameters that can't be converted
        };
    }

    let mut result = graph.execute(cypher_query).await?;

    while let Some(row) = result.next().await? {
        // Extract node properties - for now we'll use a simplified approach
        // In a real implementation, we'd need proper node deserialization
        if let Ok(score) = row.get::<f64>("score") {
            // Create a minimal EntityNode for testing
            // In production, this would deserialize the actual node from Neo4j
            use crate::nodes::{EntityNode, BaseNode};

            let base_node = BaseNode::new("placeholder".to_string(), "default".to_string());
            let placeholder_node = EntityNode {
                base: base_node,
                summary: "placeholder summary".to_string(),
                summary_embedding: None,
            };

            results.push(SearchResult {
                item: placeholder_node,
                score
            });
        }
    }

    // Sort by score descending
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    Ok(results)
}

/// Node full-text search using Lucene index
pub async fn node_fulltext_search(
    clients: &GraphitiClients,
    query: &str,
    filters: &SearchFilters,
    group_ids: Option<&[String]>,
    limit: usize,
) -> Result<Vec<SearchResult<EntityNode>>, GraphitiError> {
    if query.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    // Build the full-text search query
    let (cypher, params) = build_node_fulltext_query(query, filters, group_ids, limit);

    // Execute the Cypher query against Neo4j
    let graph = &clients.driver;
    let mut results = Vec::new();

    // Create query with parameters
    let mut cypher_query = neo4rs::query(&cypher);
    for (key, value) in params {
        cypher_query = match convert_json_to_bolt(value) {
            Ok(bolt_value) => cypher_query.param(&key, bolt_value),
            Err(_) => continue, // Skip parameters that can't be converted
        };
    }

    let mut result = graph.execute(cypher_query).await?;

    while let Some(row) = result.next().await? {
        if let (Ok(score), _node_data) = (row.get::<f64>("score"), row.get::<neo4rs::Node>("node")) {
            // Create a placeholder EntityNode for now
            // In production, this would deserialize the actual node from Neo4j
            use crate::nodes::{EntityNode, BaseNode};

            let base_node = BaseNode::new("placeholder".to_string(), "default".to_string());
            let placeholder_node = EntityNode {
                base: base_node,
                summary: "placeholder summary".to_string(),
                summary_embedding: None,
            };

            results.push(SearchResult {
                item: placeholder_node,
                score
            });
        }
    }

    Ok(results)
}

/// Episode full-text search using Lucene index
pub async fn episode_fulltext_search(
    clients: &GraphitiClients,
    query: &str,
    _filters: &SearchFilters,
    group_ids: Option<&[String]>,
    limit: usize,
) -> Result<Vec<SearchResult<EpisodicNode>>, GraphitiError> {
    if query.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let sanitized_query = lucene_sanitize(query);
    let search_query = fulltext_query(&sanitized_query, group_ids);

    if search_query.is_empty() {
        return Ok(Vec::new());
    }

    // Build and execute Neo4j full-text search for episodes
    let cypher = "CALL db.index.fulltext.queryNodes('episode_fulltext_index', $query_text) YIELD node, score RETURN node, score ORDER BY score DESC LIMIT $limit";

    let graph = &clients.driver;
    let mut results = Vec::new();

    let cypher_query = neo4rs::query(cypher)
        .param("query_text", search_query)
        .param("limit", limit as i64);

    let mut result = graph.execute(cypher_query).await?;

    while let Some(row) = result.next().await? {
        if let (Ok(score), _node_data) = (row.get::<f64>("score"), row.get::<neo4rs::Node>("node")) {
            // Create a placeholder EpisodicNode for now
            use crate::nodes::{EpisodicNode, BaseNode, EpisodeType};

            let base_node = BaseNode::new("episode".to_string(), "default".to_string());
            let placeholder_episode = EpisodicNode {
                base: base_node,
                source: EpisodeType::Text,
                source_description: "placeholder source".to_string(),
                content: "placeholder content".to_string(),
                valid_at: chrono::Utc::now(),
                entity_edges: Vec::new(),
            };

            results.push(SearchResult {
                item: placeholder_episode,
                score
            });
        }
    }

    Ok(results)
}

/// Community similarity search using vector embeddings
pub async fn community_similarity_search(
    _clients: &GraphitiClients,
    _query_vector: &[f64],
    _limit: usize,
) -> Result<Vec<SearchResult<CommunityNode>>, GraphitiError> {
    // TODO: Implement proper Neo4j vector search for communities
    Ok(Vec::new())
}

/// Community full-text search using Lucene index
pub async fn community_fulltext_search(
    _clients: &GraphitiClients,
    query: &str,
    group_ids: Option<&[String]>,
    _limit: usize,
) -> Result<Vec<SearchResult<CommunityNode>>, GraphitiError> {
    let fuzzy_query = fulltext_query(query, group_ids);
    if fuzzy_query.is_empty() {
        return Ok(Vec::new());
    }

    // TODO: Implement proper Neo4j full-text search for communities
    Ok(Vec::new())
}

/// Calculate cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        (dot_product / (norm_a * norm_b)) as f64
    }
}

/// Calculate Manhattan distance between two vectors
pub fn manhattan_distance(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() {
        return f64::INFINITY;
    }

    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs() as f64)
        .sum()
}

/// Calculate Euclidean distance between two vectors
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() {
        return f64::INFINITY;
    }

    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = (x - y) as f64;
            diff * diff
        })
        .sum::<f64>()
        .sqrt()
}

/// Build a Cypher query for vector similarity search on nodes
pub fn build_node_vector_query(
    filters: &SearchFilters,
    group_ids: Option<&[String]>,
    limit: usize,
    min_score: f64,
) -> (String, std::collections::HashMap<String, serde_json::Value>) {
    let mut query = String::from(
        "CALL db.index.vector.queryNodes('entity_node_embeddings', $k, $embedding_vector)"
    );

    let mut params = std::collections::HashMap::new();
    params.insert("k".to_string(), serde_json::Value::Number(serde_json::Number::from(limit)));
    params.insert("min_score".to_string(), serde_json::Value::Number(
        serde_json::Number::from_f64(min_score).unwrap_or(serde_json::Number::from(0))
    ));

    query.push_str(" YIELD node, score");

    // Add group_ids filter if provided
    if let Some(groups) = group_ids {
        if !groups.is_empty() {
            query.push_str(" WHERE node.group_id IN $group_ids");
            params.insert("group_ids".to_string(), serde_json::to_value(groups).unwrap());
        }
    }

    // Add search filters
    let (filter_query, filter_params) = filters.node_search_filter_query();
    if !filter_query.is_empty() {
        if query.contains("WHERE") {
            query.push_str(&filter_query);
        } else {
            query.push_str(&format!(" WHERE {}", &filter_query[5..])); // Remove " AND " prefix
        }
    }

    // Merge filter params
    for (key, value) in filter_params {
        params.insert(key, value);
    }

    query.push_str(" RETURN node, score ORDER BY score DESC");

    if limit > 0 {
        query.push_str(&format!(" LIMIT {}", limit));
    }

    (query, params)
}

/// Build a Cypher query for full-text search on nodes
pub fn build_node_fulltext_query(
    query_text: &str,
    filters: &SearchFilters,
    group_ids: Option<&[String]>,
    limit: usize,
) -> (String, std::collections::HashMap<String, serde_json::Value>) {
    let sanitized_query = lucene_sanitize(query_text);
    let search_query = fulltext_query(&sanitized_query, group_ids);

    let mut cypher = String::from(
        "CALL db.index.fulltext.queryNodes('entity_fulltext_index', $query_text)"
    );

    let mut params = std::collections::HashMap::new();
    params.insert("query_text".to_string(), serde_json::Value::String(search_query));

    cypher.push_str(" YIELD node, score");

    // Add search filters
    let (filter_query, filter_params) = filters.node_search_filter_query();
    if !filter_query.is_empty() {
        cypher.push_str(&format!(" WHERE {}", &filter_query[5..])); // Remove " AND " prefix
    }

    // Merge filter params
    for (key, value) in filter_params {
        params.insert(key, value);
    }

    cypher.push_str(" RETURN node, score ORDER BY score DESC");

    if limit > 0 {
        cypher.push_str(&format!(" LIMIT {}", limit));
    }

    (cypher, params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lucene_sanitize() {
        assert_eq!(lucene_sanitize("test+query"), "test\\+query");
        assert_eq!(lucene_sanitize("hello world"), "hello world");
        assert_eq!(lucene_sanitize("user@domain.com"), "userdomaincom");
        assert_eq!(lucene_sanitize("query*"), "query\\*");
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-6);

        let a = vec![1.0, 2.0, 3.0];
        let b = vec![3.0, 2.0, 1.0];
        let expected = 10.0 / (14.0_f32.sqrt() * 14.0_f32.sqrt()) as f64;
        assert!((cosine_similarity(&a, &b) - expected).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_distance() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        assert!((euclidean_distance(&a, &b) - 5.0).abs() < 1e-6);

        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        assert!((euclidean_distance(&a, &b) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_manhattan_distance() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        assert!((manhattan_distance(&a, &b) - 7.0).abs() < 1e-6);

        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        assert!((manhattan_distance(&a, &b) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_fulltext_query() {
        let query = fulltext_query("test query", None);
        assert_eq!(query, "(test query)");

        let groups = vec!["group1".to_string(), "group2".to_string()];
        let query = fulltext_query("test", Some(&groups));
        assert!(query.contains("group_id:\"group1\""));
        assert!(query.contains("group_id:\"group2\""));
        assert!(query.contains("test"));
    }

    #[test]
    fn test_build_node_vector_query() {
        let filters = SearchFilters::new();
        let (query, params) = build_node_vector_query(&filters, None, 10, 0.5);

        assert!(query.contains("db.index.vector.queryNodes"));
        assert!(query.contains("LIMIT 10"));
        assert!(params.contains_key("k"));
        assert!(params.contains_key("min_score"));
    }

    #[test]
    fn test_build_node_fulltext_query() {
        let filters = SearchFilters::new();
        let (query, params) = build_node_fulltext_query("test query", &filters, None, 5);

        assert!(query.contains("db.index.fulltext.queryNodes"));
        assert!(query.contains("LIMIT 5"));
        assert!(params.contains_key("query_text"));
    }
}
