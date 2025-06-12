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

//! Caching wrapper for embedder clients

use crate::{
    cache::{generate_cache_key, Cache},
    embedder::client::EmbedderClient,
    errors::GraphitiResult,
};
use async_trait::async_trait;
use std::sync::Arc;

/// Wrapper that adds caching to any EmbedderClient implementation
pub struct CachedEmbedderClient {
    inner: Arc<dyn EmbedderClient>,
    cache: Arc<dyn Cache>,
}

impl CachedEmbedderClient {
    pub fn new(inner: Arc<dyn EmbedderClient>, cache: Arc<dyn Cache>) -> Self {
        Self { inner, cache }
    }
}

#[async_trait]
impl EmbedderClient for CachedEmbedderClient {
    async fn create(&self, input_data: &str) -> GraphitiResult<Vec<f32>> {
        // Generate cache key for the input
        let cache_key = generate_cache_key(&["embedding", input_data]);

        // Try to get from cache first
        if let Ok(Some(cached_bytes)) = self.cache.get(&cache_key).await {
            if let Ok(cached_embedding) = serde_json::from_slice::<Vec<f32>>(&cached_bytes) {
                return Ok(cached_embedding);
            }
        }

        // Not in cache, compute embedding
        let embedding = self.inner.create(input_data).await?;

        // Cache the result
        if let Ok(serialized) = serde_json::to_vec(&embedding) {
            let _ = self.cache.set(&cache_key, serialized).await;
        }

        Ok(embedding)
    }

    async fn embed_query(&self, query: &str) -> GraphitiResult<Vec<f32>> {
        self.create(query).await
    }

    async fn create_batch(&self, input_data_list: &[String]) -> GraphitiResult<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(input_data_list.len());

        for input in input_data_list {
            let embedding = self.create(input).await?;
            results.push(embedding);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::{memory_cache::MemoryCache, CacheConfig};
    use mockall::mock;
    use mockall::predicate::*;
    use std::sync::Arc;

    mock! {
        TestEmbedder {}

        #[async_trait]
        impl EmbedderClient for TestEmbedder {
            async fn create(&self, input_data: &str) -> GraphitiResult<Vec<f32>>;
            async fn embed_query(&self, query: &str) -> GraphitiResult<Vec<f32>>;
            async fn create_batch(&self, input_data_list: &[String]) -> GraphitiResult<Vec<Vec<f32>>>;
        }
    }

    #[tokio::test]
    async fn test_cached_embedder_caches_results() {
        let mut mock_embedder = MockTestEmbedder::new();
        mock_embedder
            .expect_create()
            .with(eq("test input"))
            .times(1)
            .returning(|_| Ok(vec![1.0, 2.0, 3.0]));

        let cache_config = CacheConfig::default();
        let cache = Arc::new(MemoryCache::new(cache_config));
        let cached_embedder = CachedEmbedderClient::new(Arc::new(mock_embedder), cache);

        // First call should hit the embedder
        let result1 = cached_embedder.create("test input").await.unwrap();
        assert_eq!(result1, vec![1.0, 2.0, 3.0]);

        // Second call should hit the cache (mock expects only 1 call)
        let result2 = cached_embedder.create("test input").await.unwrap();
        assert_eq!(result2, vec![1.0, 2.0, 3.0]);
    }
}
