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

//! In-memory cache implementation using moka

use async_trait::async_trait;
use moka::future::Cache as MokaCache;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use crate::cache::{Cache, CacheConfig, CacheStats};
use crate::errors::GraphitiResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    data: Vec<u8>,
    expires_at: Option<u64>, // Unix timestamp
}

impl CacheEntry {
    fn new(data: Vec<u8>, ttl: Option<Duration>) -> Self {
        let expires_at = ttl.map(|ttl| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
                + ttl.as_millis() as u64
        });

        Self { data, expires_at }
    }

    fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            now > expires_at
        } else {
            false
        }
    }
}

/// In-memory cache implementation
pub struct MemoryCache {
    cache: MokaCache<String, CacheEntry>,
    config: CacheConfig,
    stats: Arc<RwLock<CacheStats>>,
}

impl MemoryCache {
    /// Create a new memory cache
    pub fn new(config: CacheConfig) -> Self {
        let cache = if let Some(max_size) = config.max_size {
            MokaCache::builder()
                .weigher(|_key: &String, entry: &CacheEntry| entry.data.len() as u32)
                .max_capacity(max_size)
                .build()
        } else {
            MokaCache::new(u64::MAX)
        };

        Self {
            cache,
            config,
            stats: Arc::new(RwLock::new(CacheStats {
                hits: 0,
                misses: 0,
                entries: 0,
                size_bytes: 0,
            })),
        }
    }

    async fn update_stats_hit(&self) {
        let mut stats = self.stats.write().await;
        stats.hits += 1;
    }

    async fn update_stats_miss(&self) {
        let mut stats = self.stats.write().await;
        stats.misses += 1;
    }

    async fn update_stats_set(&self, _size: usize) {
        let mut stats = self.stats.write().await;
        stats.entries = self.cache.entry_count();
        stats.size_bytes = self.cache.weighted_size();
    }

    async fn update_stats_remove(&self) {
        let mut stats = self.stats.write().await;
        stats.entries = self.cache.entry_count();
        stats.size_bytes = self.cache.weighted_size();
    }
}

#[async_trait]
impl Cache for MemoryCache {
    async fn get(&self, key: &str) -> GraphitiResult<Option<Vec<u8>>> {
        if let Some(entry) = self.cache.get(key).await {
            if entry.is_expired() {
                self.cache.remove(key).await;
                self.update_stats_miss().await;
                Ok(None)
            } else {
                self.update_stats_hit().await;
                Ok(Some(entry.data))
            }
        } else {
            self.update_stats_miss().await;
            Ok(None)
        }
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> GraphitiResult<()> {
        self.set_with_ttl(key, value, self.config.default_ttl).await
    }

    async fn set_with_ttl(&self, key: &str, value: Vec<u8>, ttl: Duration) -> GraphitiResult<()> {
        let entry = CacheEntry::new(value.clone(), Some(ttl));
        self.cache.insert(key.to_string(), entry).await;
        self.update_stats_set(value.len()).await;
        Ok(())
    }

    async fn remove(&self, key: &str) -> GraphitiResult<()> {
        self.cache.remove(key).await;
        self.update_stats_remove().await;
        Ok(())
    }

    async fn clear(&self) -> GraphitiResult<()> {
        self.cache.invalidate_all();
        self.cache.run_pending_tasks().await;

        let mut stats = self.stats.write().await;
        stats.entries = 0;
        stats.size_bytes = 0;

        Ok(())
    }

    async fn exists(&self, key: &str) -> GraphitiResult<bool> {
        Ok(self.cache.contains_key(key))
    }

    async fn stats(&self) -> GraphitiResult<CacheStats> {
        // Update current stats
        {
            let mut stats = self.stats.write().await;
            stats.entries = self.cache.entry_count();
            stats.size_bytes = self.cache.weighted_size();
        }

        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_memory_cache_basic_operations() {
        let config = CacheConfig::default();
        let cache = MemoryCache::new(config);

        // Test set and get
        let key = "test_key";
        let value = b"test_value".to_vec();

        cache.set(key, value.clone()).await.unwrap();
        let result = cache.get(key).await.unwrap();
        assert_eq!(result, Some(value));

        // Test exists
        assert!(cache.exists(key).await.unwrap());

        // Test remove
        cache.remove(key).await.unwrap();
        assert!(!cache.exists(key).await.unwrap());
        assert_eq!(cache.get(key).await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_memory_cache_ttl() {
        let config = CacheConfig::default();
        let cache = MemoryCache::new(config);

        let key = "ttl_key";
        let value = b"ttl_value".to_vec();
        let short_ttl = Duration::from_millis(50);

        cache
            .set_with_ttl(key, value.clone(), short_ttl)
            .await
            .unwrap();

        // Should exist initially
        assert_eq!(cache.get(key).await.unwrap(), Some(value));

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should be expired
        assert_eq!(cache.get(key).await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_memory_cache_stats() {
        let config = CacheConfig {
            default_ttl: Duration::from_secs(3600),
            max_size: Some(1024),
            cache_dir: None,
            persistent: false,
        };
        let cache = MemoryCache::new(config);

        let stats = cache.stats().await.unwrap();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);

        // Miss
        let miss_result = cache.get("nonexistent").await.unwrap();
        assert!(miss_result.is_none());
        let stats = cache.stats().await.unwrap();
        assert_eq!(stats.misses, 1);

        // Set and hit
        cache.set("key", b"value".to_vec()).await.unwrap();

        // Force synchronization by calling run_pending_tasks
        cache.cache.run_pending_tasks().await;

        let result = cache.get("key").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), b"value".to_vec());

        let stats = cache.stats().await.unwrap();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.entries, 1);
    }
}
