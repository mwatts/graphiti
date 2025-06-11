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

use std::sync::Arc;
use neo4rs::Graph;
use crate::llm_client::LlmClient;
use crate::embedder::EmbedderClient;
use crate::cross_encoder::CrossEncoderClient;
use crate::cache::Cache;

/// Core clients required for Graphiti operations
#[derive(Clone)]
pub struct GraphitiClients {
    pub driver: Arc<Graph>,
    pub llm_client: Arc<dyn LlmClient + Send + Sync>,
    pub embedder: Arc<dyn EmbedderClient + Send + Sync>,
    pub cross_encoder: Arc<dyn CrossEncoderClient + Send + Sync>,
    pub cache: Arc<dyn Cache + Send + Sync>,
}

impl GraphitiClients {
    pub fn new(
        driver: Graph,
        llm_client: Arc<dyn LlmClient + Send + Sync>,
        embedder: Arc<dyn EmbedderClient + Send + Sync>,
        cross_encoder: Arc<dyn CrossEncoderClient + Send + Sync>,
        cache: Arc<dyn Cache + Send + Sync>,
    ) -> Self {
        Self {
            driver: Arc::new(driver),
            llm_client,
            embedder,
            cross_encoder,
            cache,
        }
    }
}

/// Default database name constant
pub const DEFAULT_DATABASE: &str = "neo4j";
