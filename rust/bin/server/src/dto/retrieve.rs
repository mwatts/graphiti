use serde::{Deserialize, Serialize};
use validator::Validate;

use super::common::Message;

/// Search query request
#[derive(Debug, Deserialize, Validate)]
pub struct SearchQuery {
    #[validate(length(min = 1))]
    pub query: String,
    pub group_ids: Vec<String>,
    #[serde(default = "default_max_facts")]
    pub max_facts: usize,
}

fn default_max_facts() -> usize {
    10
}

/// Search results response
#[derive(Debug, Serialize)]
pub struct SearchResults {
    pub facts: Vec<FactResult>,
}

/// Individual fact result
#[derive(Debug, Serialize, Deserialize)]
pub struct FactResult {
    pub fact: String,
    pub uuid: String,
    pub valid_at: Option<String>,
    pub invalid_at: Option<String>,
    pub source_description: String,
    pub episodes: Vec<String>,
}

/// Request to get memory from messages
#[derive(Debug, Deserialize, Validate)]
pub struct GetMemoryRequest {
    pub group_id: String,
    #[validate(length(min = 1))]
    pub messages: Vec<Message>,
    #[serde(default = "default_max_facts")]
    pub max_facts: usize,
}

/// Response with memory facts
#[derive(Debug, Serialize)]
pub struct GetMemoryResponse {
    pub facts: Vec<FactResult>,
}
