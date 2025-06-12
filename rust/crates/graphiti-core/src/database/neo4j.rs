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

//! Neo4j database implementation

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use neo4rs::{Graph, ConfigBuilder, BoltType, BoltMap, BoltList, Node, Relation, Txn};
use uuid::Uuid;

use super::traits::{GraphDatabase, Transaction, QueryResult, QueryParameter, NodeData, EdgeData};
use super::config::DatabaseConfig;
use super::types::{DatabaseResult, DatabaseError};

/// Neo4j database implementation
#[derive(Debug)]
pub struct Neo4jDatabase {
    graph: Arc<Graph>,
}

/// Neo4j transaction wrapper
pub struct Neo4jTransaction {
    txn: Txn,
}

impl Neo4jDatabase {
    /// Create a new Neo4j database connection
    pub async fn new(config: DatabaseConfig) -> DatabaseResult<Self> {
        let mut builder = ConfigBuilder::default()
            .uri(&config.uri);

        if let Some(username) = &config.username {
            builder = builder.user(username);
        }

        if let Some(password) = &config.password {
            builder = builder.password(password);
        }

        if let Some(database) = &config.database {
            builder = builder.db(database);
        }

        let graph = Graph::connect(builder.build()?).await?;

        Ok(Self {
            graph: Arc::new(graph),
        })
    }

    /// Get the underlying Neo4j Graph for backward compatibility
    /// This should be temporary until all code is migrated to use the database abstraction
    pub fn get_graph(&self) -> &Graph {
        &self.graph
    }

    /// Convert QueryParameter to BoltType for Neo4j
    fn param_to_bolt(param: &QueryParameter) -> DatabaseResult<BoltType> {
        match param {
            QueryParameter::String(s) => Ok(BoltType::String(neo4rs::BoltString::new(s))),
            QueryParameter::Integer(i) => Ok(BoltType::Integer(neo4rs::BoltInteger::new(*i))),
            QueryParameter::Float(f) => Ok(BoltType::Float(neo4rs::BoltFloat::new(*f))),
            QueryParameter::Boolean(b) => Ok(BoltType::Boolean(neo4rs::BoltBoolean::new(*b))),
            QueryParameter::Null => Ok(BoltType::Null(neo4rs::BoltNull)),
            QueryParameter::List(list) => {
                let mut bolt_list = BoltList::new();
                for item in list {
                    bolt_list.push(Self::param_to_bolt(item)?);
                }
                Ok(BoltType::List(bolt_list))
            }
            QueryParameter::Map(map) => {
                let mut bolt_map = BoltMap::new();
                for (key, value) in map {
                    bolt_map.put(neo4rs::BoltString::new(key), Self::param_to_bolt(value)?);
                }
                Ok(BoltType::Map(bolt_map))
            }
        }
    }

    /// Convert BoltType to QueryParameter
    fn bolt_to_param(bolt: &BoltType) -> DatabaseResult<QueryParameter> {
        match bolt {
            BoltType::String(s) => Ok(QueryParameter::String(s.value().to_string())),
            BoltType::Integer(i) => Ok(QueryParameter::Integer(i.value())),
            BoltType::Float(f) => Ok(QueryParameter::Float(f.value())),
            BoltType::Boolean(b) => Ok(QueryParameter::Boolean(b.value())),
            BoltType::Null(_) => Ok(QueryParameter::Null),
            BoltType::List(list) => {
                let mut result = Vec::new();
                for item in list.value() {
                    result.push(Self::bolt_to_param(item)?);
                }
                Ok(QueryParameter::List(result))
            }
            BoltType::Map(map) => {
                let mut result = HashMap::new();
                for (key, value) in map.value() {
                    if let BoltType::String(key_str) = key {
                        result.insert(key_str.value().to_string(), Self::bolt_to_param(value)?);
                    }
                }
                Ok(QueryParameter::Map(result))
            }
            _ => Ok(QueryParameter::String(format!("{:?}", bolt))),
        }
    }

    /// Convert Neo4j Node to NodeData
    fn node_to_data(node: &Node) -> DatabaseResult<NodeData> {
        let mut properties = HashMap::new();
        for (key, value) in node.props() {
            properties.insert(key.clone(), Self::bolt_to_param(value)?);
        }

        Ok(NodeData {
            id: node.id().to_string(),
            labels: node.labels().to_vec(),
            properties,
        })
    }

    /// Convert Neo4j Relation to EdgeData
    fn relation_to_data(rel: &Relation) -> DatabaseResult<EdgeData> {
        let mut properties = HashMap::new();
        for (key, value) in rel.props() {
            properties.insert(key.clone(), Self::bolt_to_param(value)?);
        }

        Ok(EdgeData {
            id: rel.id().to_string(),
            relationship_type: rel.typ().to_string(),
            source_id: rel.start_node_id().to_string(),
            target_id: rel.end_node_id().to_string(),
            properties,
        })
    }

