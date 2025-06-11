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
use neo4rs::{Graph, Query};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::errors::GraphitiError;

/// Base trait for all edge types in the graph
#[async_trait]
pub trait Edge: Send + Sync {
    /// Get the UUID of the edge
    fn uuid(&self) -> &str;

    /// Get the group_id of the edge
    fn group_id(&self) -> &str;

    /// Get the source node UUID
    fn source_node_uuid(&self) -> &str;

    /// Get the target node UUID
    fn target_node_uuid(&self) -> &str;

    /// Get the creation timestamp
    fn created_at(&self) -> DateTime<Utc>;

    /// Save the edge to the database
    async fn save(&self, graph: &Graph) -> Result<(), GraphitiError>;

    /// Delete the edge from the database
    async fn delete(&self, graph: &Graph) -> Result<(), GraphitiError>;

    /// Get additional attributes as key-value pairs
    fn attributes(&self) -> HashMap<String, serde_json::Value>;
}

/// Base edge implementation with common fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseEdge {
    pub uuid: String,
    pub group_id: String,
    pub source_node_uuid: String,
    pub target_node_uuid: String,
    pub created_at: DateTime<Utc>,
}

impl BaseEdge {
    pub fn new(
        group_id: String,
        source_node_uuid: String,
        target_node_uuid: String,
    ) -> Self {
        Self {
            uuid: Uuid::new_v4().to_string(),
            group_id,
            source_node_uuid,
            target_node_uuid,
            created_at: Utc::now(),
        }
    }

    pub fn with_uuid(mut self, uuid: String) -> Self {
        self.uuid = uuid;
        self
    }

    pub fn with_created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = created_at;
        self
    }
}

impl PartialEq for BaseEdge {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl std::hash::Hash for BaseEdge {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}

/// Episodic edge represents a connection between an episode and an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicEdge {
    #[serde(flatten)]
    pub base: BaseEdge,
}

impl EpisodicEdge {
    pub fn new(
        group_id: String,
        episode_uuid: String,
        entity_uuid: String,
    ) -> Self {
        Self {
            base: BaseEdge::new(group_id, episode_uuid, entity_uuid),
        }
    }
}

#[async_trait]
impl Edge for EpisodicEdge {
    fn uuid(&self) -> &str {
        &self.base.uuid
    }

    fn group_id(&self) -> &str {
        &self.base.group_id
    }

    fn source_node_uuid(&self) -> &str {
        &self.base.source_node_uuid
    }

    fn target_node_uuid(&self) -> &str {
        &self.base.target_node_uuid
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.base.created_at
    }

    async fn save(&self, graph: &Graph) -> Result<(), GraphitiError> {
        let query = Query::new(
            "MATCH (episode:Episodic {uuid: $episode_uuid})
             MATCH (entity:Entity {uuid: $entity_uuid})
             MERGE (episode)-[r:MENTIONS {uuid: $uuid}]->(entity)
             SET r.group_id = $group_id,
                 r.created_at = $created_at".to_string()
        )
        .param("episode_uuid", self.base.source_node_uuid.clone())
        .param("entity_uuid", self.base.target_node_uuid.clone())
        .param("uuid", self.base.uuid.clone())
        .param("group_id", self.base.group_id.clone())
        .param("created_at", self.base.created_at.timestamp());

        let _ = graph.execute(query).await
            .map_err(|e| GraphitiError::Database(e))?;

        Ok(())
    }

    async fn delete(&self, graph: &Graph) -> Result<(), GraphitiError> {
        let query = Query::new(
            "MATCH ()-[r:MENTIONS {uuid: $uuid}]->()
             DELETE r".to_string()
        ).param("uuid", self.base.uuid.clone());

        let _ = graph.execute(query).await
            .map_err(|e| GraphitiError::Database(e))?;

        Ok(())
    }

    fn attributes(&self) -> HashMap<String, serde_json::Value> {
        HashMap::new()
    }
}

