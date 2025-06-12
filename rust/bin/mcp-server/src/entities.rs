use serde::{Deserialize, Serialize};
use validator::Validate;

/// A Requirement represents a specific need, feature, or functionality that a product or service must fulfill
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Requirement {
    /// The name of the project to which the requirement belongs
    #[validate(length(min = 1))]
    pub project_name: String,

    /// Description of the requirement
    #[validate(length(min = 1))]
    pub description: String,
}

/// A Preference represents a user's expressed like, dislike, or preference for something
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Preference {
    /// The category of the preference (e.g., 'Brands', 'Food', 'Music')
    #[validate(length(min = 1))]
    pub category: String,

    /// Brief description of the preference
    #[validate(length(min = 1))]
    pub description: String,
}

/// A Procedure informing the agent what actions to take or how to perform in certain scenarios
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Procedure {
    /// Brief description of the procedure
    #[validate(length(min = 1))]
    pub description: String,
}

/// Enum representing all supported entity types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EntityType {
    Requirement(Requirement),
    Preference(Preference),
    Procedure(Procedure),
}

impl EntityType {
    /// Get the type name as a string
    pub fn type_name(&self) -> &'static str {
        match self {
            EntityType::Requirement(_) => "Requirement",
            EntityType::Preference(_) => "Preference",
            EntityType::Procedure(_) => "Procedure",
        }
    }
}

/// Helper function to get all entity type names
pub fn get_entity_type_names() -> Vec<&'static str> {
    vec!["Requirement", "Preference", "Procedure"]
}

/// Get all entity types as a HashMap for Graphiti
pub fn get_entity_types() -> std::collections::HashMap<String, Box<dyn std::any::Any + Send + Sync>>
{
    let mut entity_types = std::collections::HashMap::new();

    // Add the entity types that match the Python implementation
    entity_types.insert(
        "Requirement".to_string(),
        Box::new(()) as Box<dyn std::any::Any + Send + Sync>,
    );
    entity_types.insert(
        "Preference".to_string(),
        Box::new(()) as Box<dyn std::any::Any + Send + Sync>,
    );
    entity_types.insert(
        "Procedure".to_string(),
        Box::new(()) as Box<dyn std::any::Any + Send + Sync>,
    );

    entity_types
}
