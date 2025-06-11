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

//! Bulk processing utilities for nodes and edges

use std::collections::HashMap;
use chrono::{DateTime, Utc};
use neo4rs::{Graph, Txn};
use uuid::Uuid;
use crate::{
    types::GraphitiClients,
    nodes::{EntityNode, EpisodicNode, EpisodeType},
    edges::{EntityEdge, EpisodicEdge},
    embedder::EmbedderClient,
    llm_client::LlmClient,
    search::{SearchFilters, get_relevant_nodes, get_relevant_edges},
    utils::datetime_utils::utc_now,
    helpers::semaphore_gather,
    errors::GraphitiError,
};

/// Chunk size for batch processing
const CHUNK_SIZE: usize = 10;

/// Raw episode data for bulk processing
#[derive(Debug, Clone)]
pub struct RawEpisode {
    pub name: String,
    pub content: String,
    pub source_description: String,
    pub source: EpisodeType,
    pub reference_time: DateTime<Utc>,
}

/// Retrieve previous episodes for multiple episodes in bulk
pub async fn retrieve_previous_episodes_bulk(
    graph: &Graph,
    episodes: &[EpisodicNode],
    episode_window_len: usize,
) -> Result<Vec<(EpisodicNode, Vec<EpisodicNode>)>, GraphitiError> {
    let futures: Vec<_> = episodes
        .iter()
        .map(|episode| async move {
            // Stub: In actual implementation, would call retrieve_episodes
            // from maintenance/temporal_operations
            (episode.clone(), Vec::new())
        })
        .collect();

    let results = semaphore_gather(futures, None).await;
    Ok(results)
}

/// Add nodes and edges in bulk to the database
pub async fn add_nodes_and_edges_bulk(
    graph: &Graph,
    episodic_nodes: Vec<EpisodicNode>,
    episodic_edges: Vec<EpisodicEdge>,
    entity_nodes: Vec<EntityNode>,
    entity_edges: Vec<EntityEdge>,
    embedder: &dyn EmbedderClient,
) -> Result<(), GraphitiError> {
    let mut txn = graph.start_txn().await?;

    add_nodes_and_edges_bulk_tx(
        &mut txn,
        episodic_nodes,
        episodic_edges,
        entity_nodes,
        entity_edges,
        embedder,
    ).await?;

    txn.commit().await?;
    Ok(())
}

/// Transaction-level bulk add operation
async fn add_nodes_and_edges_bulk_tx(
    txn: &mut Txn,
    episodic_nodes: Vec<EpisodicNode>,
    episodic_edges: Vec<EpisodicEdge>,
    mut entity_nodes: Vec<EntityNode>,
    mut entity_edges: Vec<EntityEdge>,
    embedder: &dyn EmbedderClient,
) -> Result<(), GraphitiError> {
    // Generate embeddings for nodes that don't have them
    for node in &mut entity_nodes {
        if node.name_embedding.is_none() {
            node.generate_name_embedding(embedder).await?;
        }
    }

    // Generate embeddings for edges that don't have them
    for edge in &mut entity_edges {
        if edge.fact_embedding.is_none() {
            edge.generate_embedding(embedder).await?;
        }
    }

    // Prepare data for bulk insertion
    // Note: In actual implementation, would use proper Cypher queries
    // For now, we'll stub this out

    // TODO: Implement actual bulk save queries
    // - EPISODIC_NODE_SAVE_BULK
    // - ENTITY_NODE_SAVE_BULK
    // - EPISODIC_EDGE_SAVE_BULK
    // - ENTITY_EDGE_SAVE_BULK

    Ok(())
}

