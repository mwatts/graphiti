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

//! Caching layer for graphiti-core
//!
//! This module provides caching functionality for LLM responses, embeddings,
//! and other expensive operations. It supports both in-memory and persistent caching.

pub mod disk_cache;
pub mod memory_cache;

use std::time::Duration;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::errors::GraphitiResult;

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Default TTL for cache entries
    pub default_ttl: Duration,
    /// Maximum cache size (for memory cache)
    pub max_size: Option<u64>,
    /// Cache directory (for disk cache)
    pub cache_dir: Option<String>,
    /// Whether to enable persistent cache
    pub persistent: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Duration::from_secs(3600), // 1 hour
            max_size: Some(1024 * 1024 * 100), // 100MB
            cache_dir: Some("./cache".to_string()),
            persistent: true,
        }
    }
}

/// Trait for cache implementations
#[async_trait]
pub trait Cache: Send + Sync {
    /// Get a value from the cache
    async fn get(&self, key: &str) -> GraphitiResult<Option<Vec<u8>>>;

    /// Set a value in the cache with default TTL
    async fn set(&self, key: &str, value: Vec<u8>) -> GraphitiResult<()>;

    /// Set a value in the cache with custom TTL
    async fn set_with_ttl(&self, key: &str, value: Vec<u8>, ttl: Duration) -> GraphitiResult<()>;

    /// Remove a value from the cache
    async fn remove(&self, key: &str) -> GraphitiResult<()>;

    /// Clear all cache entries
    async fn clear(&self) -> GraphitiResult<()>;

    /// Check if a key exists in the cache
    async fn exists(&self, key: &str) -> GraphitiResult<bool>;

    /// Get cache statistics
    async fn stats(&self) -> GraphitiResult<CacheStats>;
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: u64,
    pub size_bytes: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Generate a cache key from components
pub fn generate_cache_key(components: &[&str]) -> String {
    use sha2::{Digest, Sha256};

    let combined = components.join("|");
    let mut hasher = Sha256::new();
    hasher.update(combined.as_bytes());
    let hash = hasher.finalize();
    format!("{:x}", hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.default_ttl, Duration::from_secs(3600));
        assert_eq!(config.max_size, Some(1024 * 1024 * 100));
        assert!(config.persistent);
    }

    #[test]
    fn test_generate_cache_key() {
        let key1 = generate_cache_key(&["test", "key", "1"]);
        let key2 = generate_cache_key(&["test", "key", "2"]);
        let key3 = generate_cache_key(&["test", "key", "1"]);

        assert_ne!(key1, key2);
        assert_eq!(key1, key3);
        assert_eq!(key1.len(), 64); // SHA256 hex string length
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let stats = CacheStats {
            hits: 80,
            misses: 20,
            entries: 50,
            size_bytes: 1024,
        };

        assert_eq!(stats.hit_rate(), 0.8);

        let empty_stats = CacheStats {
            hits: 0,
            misses: 0,
            entries: 0,
            size_bytes: 0,
        };

        assert_eq!(empty_stats.hit_rate(), 0.0);
    }
}
