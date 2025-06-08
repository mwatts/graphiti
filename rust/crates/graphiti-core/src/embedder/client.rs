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
use crate::errors::GraphitiResult;

pub const EMBEDDING_DIM: usize = 1024;

#[derive(Debug, Clone)]
pub struct EmbedderConfig {
    pub embedding_dim: usize,
}

impl Default for EmbedderConfig {
    fn default() -> Self {
        Self {
            embedding_dim: EMBEDDING_DIM,
        }
    }
}

/// Trait for embedding text into vector representations
#[async_trait]
pub trait EmbedderClient: Send + Sync {
    /// Create embeddings for a single text input
    async fn create(&self, input_data: &str) -> GraphitiResult<Vec<f32>>;
    
    /// Create embeddings for multiple text inputs
    async fn create_batch(&self, input_data_list: &[String]) -> GraphitiResult<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(input_data_list.len());
        for input in input_data_list {
            let embedding = self.create(input).await?;
            results.push(embedding);
        }
        Ok(results)
    }
}
