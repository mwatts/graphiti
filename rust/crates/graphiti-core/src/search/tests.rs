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

#[cfg(test)]
mod tests {
    use crate::{SearchConfig, SearchFilters, SearchResults};

    #[test]
    fn test_search_config_creation() {
        let config = SearchConfig::default();
        assert_eq!(config.limit, 10);
        assert_eq!(config.node_search_config.sim_min_score, 0.0);
    }

    #[test]
    fn test_search_filters_creation() {
        let filters = SearchFilters::new();
        assert!(filters.node_labels.is_none());
        assert!(filters.edge_types.is_none());
    }

    #[test]
    fn test_search_results_creation() {
        let results = SearchResults::new();
        assert!(results.nodes.is_empty());
        assert!(results.edges.is_empty());
        assert!(results.episodes.is_empty());
        assert!(results.communities.is_empty());
    }

    #[test]
    fn test_fulltext_query_generation() {
        use crate::search::utils::fulltext_query;

        // Test simple query
        let query = "test query";
        let result = fulltext_query(query, None);
        assert_eq!(result, "(test query)");

        // Test query with groups
        let groups = vec!["group1".to_string(), "group2".to_string()];
        let result = fulltext_query("test", Some(&groups));
        assert!(result.contains("group_id:\"group1\""));
        assert!(result.contains("group_id:\"group2\""));
        assert!(result.contains("(test)"));
    }

    #[test]
    fn test_lucene_sanitize() {
        use crate::search::utils::lucene_sanitize;

        let query = "test+query-with:special*chars";
        let sanitized = lucene_sanitize(query);
        assert_eq!(sanitized, "test\\+query\\-with\\:special\\*chars");
    }
}
