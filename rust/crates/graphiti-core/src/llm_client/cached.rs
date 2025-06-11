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

//! Caching wrapper for LLM clients

use async_trait::async_trait;
use std::sync::Arc;
use std::collections::HashMap;
use serde_json::Value;
use crate::{
    cache::{Cache, generate_cache_key},
    llm_client::{LlmClient, models::Message, config::ModelSize},
    errors::LlmResult,
};

/// Wrapper that adds caching to any LlmClient implementation
pub struct CachedLlmClient {
    inner: Arc<dyn LlmClient>,
    cache: Arc<dyn Cache>,
}

impl CachedLlmClient {
    pub fn new(inner: Arc<dyn LlmClient>, cache: Arc<dyn Cache>) -> Self {
        Self { inner, cache }
    }

    /// Generate cache key for LLM request
    fn generate_response_cache_key(
        &self,
        messages: &[Message],
        response_model: Option<&str>,
        max_tokens: Option<u32>,
        model_size: ModelSize,
    ) -> String {
        let messages_str = serde_json::to_string(messages).unwrap_or_default();
        let response_model_str = response_model.unwrap_or("");
        let max_tokens_str = max_tokens.map(|t| t.to_string()).unwrap_or_default();
        let model_size_str = format!("{:?}", model_size);

        let cache_key_str = format!("llm_{}_{}_{}_{}", messages_str, response_model_str, max_tokens_str, model_size_str);
        generate_cache_key(&[&cache_key_str])
    }

    fn generate_chat_cache_key(&self, messages: &[Message], json_params: Option<&Value>) -> String {
        let messages_str = serde_json::to_string(messages).unwrap_or_default();
        let params_str = json_params.map(|p| serde_json::to_string(p).unwrap_or_default()).unwrap_or_default();

        let cache_key_str = format!("chat_{}_{}", messages_str, params_str);
        generate_cache_key(&[&cache_key_str])
    }
}

#[async_trait]
impl LlmClient for CachedLlmClient {
    async fn generate_response(
        &self,
        messages: &[Message],
        response_model: Option<&str>,
        max_tokens: Option<u32>,
        model_size: ModelSize,
    ) -> LlmResult<HashMap<String, Value>> {
        let cache_key = self.generate_response_cache_key(messages, response_model, max_tokens, model_size);

        // Try to get from cache first
        if let Ok(Some(cached_bytes)) = self.cache.get(&cache_key).await {
            if let Ok(cached_response) = serde_json::from_slice::<HashMap<String, Value>>(&cached_bytes) {
                return Ok(cached_response);
            }
        }

        // Not in cache, make the request
        let response = self.inner.generate_response(messages, response_model, max_tokens, model_size).await?;

        // Cache the result
        if let Ok(serialized) = serde_json::to_vec(&response) {
            let _ = self.cache.set(&cache_key, serialized).await;
        }

        Ok(response)
    }

    async fn chat_completion(
        &self,
        messages: &[Message],
        json_params: Option<Value>,
    ) -> LlmResult<Value> {
        let cache_key = self.generate_chat_cache_key(messages, json_params.as_ref());

        // Try to get from cache first
        if let Ok(Some(cached_bytes)) = self.cache.get(&cache_key).await {
            if let Ok(cached_response) = serde_json::from_slice::<Value>(&cached_bytes) {
                return Ok(cached_response);
            }
        }

        // Not in cache, make the request
        let response = self.inner.chat_completion(messages, json_params).await?;

        // Cache the result
        if let Ok(serialized) = serde_json::to_vec(&response) {
            let _ = self.cache.set(&cache_key, serialized).await;
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    // TODO: Add proper tests for CachedLlmClient
    // The tests need to handle lifetime parameters properly
}
