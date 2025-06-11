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

//! Maintenance utility functions

use std::collections::HashMap;
use uuid::Uuid;
use crate::{
    nodes::EntityNode,
    edges::EntityEdge,
    errors::GraphitiError,
};

/// Validate graph consistency
pub async fn validate_graph_consistency(
    nodes: &[EntityNode],
    edges: &[EntityEdge],
) -> Result<ValidationReport, GraphitiError> {
    let mut report = ValidationReport::default();

    // Check for orphaned edges (edges pointing to non-existent nodes)
    let node_uuids: std::collections::HashSet<_> = nodes.iter().map(|n| n.base.uuid.clone()).collect();

    for edge in edges {
        if !node_uuids.contains(&edge.base.source_node_uuid) {
            if let Ok(uuid) = edge.base.uuid.parse::<Uuid>() {
                report.orphaned_edges.push(uuid);
            }
        }
        if !node_uuids.contains(&edge.base.target_node_uuid) {
            if let Ok(uuid) = edge.base.uuid.parse::<Uuid>() {
                report.orphaned_edges.push(uuid);
            }
        }
    }

    // Check for self-loops
    for edge in edges {
        if edge.base.source_node_uuid == edge.base.target_node_uuid {
            if let Ok(uuid) = edge.base.uuid.parse::<Uuid>() {
                report.self_loops.push(uuid);
            }
        }
    }

    // Check for duplicate nodes (same name)
    let mut name_counts = HashMap::new();
    for node in nodes {
        *name_counts.entry(node.base.name.clone()).or_insert(0) += 1;
    }

    for (name, count) in name_counts {
        if count > 1 {
            report.duplicate_node_names.push(name);
        }
    }

    Ok(report)
}

/// Graph validation report
#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    pub orphaned_edges: Vec<Uuid>,
    pub self_loops: Vec<Uuid>,
    pub duplicate_node_names: Vec<String>,
    pub temporal_inconsistencies: Vec<Uuid>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.orphaned_edges.is_empty()
            && self.self_loops.is_empty()
            && self.duplicate_node_names.is_empty()
            && self.temporal_inconsistencies.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.orphaned_edges.len()
            + self.self_loops.len()
            + self.duplicate_node_names.len()
            + self.temporal_inconsistencies.len()
    }
}

/// Clean up graph data by removing invalid elements
pub fn cleanup_graph_data(
    mut nodes: Vec<EntityNode>,
    mut edges: Vec<EntityEdge>,
    validation_report: &ValidationReport,
) -> (Vec<EntityNode>, Vec<EntityEdge>) {
    // Remove edges with orphaned references
    edges.retain(|edge| {
        if let Ok(edge_uuid) = edge.base.uuid.parse::<Uuid>() {
            !validation_report.orphaned_edges.contains(&edge_uuid)
        } else {
            true // Keep edges with invalid UUIDs for now
        }
    });

    // Remove self-loops if desired
    edges.retain(|edge| {
        if let Ok(edge_uuid) = edge.base.uuid.parse::<Uuid>() {
            !validation_report.self_loops.contains(&edge_uuid)
        } else {
            true // Keep edges with invalid UUIDs for now
        }
    });

    // For duplicate node names, keep only the first occurrence
    let mut seen_names = std::collections::HashSet::new();
    nodes.retain(|node| {
        if validation_report.duplicate_node_names.contains(&node.base.name) {
            seen_names.insert(node.base.name.clone())
        } else {
            true
        }
    });

    (nodes, edges)
}

/// Merge similar nodes based on similarity threshold
pub fn merge_similar_nodes(
    nodes: Vec<EntityNode>,
    _similarity_threshold: f64,
) -> (Vec<EntityNode>, HashMap<Uuid, Uuid>) {
    // Stub implementation - would use embedding similarity to merge nodes
    // Returns (merged_nodes, uuid_mapping)

    let uuid_mapping = HashMap::new();
    (nodes, uuid_mapping)
}

/// Calculate graph statistics
pub fn calculate_graph_stats(
    nodes: &[EntityNode],
    edges: &[EntityEdge],
) -> GraphStats {
    let node_count = nodes.len();
    let edge_count = edges.len();

    // Calculate degree distribution
    let mut degree_counts = HashMap::new();
    for edge in edges {
        let source_uuid = edge.base.source_node_uuid.clone();
        let target_uuid = edge.base.target_node_uuid.clone();
        *degree_counts.entry(source_uuid).or_insert(0) += 1;
        *degree_counts.entry(target_uuid).or_insert(0) += 1;
    }

    let avg_degree = if node_count > 0 {
        degree_counts.values().sum::<usize>() as f64 / node_count as f64
    } else {
        0.0
    };

    let max_degree = degree_counts.values().max().copied().unwrap_or(0);

    // Calculate density
    let max_possible_edges = if node_count > 1 {
        node_count * (node_count - 1) / 2
    } else {
        0
    };

    let density = if max_possible_edges > 0 {
        edge_count as f64 / max_possible_edges as f64
    } else {
        0.0
    };

    GraphStats {
        node_count,
        edge_count,
        avg_degree,
        max_degree,
        density,
    }
}

/// Graph statistics
#[derive(Debug, Clone)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub avg_degree: f64,
    pub max_degree: usize,
    pub density: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Complex test - needs proper setup
    async fn test_validate_graph_consistency() {
        // Test disabled until we have proper setup
    }

    #[test]
    fn test_validation_report() {
        let report = ValidationReport::default();
        assert!(report.is_valid());
        assert_eq!(report.error_count(), 0);
    }

    #[test]
    fn test_graph_stats_empty() {
        let nodes = Vec::new();
        let edges = Vec::new();
        let stats = calculate_graph_stats(&nodes, &edges);

        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.edge_count, 0);
        assert_eq!(stats.avg_degree, 0.0);
        assert_eq!(stats.max_degree, 0);
        assert_eq!(stats.density, 0.0);
    }
}
