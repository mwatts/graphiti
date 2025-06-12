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

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanEqual,
    LessThanEqual,
}

impl ComparisonOperator {
    pub fn to_cypher(&self) -> &'static str {
        match self {
            ComparisonOperator::Equals => "=",
            ComparisonOperator::NotEquals => "<>",
            ComparisonOperator::GreaterThan => ">",
            ComparisonOperator::LessThan => "<",
            ComparisonOperator::GreaterThanEqual => ">=",
            ComparisonOperator::LessThanEqual => "<=",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateFilter {
    pub date: DateTime<Utc>,
    pub comparison_operator: ComparisonOperator,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchFilters {
    pub node_labels: Option<Vec<String>>,
    pub edge_types: Option<Vec<String>>,
    pub valid_at: Option<Vec<Vec<DateFilter>>>,
    pub invalid_at: Option<Vec<Vec<DateFilter>>>,
    pub created_at: Option<Vec<Vec<DateFilter>>>,
    pub expired_at: Option<Vec<Vec<DateFilter>>>,
}

impl SearchFilters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_node_labels(mut self, labels: Vec<String>) -> Self {
        self.node_labels = Some(labels);
        self
    }

    pub fn with_edge_types(mut self, types: Vec<String>) -> Self {
        self.edge_types = Some(types);
        self
    }

    pub fn with_valid_at(mut self, filters: Vec<Vec<DateFilter>>) -> Self {
        self.valid_at = Some(filters);
        self
    }

    /// Construct node search filter query parts
    pub fn node_search_filter_query(&self) -> (String, HashMap<String, serde_json::Value>) {
        let mut filter_query = String::new();
        let filter_params = HashMap::new();

        if let Some(ref node_labels) = self.node_labels {
            let node_labels_str = node_labels.join("|");
            filter_query.push_str(&format!(" AND n:{}", node_labels_str));
        }

        (filter_query, filter_params)
    }

    /// Construct edge search filter query parts
    pub fn edge_search_filter_query(&self) -> (String, HashMap<String, serde_json::Value>) {
        let mut filter_query = String::new();
        let mut filter_params = HashMap::new();

        if let Some(ref edge_types) = self.edge_types {
            filter_query.push_str("\nAND r.name in $edge_types");
            filter_params.insert(
                "edge_types".to_string(),
                serde_json::to_value(edge_types).unwrap(),
            );
        }

        if let Some(ref node_labels) = self.node_labels {
            let node_labels_str = node_labels.join("|");
            filter_query.push_str(&format!(
                "\nAND n:{} AND m:{}",
                node_labels_str, node_labels_str
            ));
        }

        if let Some(ref valid_at) = self.valid_at {
            filter_query.push_str("\nAND (");
            for (i, or_list) in valid_at.iter().enumerate() {
                for (j, date_filter) in or_list.iter().enumerate() {
                    let param_name = format!("valid_at_{}", j);
                    filter_params.insert(
                        param_name.clone(),
                        serde_json::to_value(date_filter.date.timestamp()).unwrap(),
                    );
                }

                let and_filters: Vec<String> = or_list
                    .iter()
                    .enumerate()
                    .map(|(j, date_filter)| {
                        format!(
                            "(r.valid_at {} $valid_at_{})",
                            date_filter.comparison_operator.to_cypher(),
                            j
                        )
                    })
                    .collect();

                let and_filter_query = and_filters.join(" AND ");
                filter_query.push_str(&and_filter_query);

                if i != valid_at.len() - 1 {
                    filter_query.push_str(" OR ");
                }
            }
            filter_query.push(')');
        }

        (filter_query, filter_params)
    }

    /// Construct episode search filter query parts
    pub fn episode_search_filter_query(&self) -> (String, HashMap<String, serde_json::Value>) {
        let mut filter_query = String::new();
        let mut filter_params = HashMap::new();

        if let Some(ref created_at) = self.created_at {
            filter_query.push_str("\nAND (");
            for (i, or_list) in created_at.iter().enumerate() {
                for (j, date_filter) in or_list.iter().enumerate() {
                    let param_name = format!("created_at_{}", j);
                    filter_params.insert(
                        param_name.clone(),
                        serde_json::to_value(date_filter.date.timestamp()).unwrap(),
                    );
                }

                let and_filters: Vec<String> = or_list
                    .iter()
                    .enumerate()
                    .map(|(j, date_filter)| {
                        format!(
                            "(n.created_at {} $created_at_{})",
                            date_filter.comparison_operator.to_cypher(),
                            j
                        )
                    })
                    .collect();

                let and_filter_query = and_filters.join(" AND ");
                filter_query.push_str(&and_filter_query);

                if i != created_at.len() - 1 {
                    filter_query.push_str(" OR ");
                }
            }
            filter_query.push(')');
        }

        (filter_query, filter_params)
    }

    /// Construct community search filter query parts
    pub fn community_search_filter_query(&self) -> (String, HashMap<String, serde_json::Value>) {
        let filter_query = String::new();
        let filter_params = HashMap::new();
        // Communities don't have specific filters in the Python implementation
        (filter_query, filter_params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_filters_new() {
        let filters = SearchFilters::new();
        assert!(filters.node_labels.is_none());
        assert!(filters.edge_types.is_none());
    }

    #[test]
    fn test_search_filters_with_node_labels() {
        let filters =
            SearchFilters::new().with_node_labels(vec!["Entity".to_string(), "Person".to_string()]);
        assert_eq!(filters.node_labels.unwrap(), vec!["Entity", "Person"]);
    }

    #[test]
    fn test_node_search_filter_query() {
        let filters = SearchFilters::new().with_node_labels(vec!["Entity".to_string()]);
        let (query, _params) = filters.node_search_filter_query();
        assert_eq!(query, " AND n:Entity");
    }

    #[test]
    fn test_edge_search_filter_query() {
        let filters = SearchFilters::new().with_edge_types(vec!["RELATES_TO".to_string()]);
        let (query, params) = filters.edge_search_filter_query();
        assert!(query.contains("AND r.name in $edge_types"));
        assert!(params.contains_key("edge_types"));
    }

    #[test]
    fn test_comparison_operator_to_cypher() {
        assert_eq!(ComparisonOperator::Equals.to_cypher(), "=");
        assert_eq!(ComparisonOperator::GreaterThan.to_cypher(), ">");
        assert_eq!(ComparisonOperator::LessThanEqual.to_cypher(), "<=");
    }
}
