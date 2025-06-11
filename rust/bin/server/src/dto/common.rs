use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use validator::Validate;

/// Common message structure
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Message {
    pub uuid: Uuid,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub role: Option<String>,
    pub role_type: Option<String>,
    pub source_description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Common result structure for operations
#[derive(Debug, Serialize, Deserialize)]
pub struct Result {
    pub message: String,
    pub success: bool,
}
