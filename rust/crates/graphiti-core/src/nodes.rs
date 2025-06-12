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
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    errors::GraphitiError,
    database::{GraphDatabase, QueryParameter},
};

/// Enumeration of different types of episodes that can be processed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EpisodeType {
    /// Represents a standard message-type episode. The content should be
    /// formatted as "actor: content". For example, "user: Hello, how are you?".to_string()
    Message,
    /// Represents an episode containing a JSON string object with structured data.
    Json,
    /// Represents a plain text episode.
    Text,
}

impl EpisodeType {
    pub fn from_str(s: &str) -> Result<Self, GraphitiError> {
        match s.to_lowercase().as_str() {
            "message" => Ok(EpisodeType::Message),
            "json" => Ok(EpisodeType::Json),
            "text" => Ok(EpisodeType::Text),
            _ => Err(GraphitiError::InvalidInput(format!(
                "Episode type: {} not implemented",
                s
            ))),
        }
    }
}

/// Base trait for all node types in the graph
#[async_trait]
pub trait Node: Send + Sync {
    /// Get the UUID of the node
    fn uuid(&self) -> &str;

    /// Get the name of the node
    fn name(&self) -> &str;

    /// Get the group_id of the node
    fn group_id(&self) -> &str;

    /// Get the labels of the node
    fn labels(&self) -> &[String];

    /// Get the creation timestamp
    fn created_at(&self) -> DateTime<Utc>;

    /// Save the node to the database
    async fn save(&self, database: &dyn GraphDatabase) -> Result<(), GraphitiError>;

    /// Delete the node from the database
    async fn delete(&self, database: &dyn GraphDatabase) -> Result<(), GraphitiError>;

    /// Get additional attributes as key-value pairs
    fn attributes(&self) -> HashMap<String, serde_json::Value>;
}

/// Base node implementation with common fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseNode {
    pub uuid: String,
    pub name: String,
    pub group_id: String,
    pub labels: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl BaseNode {
    pub fn new(name: String, group_id: String) -> Self {
        Self {
            uuid: Uuid::new_v4().to_string(),
            name,
            group_id,
            labels: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_uuid(mut self, uuid: String) -> Self {
        self.uuid = uuid;
        self
    }

    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    pub fn with_created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = created_at;
        self
    }

    /// Delete node by group_id
    pub async fn delete_by_group_id(database: &dyn GraphDatabase, group_id: &str) -> Result<(), GraphitiError> {
        database.delete_by_group_id(group_id).await?;
        Ok(())
    }
}

impl PartialEq for BaseNode {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl std::hash::Hash for BaseNode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}

/// Episodic node represents a specific episode or event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicNode {
    #[serde(flatten)]
    pub base: BaseNode,
    pub source: EpisodeType,
    pub source_description: String,
    pub content: String,
    pub valid_at: DateTime<Utc>,
    pub entity_edges: Vec<String>,
}

impl EpisodicNode {
    pub fn new(
        name: String,
        group_id: String,
        source: EpisodeType,
        source_description: String,
        content: String,
        valid_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseNode::new(name, group_id),
            source,
            source_description,
            content,
            valid_at,
            entity_edges: Vec::new(),
        }
    }
}