/// Extract nodes and edges from multiple episodes in bulk
pub async fn extract_nodes_and_edges_bulk(
    clients: &GraphitiClients,
    episode_tuples: Vec<(EpisodicNode, Vec<EpisodicNode>)>,
) -> Result<(Vec<EntityNode>, Vec<EntityEdge>, Vec<EpisodicEdge>), GraphitiError> {
    // Extract nodes from all episodes
    let node_futures: Vec<_> = episode_tuples
        .iter()
        .map(|(episode, previous_episodes)| async move {
            // Stub: Would call extract_nodes from maintenance/node_operations
            Vec::<EntityNode>::new()
        })
        .collect();

    let extracted_nodes_bulk = semaphore_gather(node_futures, None).await;

    // Extract edges from all episodes
    let edge_futures: Vec<_> = episode_tuples
        .iter()
        .enumerate()
        .map(|(i, (episode, previous_episodes))| async move {
            // Stub: Would call extract_edges from maintenance/edge_operations
            Vec::<EntityEdge>::new()
        })
        .collect();

    let extracted_edges_bulk = semaphore_gather(edge_futures, None).await;

    // Build episodic edges
    let mut episodic_edges = Vec::new();
    for (i, (episode, _)) in episode_tuples.iter().enumerate() {
        // Stub: Would call build_episodic_edges
        // episodic_edges.extend(build_episodic_edges(&extracted_nodes_bulk[i], episode, episode.created_at));
    }

    // Flatten results
    let nodes: Vec<EntityNode> = extracted_nodes_bulk.into_iter().flatten().collect();
    let edges: Vec<EntityEdge> = extracted_edges_bulk.into_iter().flatten().collect();

    Ok((nodes, edges, episodic_edges))
}

/// Deduplicate nodes in bulk
pub async fn dedupe_nodes_bulk(
    graph: &Graph,
    llm_client: &dyn LlmClient,
    extracted_nodes: Vec<EntityNode>,
) -> Result<(Vec<EntityNode>, HashMap<String, String>), GraphitiError> {
    // First, match nodes by name
    let (nodes, uuid_map) = node_name_match(extracted_nodes);

    // Compress nodes using LLM-based deduplication
    let (compressed_nodes, compressed_map) = compress_nodes(llm_client, nodes, uuid_map).await?;

    // Split into chunks for parallel processing
    let node_chunks: Vec<Vec<EntityNode>> = compressed_nodes
        .chunks(CHUNK_SIZE)
        .map(|chunk| chunk.to_vec())
        .collect();

    // Get existing nodes for each chunk
    let existing_futures: Vec<_> = node_chunks
        .iter()
        .map(|chunk| async move {
            get_relevant_nodes(graph, chunk, &SearchFilters::default()).await.unwrap_or_default()
        })
        .collect();

    let _existing_nodes_chunks = semaphore_gather(existing_futures, None).await;

    // Deduplicate each chunk against existing nodes
    let dedupe_futures: Vec<_> = node_chunks
        .into_iter()
        .enumerate()
        .map(|(_i, chunk)| async move {
            // Stub: Would call dedupe_extracted_nodes
            (chunk, HashMap::<String, String>::new())
        })
        .collect();

    let results = semaphore_gather(dedupe_futures, None).await;

    // Combine results
    let mut final_nodes = Vec::new();
    let mut final_uuid_map = compressed_map;

    for (nodes, partial_uuid_map) in results {
        final_nodes.extend(nodes);
        final_uuid_map.extend(partial_uuid_map);
    }

    Ok((final_nodes, final_uuid_map))
}

/// Deduplicate edges in bulk
pub async fn dedupe_edges_bulk(
    graph: &Graph,
    llm_client: &dyn LlmClient,
    extracted_edges: Vec<EntityEdge>,
) -> Result<Vec<EntityEdge>, GraphitiError> {
    // First compress edges
    let compressed_edges = compress_edges(llm_client, extracted_edges).await?;

    // Split into chunks for parallel processing
    let edge_chunks: Vec<Vec<EntityEdge>> = compressed_edges
        .chunks(CHUNK_SIZE)
        .map(|chunk| chunk.to_vec())
        .collect();

    // Get relevant edges for each chunk
    let relevant_futures: Vec<_> = edge_chunks
        .iter()
        .map(|chunk| async move {
            get_relevant_edges(graph, chunk, &SearchFilters::default()).await.unwrap_or_default()
        })
        .collect();

    let _relevant_edges_chunks = semaphore_gather(relevant_futures, None).await;

    // Deduplicate each chunk
    let dedupe_futures: Vec<_> = edge_chunks
        .into_iter()
        .enumerate()
        .map(|(_i, chunk)| async move {
            // Stub: Would call dedupe_extracted_edges
            chunk
        })
        .collect();

    let resolved_edge_chunks = semaphore_gather(dedupe_futures, None).await;

    // Flatten results
    let edges: Vec<EntityEdge> = resolved_edge_chunks.into_iter().flatten().collect();
    Ok(edges)
}

