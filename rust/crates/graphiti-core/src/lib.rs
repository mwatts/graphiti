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

//! # Graphiti Core
//!
//! A temporal graph building library for AI agents.
//!
//! This crate provides the core functionality for building and managing
//! temporal graphs with nodes and edges that can evolve over time.

pub mod cross_encoder;
pub mod edges;
pub mod embedder;
pub mod errors;
pub mod llm_client;
pub mod nodes;
pub mod types;

// Re-export commonly used types
pub use errors::{GraphitiError, LlmError};
pub use types::GraphitiClients;

// Re-export traits
pub use cross_encoder::CrossEncoderClient;
pub use edges::Edge;
pub use embedder::EmbedderClient;
pub use llm_client::LlmClient;
pub use nodes::Node;

// Re-export concrete types
pub use cross_encoder::OpenAIRerankerClient;
pub use edges::{BaseEdge, CommunityEdge, EntityEdge, EpisodicEdge};
pub use embedder::OpenAiEmbedder;
pub use llm_client::{
    config::LlmConfig,
    models::{Message, TokenUsage},
    openai_client::OpenAiClient,
};
pub use nodes::{BaseNode, CommunityNode, EntityNode, EpisodicNode, EpisodeType};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exports() {
        // This test ensures that all the main exports are available
        // and can be used together
        let _config = LlmConfig::default();
        let _episode_type = EpisodeType::Text;
    }
}