#[async_trait]
impl Node for EpisodicNode {
    fn uuid(&self) -> &str {
        &self.base.uuid
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn group_id(&self) -> &str {
        &self.base.group_id
    }

    fn labels(&self) -> &[String] {
        &self.base.labels
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.base.created_at
    }

    async fn save(&self, database: &dyn GraphDatabase) -> Result<(), GraphitiError> {
        use crate::database::traits::QueryParameter;
        
        // Convert node attributes to database parameters
        let mut properties = HashMap::new();
        properties.insert("uuid".to_string(), QueryParameter::String(self.base.uuid.clone()));
        properties.insert("name".to_string(), QueryParameter::String(self.base.name.clone()));
        properties.insert("group_id".to_string(), QueryParameter::String(self.base.group_id.clone()));
        properties.insert("created_at".to_string(), QueryParameter::String(self.base.created_at.to_rfc3339()));
        
        // Convert EpisodeType to string
        let source_str = match self.source {
            EpisodeType::Message => "message",
            EpisodeType::Json => "json", 
            EpisodeType::Text => "text",
        };
        properties.insert("source".to_string(), QueryParameter::String(source_str.to_string()));
        properties.insert("source_description".to_string(), QueryParameter::String(self.source_description.clone()));
        properties.insert("content".to_string(), QueryParameter::String(self.content.clone()));
        properties.insert("valid_at".to_string(), QueryParameter::String(self.valid_at.to_rfc3339()));
        
        // Convert entity_edges to string representation
        let entity_edges_json = serde_json::to_string(&self.entity_edges).unwrap_or_default();
        properties.insert("entity_edges".to_string(), QueryParameter::String(entity_edges_json));

        // Check if node exists, then create or update
        if let Some(_existing) = database.get_node(&self.base.uuid).await.map_err(|e| GraphitiError::DatabaseLayer(e))? {
            database.update_node(&self.base.uuid, properties).await.map_err(|e| GraphitiError::DatabaseLayer(e))?;
        } else {
            database.create_node(self.base.labels.clone(), properties).await.map_err(|e| GraphitiError::DatabaseLayer(e))?;
        }
        
        Ok(())
    }

    async fn delete(&self, database: &dyn GraphDatabase) -> Result<(), GraphitiError> {
        database.delete_node(&self.base.uuid).await.map_err(|e| GraphitiError::DatabaseLayer(e))?;
        Ok(())
    }

    fn attributes(&self) -> HashMap<String, serde_json::Value> {
        let mut attrs = HashMap::new();
        attrs.insert("source".to_string(), serde_json::to_value(&self.source).unwrap());
        attrs.insert("source_description".to_string(), serde_json::Value::String(self.source_description.clone()));
        attrs.insert("content".to_string(), serde_json::Value::String(self.content.clone()));
        attrs.insert("valid_at".to_string(), serde_json::Value::String(self.valid_at.to_rfc3339()));
        attrs.insert("entity_edges".to_string(), serde_json::to_value(&self.entity_edges).unwrap());
        attrs
    }
}

/// Entity node represents a person, place, thing, or concept
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityNode {
    #[serde(flatten)]
    pub base: BaseNode,
    pub summary: String,
    pub summary_embedding: Option<Vec<f64>>,
}

impl EntityNode {
    pub fn new(name: String, group_id: String, summary: String) -> Self {
        Self {
            base: BaseNode::new(name, group_id),
            summary,
            summary_embedding: None,
        }
    }

    pub fn with_summary_embedding(mut self, embedding: Vec<f64>) -> Self {
        self.summary_embedding = Some(embedding);
        self
    }
}

#[async_trait]
impl Node for EntityNode {
    fn uuid(&self) -> &str {
        &self.base.uuid
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn group_id(&self) -> &str {
        &self.base.group_id
    }

    fn labels(&self) -> &[String] {
        &self.base.labels
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.base.created_at
    }

    async fn save(&self, database: &dyn GraphDatabase) -> Result<(), GraphitiError> {
        use crate::database::traits::QueryParameter;
        
        // Convert node attributes to database parameters
        let mut properties = HashMap::new();
        properties.insert("uuid".to_string(), QueryParameter::String(self.base.uuid.clone()));
        properties.insert("name".to_string(), QueryParameter::String(self.base.name.clone()));
        properties.insert("group_id".to_string(), QueryParameter::String(self.base.group_id.clone()));
        properties.insert("created_at".to_string(), QueryParameter::String(self.base.created_at.to_rfc3339()));
        properties.insert("summary".to_string(), QueryParameter::String(self.summary.clone()));
        
        // Handle optional summary_embedding
        if let Some(ref embedding) = self.summary_embedding {
            let embedding_json = serde_json::to_string(embedding).unwrap_or_default();
            properties.insert("summary_embedding".to_string(), QueryParameter::String(embedding_json));
        }

        // Check if node exists, then create or update
        if let Some(_existing) = database.get_node(&self.base.uuid).await.map_err(|e| GraphitiError::DatabaseLayer(e))? {
            database.update_node(&self.base.uuid, properties).await.map_err(|e| GraphitiError::DatabaseLayer(e))?;
        } else {
            database.create_node(self.base.labels.clone(), properties).await.map_err(|e| GraphitiError::DatabaseLayer(e))?;
        }
        
        Ok(())
    }

    async fn delete(&self, database: &dyn GraphDatabase) -> Result<(), GraphitiError> {
        database.delete_node(&self.base.uuid).await.map_err(|e| GraphitiError::DatabaseLayer(e))?;
        Ok(())
    }

    fn attributes(&self) -> HashMap<String, serde_json::Value> {
        let mut attrs = HashMap::new();
        attrs.insert("summary".to_string(), serde_json::Value::String(self.summary.clone()));
        if let Some(ref embedding) = self.summary_embedding {
            attrs.insert("summary_embedding".to_string(), serde_json::to_value(embedding).unwrap());
        }
        attrs
    }
}

/// Community node represents a cluster or group of related entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityNode {
    #[serde(flatten)]
    pub base: BaseNode,
    pub summary: String,
    pub summary_embedding: Option<Vec<f64>>,
}

impl CommunityNode {
    pub fn new(name: String, group_id: String, summary: String) -> Self {
        Self {
            base: BaseNode::new(name, group_id),
            summary,
            summary_embedding: None,
        }
    }