/// Match nodes by name to find duplicates
fn node_name_match(nodes: Vec<EntityNode>) -> (Vec<EntityNode>, HashMap<String, String>) {
    let mut uuid_map = HashMap::new();
    let mut name_map = HashMap::new();
    let mut unique_nodes = Vec::new();

    for node in nodes {
        if let Some(existing_node) = name_map.get(&node.name) {
            // Found duplicate by name
            uuid_map.insert(node.uuid.to_string(), existing_node.uuid.to_string());
        } else {
            // New unique node
            name_map.insert(node.name.clone(), node.clone());
            unique_nodes.push(node);
        }
    }

    (unique_nodes, uuid_map)
}

/// Compress nodes using LLM-based deduplication
async fn compress_nodes(
    llm_client: &dyn LlmClient,
    nodes: Vec<EntityNode>,
    uuid_map: HashMap<String, String>,
) -> Result<(Vec<EntityNode>, HashMap<String, String>), GraphitiError> {
    if nodes.is_empty() {
        return Ok((nodes, uuid_map));
    }

    // Calculate optimal chunk size (sqrt of total nodes, minimum CHUNK_SIZE)
    let chunk_size = (nodes.len() as f64).sqrt().max(CHUNK_SIZE as f64) as usize;

    // For now, we'll use a simplified approach without semantic similarity
    // TODO: Implement semantic similarity-based chunking using embeddings

    let node_chunks: Vec<Vec<EntityNode>> = nodes
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect();

    // Process chunks in parallel
    let dedupe_futures: Vec<_> = node_chunks
        .into_iter()
        .map(|chunk| async move {
            // Stub: Would call dedupe_node_list
            (chunk, HashMap::<String, String>::new())
        })
        .collect();

    let results = semaphore_gather(dedupe_futures, None).await;

    // Combine results
    let mut extended_map = uuid_map;
    let mut compressed_nodes = Vec::new();

    for (chunk_nodes, chunk_uuid_map) in results {
        compressed_nodes.extend(chunk_nodes);
        extended_map.extend(chunk_uuid_map);
    }

    // Check if we need another round of compression
    if compressed_nodes.len() == nodes.len() {
        let compressed_uuid_map = compress_uuid_map(extended_map);
        Ok((compressed_nodes, compressed_uuid_map))
    } else {
        // Recursive compression
        compress_nodes(llm_client, compressed_nodes, extended_map).await
    }
}

/// Compress edges using LLM-based deduplication
async fn compress_edges(
    llm_client: &dyn LlmClient,
    edges: Vec<EntityEdge>,
) -> Result<Vec<EntityEdge>, GraphitiError> {
    if edges.is_empty() {
        return Ok(edges);
    }

    // Group edges by node pairs
    let edge_chunks = chunk_edges_by_nodes(edges);

    // Process chunks in parallel
    let dedupe_futures: Vec<_> = edge_chunks
        .into_iter()
        .map(|chunk| async move {
            // Stub: Would call dedupe_edge_list
            chunk
        })
        .collect();

    let results = semaphore_gather(dedupe_futures, None).await;
    let compressed_edges: Vec<EntityEdge> = results.into_iter().flatten().collect();

    // Check if we need another round of compression
    if compressed_edges.len() == edges.len() {
        Ok(compressed_edges)
    } else {
        // Recursive compression
        compress_edges(llm_client, compressed_edges).await
    }
}

/// Compress UUID mapping to resolve transitive mappings
fn compress_uuid_map(uuid_map: HashMap<String, String>) -> HashMap<String, String> {
    let mut compressed_map = HashMap::new();

    for (key, mut value) in uuid_map.iter() {
        // Follow the chain to find the final mapping
        while let Some(next_value) = uuid_map.get(&value) {
            value = next_value.clone();
        }
        compressed_map.insert(key.clone(), value);
    }

    compressed_map
}

/// Resolve edge pointers using UUID mapping
pub fn resolve_edge_pointers<E: AsRef<EntityEdge> + AsMut<EntityEdge>>(
    edges: &mut [E],
    uuid_map: &HashMap<String, String>,
) {
    for edge in edges {
        let edge = edge.as_mut();

        // Resolve source node UUID
        if let Some(new_uuid) = uuid_map.get(&edge.source_node_uuid.to_string()) {
            if let Ok(uuid) = Uuid::parse_str(new_uuid) {
                edge.source_node_uuid = uuid;
            }
        }

        // Resolve target node UUID
        if let Some(new_uuid) = uuid_map.get(&edge.target_node_uuid.to_string()) {
            if let Ok(uuid) = Uuid::parse_str(new_uuid) {
                edge.target_node_uuid = uuid;
            }
        }
    }
}

