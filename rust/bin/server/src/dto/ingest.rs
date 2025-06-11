use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use super::common::Message;

/// Request to add messages to the graph
#[derive(Debug, Deserialize, Validate)]
pub struct AddMessagesRequest {
    pub group_id: String,
    #[validate(length(min = 1))]
    pub messages: Vec<Message>,
}

/// Request to add an entity node
#[derive(Debug, Deserialize, Validate)]
pub struct AddEntityNodeRequest {
    pub uuid: Uuid,
    pub group_id: String,
    #[validate(length(min = 1))]
    pub name: String,
    pub summary: String,
}
