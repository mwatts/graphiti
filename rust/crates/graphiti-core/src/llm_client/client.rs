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

use async_trait::async_trait;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration;
use tokio_retry::{strategy::ExponentialBackoff, RetryIf};
use tracing::{debug, warn};

use super::config::{LlmConfig, ModelSize};
use super::models::Message;
use crate::errors::{LlmError, LlmResult};

const DEFAULT_CACHE_DIR: &str = "./llm_cache";
const MULTILINGUAL_EXTRACTION_RESPONSES: &str =
    "\n\nAny extracted information should be returned in the same language as it was written in.";

/// Trait for LLM clients that can generate responses
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Generate a response from the LLM
    async fn generate_response(
        &self,
        messages: &[Message],
        response_model: Option<&str>, // JSON schema as string
        max_tokens: Option<u32>,
        model_size: ModelSize,
    ) -> LlmResult<HashMap<String, Value>>;

    /// Chat completion method (for compatibility with cross encoder)
    async fn chat_completion(
        &self,
        messages: &[Message],
        json_params: Option<Value>,
    ) -> LlmResult<Value>;

    /// Generate a simple text response
    async fn generate_text(&self, messages: &[Message]) -> LlmResult<String> {
        let response = self
            .generate_response(messages, None, None, ModelSize::Medium)
            .await?;
        response
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| LlmError::EmptyResponse {
                message: "No content field in response".to_string(),
            })
    }
}

/// Base implementation for LLM clients with caching and retry logic
pub struct BaseLlmClient {
    pub config: LlmConfig,
    pub cache_enabled: bool,
    pub cache: Option<sled::Db>,
}

impl BaseLlmClient {
    pub fn new(config: LlmConfig, cache_enabled: bool) -> LlmResult<Self> {
        let cache = if cache_enabled {
            Some(
                sled::open(DEFAULT_CACHE_DIR).map_err(|e| LlmError::InvalidConfig {
                    message: format!("Failed to open cache: {}", e),
                })?,
            )
        } else {
            None
        };

        Ok(Self {
            config,
            cache_enabled,
            cache,
        })
    }

    /// Clean input string of invalid unicode and control characters
    pub fn clean_input(&self, input: &str) -> String {
        // Remove zero-width characters and other invisible unicode
        let zero_width_chars = ['\u{200b}', '\u{200c}', '\u{200d}', '\u{feff}', '\u{2060}'];
        let mut cleaned = input.to_string();
        for char in zero_width_chars {
            cleaned = cleaned.replace(char, "");
        }

        // Remove control characters except newlines, returns, and tabs
        cleaned
            .chars()
            .filter(|&c| (c as u32) >= 32 || c == '\n' || c == '\r' || c == '\t')
            .collect()
    }

    /// Generate cache key for messages
    pub fn get_cache_key(&self, messages: &[Message]) -> String {
        let messages_json = serde_json::to_string(messages).unwrap_or_default();
        let key_string = format!(
            "{}:{}",
            self.config.model.as_deref().unwrap_or("default"),
            messages_json
        );

        let mut hasher = Sha256::new();
        hasher.update(key_string.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Check cache for response
    pub async fn get_cached_response(&self, cache_key: &str) -> Option<HashMap<String, Value>> {
        if let Some(cache) = &self.cache {
            if let Ok(Some(cached_data)) = cache.get(cache_key) {
                if let Ok(response) = serde_json::from_slice::<HashMap<String, Value>>(&cached_data)
                {
                    debug!("Cache hit for {}", cache_key);
                    return Some(response);
                }
            }
        }
        None
    }

    /// Store response in cache
    pub async fn cache_response(&self, cache_key: &str, response: &HashMap<String, Value>) {
        if let Some(cache) = &self.cache {
            if let Ok(data) = serde_json::to_vec(response) {
                let _ = cache.insert(cache_key, data);
                let _ = cache.flush_async().await;
            }
        }
    }

    /// Prepare messages for LLM call
    pub fn prepare_messages(
        &self,
        mut messages: Vec<Message>,
        response_model: Option<&str>,
    ) -> Vec<Message> {
        // Add response model schema if provided
        if let Some(schema) = response_model {
            if let Some(last_msg) = messages.last_mut() {
                last_msg.content.push_str(&format!(
                    "\n\nRespond with a JSON object in the following format:\n\n{}",
                    schema
                ));
            }
        }

        // Add multilingual extraction instructions
        if let Some(first_msg) = messages.first_mut() {
            first_msg
                .content
                .push_str(MULTILINGUAL_EXTRACTION_RESPONSES);
        }

        // Clean all message content
        for message in &mut messages {
            message.content = self.clean_input(&message.content);
        }

        messages
    }

    /// Execute with retry logic
    pub async fn execute_with_retry<F, Fut, T>(&self, operation: F) -> LlmResult<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = LlmResult<T>>,
    {
        let retry_strategy = ExponentialBackoff::from_millis(5000)
            .max_delay(Duration::from_secs(120))
            .take(4);

        let retry_condition = |error: &LlmError| {
            matches!(
                error,
                LlmError::RateLimit | LlmError::Http(_) | LlmError::Timeout { .. }
            )
        };

        RetryIf::spawn(retry_strategy, operation, retry_condition)
            .await
            .map_err(|e| {
                warn!("All retry attempts exhausted: {:?}", e);
                e
            })
    }
}