    pub fn with_summary_embedding(mut self, embedding: Vec<f64>) -> Self {
        self.summary_embedding = Some(embedding);
        self
    }
}

#[async_trait]
impl Node for CommunityNode {
    fn uuid(&self) -> &str {
        &self.base.uuid
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn group_id(&self) -> &str {
        &self.base.group_id
    }

    fn labels(&self) -> &[String] {
        &self.base.labels
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.base.created_at
    }

    async fn save(&self, database: &dyn GraphDatabase) -> Result<(), GraphitiError> {
        use crate::database::traits::QueryParameter;
        
        // Convert node attributes to database parameters
        let mut properties = HashMap::new();
        properties.insert("uuid".to_string(), QueryParameter::String(self.base.uuid.clone()));
        properties.insert("name".to_string(), QueryParameter::String(self.base.name.clone()));
        properties.insert("group_id".to_string(), QueryParameter::String(self.base.group_id.clone()));
        properties.insert("created_at".to_string(), QueryParameter::String(self.base.created_at.to_rfc3339()));
        properties.insert("summary".to_string(), QueryParameter::String(self.summary.clone()));
        
        // Handle optional summary_embedding
        if let Some(ref embedding) = self.summary_embedding {
            let embedding_json = serde_json::to_string(embedding).unwrap_or_default();
            properties.insert("summary_embedding".to_string(), QueryParameter::String(embedding_json));
        }

        // Check if node exists, then create or update
        if let Some(_existing) = database.get_node(&self.base.uuid).await.map_err(|e| GraphitiError::DatabaseLayer(e))? {
            database.update_node(&self.base.uuid, properties).await.map_err(|e| GraphitiError::DatabaseLayer(e))?;
        } else {
            database.create_node(self.base.labels.clone(), properties).await.map_err(|e| GraphitiError::DatabaseLayer(e))?;
        }
        
        Ok(())
    }

    async fn delete(&self, database: &dyn GraphDatabase) -> Result<(), GraphitiError> {
        database.delete_node(&self.base.uuid).await.map_err(|e| GraphitiError::DatabaseLayer(e))?;
        Ok(())
    }

    fn attributes(&self) -> HashMap<String, serde_json::Value> {
        let mut attrs = HashMap::new();
        attrs.insert("summary".to_string(), serde_json::Value::String(self.summary.clone()));
        if let Some(ref embedding) = self.summary_embedding {
            attrs.insert("summary_embedding".to_string(), serde_json::to_value(embedding).unwrap());
        }
        attrs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episode_type_from_str() {
        assert_eq!(EpisodeType::from_str("message").unwrap(), EpisodeType::Message);
        assert_eq!(EpisodeType::from_str("json").unwrap(), EpisodeType::Json);
        assert_eq!(EpisodeType::from_str("text").unwrap(), EpisodeType::Text);
        assert!(EpisodeType::from_str("invalid").is_err());
    }

    #[test]
    fn test_base_node_creation() {
        let node = BaseNode::new("Test Node".to_string(), "group1".to_string());
        assert_eq!(node.name, "Test Node");
        assert_eq!(node.group_id, "group1");
        assert!(!node.uuid.is_empty());
    }

    #[test]
    fn test_episodic_node_creation() {
        let node = EpisodicNode::new(
            "Episode 1".to_string(),
            "group1".to_string(),
            EpisodeType::Text,
            "Test source".to_string(),
            "Test content".to_string(),
            Utc::now(),
        );

        assert_eq!(node.name(), "Episode 1");
        assert_eq!(node.source, EpisodeType::Text);
        assert_eq!(node.content, "Test content");
    }
}