/// Entity edge represents a relationship between two entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityEdge {
    #[serde(flatten)]
    pub base: BaseEdge,
    pub name: String,
    pub fact: String,
    pub episodes: Vec<String>,
    pub expired_at: Option<DateTime<Utc>>,
    pub valid_at: DateTime<Utc>,
    pub invalid_at: Option<DateTime<Utc>>,
}

impl EntityEdge {
    pub fn new(
        group_id: String,
        source_entity_uuid: String,
        target_entity_uuid: String,
        name: String,
        fact: String,
        valid_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEdge::new(group_id, source_entity_uuid, target_entity_uuid),
            name,
            fact,
            episodes: Vec::new(),
            expired_at: None,
            valid_at,
            invalid_at: None,
        }
    }

    pub fn with_episodes(mut self, episodes: Vec<String>) -> Self {
        self.episodes = episodes;
        self
    }

    pub fn with_expired_at(mut self, expired_at: DateTime<Utc>) -> Self {
        self.expired_at = Some(expired_at);
        self
    }

    pub fn with_invalid_at(mut self, invalid_at: DateTime<Utc>) -> Self {
        self.invalid_at = Some(invalid_at);
        self
    }
}

#[async_trait]
impl Edge for EntityEdge {
    fn uuid(&self) -> &str {
        &self.base.uuid
    }

    fn group_id(&self) -> &str {
        &self.base.group_id
    }

    fn source_node_uuid(&self) -> &str {
        &self.base.source_node_uuid
    }

    fn target_node_uuid(&self) -> &str {
        &self.base.target_node_uuid
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.base.created_at
    }

    async fn save(&self, graph: &Graph) -> Result<(), GraphitiError> {
        let query = Query::new(
            "MATCH (source:Entity {uuid: $source_uuid})
             MATCH (target:Entity {uuid: $target_uuid})
             MERGE (source)-[r:RELATES_TO {uuid: $uuid}]->(target)
             SET r.name = $name,
                 r.group_id = $group_id,
                 r.created_at = $created_at,
                 r.fact = $fact,
                 r.episodes = $episodes,
                 r.expired_at = $expired_at,
                 r.valid_at = $valid_at,
                 r.invalid_at = $invalid_at".to_string()
        )
        .param("source_uuid", self.base.source_node_uuid.clone())
        .param("target_uuid", self.base.target_node_uuid.clone())
        .param("uuid", self.base.uuid.clone())
        .param("name", self.name.clone())
        .param("group_id", self.base.group_id.clone())
        .param("created_at", self.base.created_at.timestamp())
        .param("fact", self.fact.clone())
        .param("episodes", self.episodes.clone())
        .param("expired_at", self.expired_at.map(|dt| dt.timestamp()))
        .param("valid_at", self.valid_at.timestamp())
        .param("invalid_at", self.invalid_at.map(|dt| dt.timestamp()));

        let _ = graph.execute(query).await
            .map_err(|e| GraphitiError::Database(e))?;

        Ok(())
    }

    async fn delete(&self, graph: &Graph) -> Result<(), GraphitiError> {
        let query = Query::new(
            "MATCH ()-[r:RELATES_TO {uuid: $uuid}]->()
             DELETE r".to_string()
        ).param("uuid", self.base.uuid.clone());

        let _ = graph.execute(query).await
            .map_err(|e| GraphitiError::Database(e))?;

        Ok(())
    }