    /// Execute a query and convert results
    async fn execute_query(&self, query: &str, parameters: HashMap<String, QueryParameter>) -> DatabaseResult<QueryResult> {
        // For now, provide a basic implementation that returns empty results
        // This will be improved in subsequent iterations
        let _ = query;
        let _ = parameters;

        Ok(QueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
        })
    }
}

#[async_trait]
impl GraphDatabase for Neo4jDatabase {
    async fn execute(&self, query: &str, parameters: HashMap<String, QueryParameter>) -> DatabaseResult<QueryResult> {
        self.execute_query(query, parameters).await
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn begin_transaction(&self) -> DatabaseResult<Box<dyn Transaction>> {
        let txn = self.graph.start_txn().await?;
        Ok(Box::new(Neo4jTransaction { txn }))
    }

    async fn close(&self) -> DatabaseResult<()> {
        // Neo4j graph is automatically cleaned up when dropped
        Ok(())
    }

    async fn health_check(&self) -> DatabaseResult<bool> {
        let result = self.execute("RETURN 1 as health", HashMap::new()).await?;
        Ok(!result.rows.is_empty())
    }

    async fn create_node(&self, labels: Vec<String>, properties: HashMap<String, QueryParameter>) -> DatabaseResult<String> {
        let uuid = Uuid::new_v4().to_string();
        let labels_str = labels.join(":");
        let query = format!("CREATE (n:{}) SET n.uuid = $uuid SET n += $props RETURN n.uuid as id", labels_str);

        let mut params = HashMap::new();
        params.insert("uuid".to_string(), QueryParameter::String(uuid.clone()));
        params.insert("props".to_string(), QueryParameter::Map(properties));

        self.execute(&query, params).await?;
        Ok(uuid)
    }

    async fn get_node(&self, id: &str) -> DatabaseResult<Option<NodeData>> {
        let query = "MATCH (n {uuid: $id}) RETURN n";
        let mut params = HashMap::new();
        params.insert("id".to_string(), QueryParameter::String(id.to_string()));

        let result = self.execute(query, params).await?;
        if let Some(row) = result.rows.first() {
            if let Some(QueryParameter::Map(_)) = row.get("n") {
                // For simplicity, we'll need to reconstruct the node data
                // In a real implementation, you'd want to parse the actual node structure
                return Ok(Some(NodeData {
                    id: id.to_string(),
                    labels: vec![], // Would need to extract from actual node
                    properties: HashMap::new(), // Would need to extract from actual node
                }));
            }
        }
        Ok(None)
    }

    async fn update_node(&self, id: &str, properties: HashMap<String, QueryParameter>) -> DatabaseResult<()> {
        let query = "MATCH (n {uuid: $id}) SET n += $props";
        let mut params = HashMap::new();
        params.insert("id".to_string(), QueryParameter::String(id.to_string()));
        params.insert("props".to_string(), QueryParameter::Map(properties));

        self.execute(query, params).await?;
        Ok(())
    }

    async fn delete_node(&self, id: &str) -> DatabaseResult<()> {
        let query = "MATCH (n {uuid: $id}) DETACH DELETE n";
        let mut params = HashMap::new();
        params.insert("id".to_string(), QueryParameter::String(id.to_string()));

        self.execute(query, params).await?;
        Ok(())
    }

    async fn find_nodes(&self, label: Option<&str>, properties: HashMap<String, QueryParameter>) -> DatabaseResult<Vec<NodeData>> {
        let label_part = if let Some(l) = label {
            format!(":{}", l)
        } else {
            String::new()
        };

        let query = format!("MATCH (n{}) WHERE all(key in keys($props) WHERE n[key] = $props[key]) RETURN n", label_part);
        let mut params = HashMap::new();
        params.insert("props".to_string(), QueryParameter::Map(properties));

        let result = self.execute(&query, params).await?;
        // For simplicity, returning empty vector - would need full implementation
        Ok(Vec::new())
    }

    async fn create_edge(&self, source_id: &str, target_id: &str, edge_type: &str, properties: HashMap<String, QueryParameter>) -> DatabaseResult<String> {
        let uuid = Uuid::new_v4().to_string();
        let query = format!(
            "MATCH (a {{uuid: $source_id}}), (b {{uuid: $target_id}}) CREATE (a)-[r:{} {{uuid: $uuid}}]->(b) SET r += $props RETURN r.uuid as id",
            edge_type
        );

        let mut params = HashMap::new();
        params.insert("source_id".to_string(), QueryParameter::String(source_id.to_string()));
        params.insert("target_id".to_string(), QueryParameter::String(target_id.to_string()));
        params.insert("uuid".to_string(), QueryParameter::String(uuid.clone()));
        params.insert("props".to_string(), QueryParameter::Map(properties));

        self.execute(&query, params).await?;
        Ok(uuid)
    }

    async fn get_edge(&self, id: &str) -> DatabaseResult<Option<EdgeData>> {
        let query = "MATCH ()-[r {uuid: $id}]->() RETURN r";
        let mut params = HashMap::new();
        params.insert("id".to_string(), QueryParameter::String(id.to_string()));

        let result = self.execute(query, params).await?;
        // Simplified implementation - would need full edge data extraction
        Ok(None)
    }

    async fn update_edge(&self, id: &str, properties: HashMap<String, QueryParameter>) -> DatabaseResult<()> {
        let query = "MATCH ()-[r {uuid: $id}]->() SET r += $props";
        let mut params = HashMap::new();
        params.insert("id".to_string(), QueryParameter::String(id.to_string()));
        params.insert("props".to_string(), QueryParameter::Map(properties));

        self.execute(query, params).await?;
        Ok(())
    }

    async fn delete_edge(&self, id: &str) -> DatabaseResult<()> {
        let query = "MATCH ()-[r {uuid: $id}]->() DELETE r";
        let mut params = HashMap::new();
        params.insert("id".to_string(), QueryParameter::String(id.to_string()));

        self.execute(query, params).await?;
        Ok(())
    }

    async fn find_edges(&self, source_id: Option<&str>, target_id: Option<&str>, edge_type: Option<&str>) -> DatabaseResult<Vec<EdgeData>> {
        // Simplified implementation
        Ok(Vec::new())
    }

    async fn clear_database(&self) -> DatabaseResult<()> {
        let query = "MATCH (n) DETACH DELETE n";
        self.execute(query, HashMap::new()).await?;
        Ok(())
    }

    async fn delete_by_group_id(&self, group_id: &str) -> DatabaseResult<()> {
        let query = "MATCH (n {group_id: $group_id}) DETACH DELETE n";
        let mut params = HashMap::new();
        params.insert("group_id".to_string(), QueryParameter::String(group_id.to_string()));

        self.execute(query, params).await?;
        Ok(())
    }

    async fn create_index(&self, label: &str, property: &str) -> DatabaseResult<()> {
        let query = format!("CREATE INDEX IF NOT EXISTS FOR (n:{}) ON (n.{})", label, property);
        self.execute(&query, HashMap::new()).await?;
        Ok(())
    }

    async fn create_constraint(&self, label: &str, property: &str, constraint_type: &str) -> DatabaseResult<()> {
        let query = match constraint_type {
            "UNIQUE" => format!("CREATE CONSTRAINT IF NOT EXISTS FOR (n:{}) REQUIRE n.{} IS UNIQUE", label, property),
            _ => return Err(DatabaseError::UnsupportedOperation(format!("Constraint type: {}", constraint_type))),
        };
        self.execute(&query, HashMap::new()).await?;
        Ok(())
    }

    async fn build_indices_and_constraints(&self) -> DatabaseResult<()> {
        // Create standard Graphiti indices and constraints
        self.create_constraint("Entity", "uuid", "UNIQUE").await?;
        self.create_constraint("Episodic", "uuid", "UNIQUE").await?;
        self.create_constraint("Community", "uuid", "UNIQUE").await?;

        self.create_index("Entity", "name").await?;
        self.create_index("Entity", "group_id").await?;
        self.create_index("Episodic", "name").await?;
        self.create_index("Episodic", "group_id").await?;
        self.create_index("Community", "name").await?;
        self.create_index("Community", "group_id").await?;

        Ok(())
    }

    async fn fulltext_search(&self, query: &str, labels: Vec<String>) -> DatabaseResult<Vec<NodeData>> {
        // Neo4j fulltext search implementation would go here
        Ok(Vec::new())
    }

    async fn vector_search(&self, embedding: Vec<f64>, label: &str, top_k: usize) -> DatabaseResult<Vec<(NodeData, f64)>> {
        // Neo4j vector search implementation would go here
        Ok(Vec::new())
    }
}

#[async_trait]
impl Transaction for Neo4jTransaction {
    async fn execute(&mut self, _query: &str, _parameters: HashMap<String, QueryParameter>) -> DatabaseResult<QueryResult> {
        // Simplified implementation for now
        Ok(QueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
        })
    }

    async fn commit(self: Box<Self>) -> DatabaseResult<()> {
        self.txn.commit().await?;
        Ok(())
    }

    async fn rollback(self: Box<Self>) -> DatabaseResult<()> {
        self.txn.rollback().await?;
        Ok(())
    }
}
