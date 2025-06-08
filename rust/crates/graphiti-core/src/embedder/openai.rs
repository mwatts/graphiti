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
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::errors::{GraphitiError, GraphitiResult};
use super::client::{EmbedderClient, EmbedderConfig};

const DEFAULT_EMBEDDING_MODEL: &str = "text-embedding-3-small";
const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

#[derive(Debug, Clone)]
pub struct OpenAiEmbedderConfig {
    pub base: EmbedderConfig,
    pub embedding_model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

impl Default for OpenAiEmbedderConfig {
    fn default() -> Self {
        Self {
            base: EmbedderConfig::default(),
            embedding_model: DEFAULT_EMBEDDING_MODEL.to_string(),
            api_key: None,
            base_url: None,
        }
    }
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

pub struct OpenAiEmbedder {
    config: OpenAiEmbedderConfig,
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenAiEmbedder {
    pub fn new(config: OpenAiEmbedderConfig) -> GraphitiResult<Self> {
        let api_key = config.api_key.clone()
            .ok_or_else(|| GraphitiError::Config {
                message: "OpenAI API key is required".to_string(),
            })?;
            
        let base_url = config.base_url.clone()
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
            
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| GraphitiError::Config {
                message: format!("Failed to create HTTP client: {}", e),
            })?;
            
        Ok(Self {
            config,
            client,
            api_key,
            base_url,
        })
    }
    
    async fn create_embeddings_request(&self, input: Vec<String>) -> GraphitiResult<Vec<Vec<f32>>> {
        let request = EmbeddingRequest {
            input,
            model: self.config.embedding_model.clone(),
        };
        
        let url = format!("{}/embeddings", self.base_url);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(GraphitiError::Http)?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(GraphitiError::Config {
                message: format!("OpenAI API error {}: {}", status, error_text),
            });
        }
        
        let embedding_response: EmbeddingResponse = response.json().await
            .map_err(GraphitiError::Http)?;
            
        let embeddings = embedding_response.data
            .into_iter()
            .map(|data| {
                let max_dim = self.config.base.embedding_dim.min(data.embedding.len());
                data.embedding[..max_dim].to_vec()
            })
            .collect();
            
        Ok(embeddings)
    }
}

#[async_trait]
impl EmbedderClient for OpenAiEmbedder {
    async fn create(&self, input_data: &str) -> GraphitiResult<Vec<f32>> {
        let embeddings = self.create_embeddings_request(vec![input_data.to_string()]).await?;
        embeddings.into_iter().next()
            .ok_or_else(|| GraphitiError::Config {
                message: "No embeddings returned from OpenAI".to_string(),
            })
    }
    
    async fn create_batch(&self, input_data_list: &[String]) -> GraphitiResult<Vec<Vec<f32>>> {
        self.create_embeddings_request(input_data_list.to_vec()).await
    }
}