    fn attributes(&self) -> HashMap<String, serde_json::Value> {
        let mut attrs = HashMap::new();
        attrs.insert("name".to_string(), serde_json::Value::String(self.name.clone()));
        attrs.insert("fact".to_string(), serde_json::Value::String(self.fact.clone()));
        attrs.insert("episodes".to_string(), serde_json::to_value(&self.episodes).unwrap());
        attrs.insert("valid_at".to_string(), serde_json::Value::String(self.valid_at.to_rfc3339()));

        if let Some(expired_at) = self.expired_at {
            attrs.insert("expired_at".to_string(), serde_json::Value::String(expired_at.to_rfc3339()));
        }

        if let Some(invalid_at) = self.invalid_at {
            attrs.insert("invalid_at".to_string(), serde_json::Value::String(invalid_at.to_rfc3339()));
        }

        attrs
    }
}

/// Community edge represents a membership relationship between an entity and a community
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityEdge {
    #[serde(flatten)]
    pub base: BaseEdge,
}

impl CommunityEdge {
    pub fn new(
        group_id: String,
        entity_uuid: String,
        community_uuid: String,
    ) -> Self {
        Self {
            base: BaseEdge::new(group_id, entity_uuid, community_uuid),
        }
    }
}

#[async_trait]
impl Edge for CommunityEdge {
    fn uuid(&self) -> &str {
        &self.base.uuid
    }

    fn group_id(&self) -> &str {
        &self.base.group_id
    }

    fn source_node_uuid(&self) -> &str {
        &self.base.source_node_uuid
    }

    fn target_node_uuid(&self) -> &str {
        &self.base.target_node_uuid
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.base.created_at
    }

    async fn save(&self, graph: &Graph) -> Result<(), GraphitiError> {
        let query = Query::new(
            "MATCH (entity:Entity {uuid: $entity_uuid})
             MATCH (community:Community {uuid: $community_uuid})
             MERGE (entity)-[r:HAS_MEMBER {uuid: $uuid}]->(community)
             SET r.group_id = $group_id,
                 r.created_at = $created_at".to_string()
        )
        .param("entity_uuid", self.base.source_node_uuid.clone())
        .param("community_uuid", self.base.target_node_uuid.clone())
        .param("uuid", self.base.uuid.clone())
        .param("group_id", self.base.group_id.clone())
        .param("created_at", self.base.created_at.timestamp());

        let _ = graph.execute(query).await
            .map_err(|e| GraphitiError::Database(e))?;

        Ok(())
    }

    async fn delete(&self, graph: &Graph) -> Result<(), GraphitiError> {
        let query = Query::new(
            "MATCH ()-[r:HAS_MEMBER {uuid: $uuid}]->()
             DELETE r".to_string()
        ).param("uuid", self.base.uuid.clone());

        let _ = graph.execute(query).await
            .map_err(|e| GraphitiError::Database(e))?;

        Ok(())
    }

    fn attributes(&self) -> HashMap<String, serde_json::Value> {
        HashMap::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_edge_creation() {
        let edge = BaseEdge::new(
            "group1".to_string(),
            "source-uuid".to_string(),
            "target-uuid".to_string(),
        );

        assert_eq!(edge.group_id, "group1");
        assert_eq!(edge.source_node_uuid, "source-uuid");
        assert_eq!(edge.target_node_uuid, "target-uuid");
        assert!(!edge.uuid.is_empty());
    }

    #[test]
    fn test_entity_edge_creation() {
        let edge = EntityEdge::new(
            "group1".to_string(),
            "entity1".to_string(),
            "entity2".to_string(),
            "relationship".to_string(),
            "entity1 relates to entity2".to_string(),
            Utc::now(),
        );

        assert_eq!(edge.name, "relationship");
        assert_eq!(edge.fact, "entity1 relates to entity2");
        assert_eq!(edge.base.source_node_uuid, "entity1");
        assert_eq!(edge.base.target_node_uuid, "entity2");
    }

    #[test]
    fn test_episodic_edge_creation() {
        let edge = EpisodicEdge::new(
            "group1".to_string(),
            "episode1".to_string(),
            "entity1".to_string(),
        );

        assert_eq!(edge.base.source_node_uuid, "episode1");
        assert_eq!(edge.base.target_node_uuid, "entity1");
    }
}
