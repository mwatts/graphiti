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

use thiserror::Error;
use uuid::Uuid;

/// Base error type for Graphiti Core operations
#[derive(Debug, Error)]
pub enum GraphitiError {
    #[error("Database error: {0}")]
    Database(#[from] neo4rs::Error),
    
    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Cache error: {0}")]
    Cache(#[from] sled::Error),
    
    #[error("Edge {uuid} not found")]
    EdgeNotFound { uuid: Uuid },
    
    #[error("None of the edges for {uuids:?} were found")]
    EdgesNotFound { uuids: Vec<Uuid> },
    
    #[error("No edges found for group ids {group_ids:?}")]
    GroupsEdgesNotFound { group_ids: Vec<String> },
    
    #[error("No nodes found for group ids {group_ids:?}")]
    GroupsNodesNotFound { group_ids: Vec<String> },
    
    #[error("Node {uuid} not found")]
    NodeNotFound { uuid: Uuid },
    
    #[error("Search reranker error: {text}")]
    SearchReranker { text: String },
    
    #[error("{entity_type_attribute} cannot be used as an attribute for {entity_type} as it is a protected attribute name")]
    EntityTypeValidation {
        entity_type: String,
        entity_type_attribute: String,
    },
    
    #[error("Configuration error: {message}")]
    Config { message: String },
    
    #[error("Validation error: {message}")]
    Validation { message: String },
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// LLM-specific error types
#[derive(Debug, Error)]
pub enum LlmError {
    #[error("Rate limit exceeded. Please try again later.")]
    RateLimit,
    
    #[error("LLM refused to generate a response: {message}")]
    Refusal { message: String },
    
    #[error("LLM returned an empty response: {message}")]
    EmptyResponse { message: String },
    
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Authentication error: {message}")]
    Authentication { message: String },
    
    #[error("Invalid model configuration: {message}")]
    InvalidConfig { message: String },
    
    #[error("Timeout error: {message}")]
    Timeout { message: String },
    
    #[error("Network error: {message}")]
    NetworkError { message: String },
}

/// Result type alias for Graphiti operations
pub type GraphitiResult<T> = Result<T, GraphitiError>;

/// Result type alias for LLM operations  
pub type LlmResult<T> = Result<T, LlmError>;