/// Extract edge dates in bulk
pub async fn extract_edge_dates_bulk(
    llm_client: &dyn LlmClient,
    mut extracted_edges: Vec<EntityEdge>,
    episode_pairs: Vec<(EpisodicNode, Vec<EpisodicNode>)>,
) -> Result<Vec<EntityEdge>, GraphitiError> {
    // Filter edges that have episodes
    let mut edges_with_episodes = Vec::new();
    for edge in extracted_edges {
        if edge.episodes.as_ref().map_or(false, |eps| !eps.is_empty()) {
            edges_with_episodes.push(edge);
        }
    }

    // Create episode UUID mapping
    let episode_uuid_map: HashMap<String, (EpisodicNode, Vec<EpisodicNode>)> = episode_pairs
        .into_iter()
        .map(|(episode, previous_episodes)| (episode.uuid.to_string(), (episode, previous_episodes)))
        .collect();

    // Extract dates for each edge
    let date_futures: Vec<_> = edges_with_episodes
        .iter()
        .map(|edge| async move {
            if let Some(episodes) = &edge.episodes {
                if let Some(episode_uuid) = episodes.first() {
                    if let Some((episode, previous_episodes)) = episode_uuid_map.get(&episode_uuid.to_string()) {
                        // Stub: Would call extract_edge_dates
                        return (None, None);
                    }
                }
            }
            (None, None)
        })
        .collect();

    let results = semaphore_gather(date_futures, None).await;

    // Apply extracted dates
    for (i, (valid_at, invalid_at)) in results.into_iter().enumerate() {
        let edge = &mut edges_with_episodes[i];
        edge.valid_at = valid_at;
        edge.invalid_at = invalid_at;

        if edge.invalid_at.is_some() {
            edge.expired_at = Some(utc_now());
        }
    }

    Ok(edges_with_episodes)
}

/// Group edges by node pairs for deduplication
fn chunk_edges_by_nodes(edges: Vec<EntityEdge>) -> Vec<Vec<EntityEdge>> {
    let mut edge_chunk_map: HashMap<String, Vec<EntityEdge>> = HashMap::new();

    for edge in edges {
        // Skip self-loops
        if edge.source_node_uuid == edge.target_node_uuid {
            continue;
        }

        // Create consistent key regardless of direction
        let mut pointers = vec![
            edge.source_node_uuid.to_string(),
            edge.target_node_uuid.to_string(),
        ];
        pointers.sort();
        let key = pointers.join("");

        edge_chunk_map.entry(key).or_default().push(edge);
    }

    edge_chunk_map.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_node_name_match() {
        let nodes = vec![
            EntityNode {
                uuid: Uuid::new_v4(),
                name: "Alice".to_string(),
                ..Default::default()
            },
            EntityNode {
                uuid: Uuid::new_v4(),
                name: "Bob".to_string(),
                ..Default::default()
            },
            EntityNode {
                uuid: Uuid::new_v4(),
                name: "Alice".to_string(), // Duplicate
                ..Default::default()
            },
        ];

        let (unique_nodes, uuid_map) = node_name_match(nodes);

        assert_eq!(unique_nodes.len(), 2);
        assert_eq!(uuid_map.len(), 1);
    }

    #[test]
    fn test_compress_uuid_map() {
        let mut uuid_map = HashMap::new();
        uuid_map.insert("a".to_string(), "b".to_string());
        uuid_map.insert("b".to_string(), "c".to_string());
        uuid_map.insert("d".to_string(), "e".to_string());

        let compressed = compress_uuid_map(uuid_map);

        assert_eq!(compressed.get("a"), Some(&"c".to_string()));
        assert_eq!(compressed.get("b"), Some(&"c".to_string()));
        assert_eq!(compressed.get("d"), Some(&"e".to_string()));
    }

    #[test]
    fn test_chunk_edges_by_nodes() {
        let edges = vec![
            EntityEdge {
                source_node_uuid: Uuid::new_v4(),
                target_node_uuid: Uuid::new_v4(),
                ..Default::default()
            },
            EntityEdge {
                source_node_uuid: Uuid::new_v4(),
                target_node_uuid: Uuid::new_v4(),
                ..Default::default()
            },
        ];

        let chunks = chunk_edges_by_nodes(edges);
        assert_eq!(chunks.len(), 2); // Each edge in its own chunk since different node pairs
    }
}
