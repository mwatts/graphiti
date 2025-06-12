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

//! Database abstraction traits

use std::collections::HashMap;
use std::fmt::Debug;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use super::types::{DatabaseResult, DatabaseError};

/// Represents a query parameter value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryParameter {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
    List(Vec<QueryParameter>),
    Map(HashMap<String, QueryParameter>),
}

/// Represents node data returned from queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeData {
    pub id: String,
    pub labels: Vec<String>,
    pub properties: HashMap<String, QueryParameter>,
}

/// Represents edge/relationship data returned from queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeData {
    pub id: String,
    pub relationship_type: String,
    pub source_id: String,
    pub target_id: String,
    pub properties: HashMap<String, QueryParameter>,
}

/// Represents a query result row
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<HashMap<String, QueryParameter>>,
}

/// Transaction interface for database operations
#[async_trait]
pub trait Transaction: Send + Sync {
    /// Execute a query within the transaction
    async fn execute(&mut self, query: &str, parameters: HashMap<String, QueryParameter>) -> DatabaseResult<QueryResult>;

    /// Commit the transaction
    async fn commit(self: Box<Self>) -> DatabaseResult<()>;

    /// Rollback the transaction
    async fn rollback(self: Box<Self>) -> DatabaseResult<()>;
}

/// Main database trait that abstracts graph database operations
#[async_trait]
pub trait GraphDatabase: Send + Sync + Debug {
    /// Execute a query and return results
    async fn execute(&self, query: &str, parameters: HashMap<String, QueryParameter>) -> DatabaseResult<QueryResult>;

    /// Begin a transaction
    async fn begin_transaction(&self) -> DatabaseResult<Box<dyn Transaction>>;

    /// Close the database connection
    async fn close(&self) -> DatabaseResult<()>;

    /// Check if the database connection is healthy
    async fn health_check(&self) -> DatabaseResult<bool>;

    /// Get a reference to the underlying object as Any for downcasting
    /// This is used for backward compatibility during migration
    fn as_any(&self) -> &dyn std::any::Any;

    // Node operations
    async fn create_node(&self, labels: Vec<String>, properties: HashMap<String, QueryParameter>) -> DatabaseResult<String>;
    async fn get_node(&self, id: &str) -> DatabaseResult<Option<NodeData>>;
    async fn update_node(&self, id: &str, properties: HashMap<String, QueryParameter>) -> DatabaseResult<()>;
    async fn delete_node(&self, id: &str) -> DatabaseResult<()>;
    async fn find_nodes(&self, label: Option<&str>, properties: HashMap<String, QueryParameter>) -> DatabaseResult<Vec<NodeData>>;

    // Edge operations
    async fn create_edge(&self, source_id: &str, target_id: &str, edge_type: &str, properties: HashMap<String, QueryParameter>) -> DatabaseResult<String>;
    async fn get_edge(&self, id: &str) -> DatabaseResult<Option<EdgeData>>;
    async fn update_edge(&self, id: &str, properties: HashMap<String, QueryParameter>) -> DatabaseResult<()>;
    async fn delete_edge(&self, id: &str) -> DatabaseResult<()>;
    async fn find_edges(&self, source_id: Option<&str>, target_id: Option<&str>, edge_type: Option<&str>) -> DatabaseResult<Vec<EdgeData>>;

    // Graph operations
    async fn clear_database(&self) -> DatabaseResult<()>;
    async fn delete_by_group_id(&self, group_id: &str) -> DatabaseResult<()>;

    // Index and constraint management
    async fn create_index(&self, label: &str, property: &str) -> DatabaseResult<()>;
    async fn create_constraint(&self, label: &str, property: &str, constraint_type: &str) -> DatabaseResult<()>;
    async fn build_indices_and_constraints(&self) -> DatabaseResult<()>;

    // Search operations
    async fn fulltext_search(&self, query: &str, labels: Vec<String>) -> DatabaseResult<Vec<NodeData>>;
    async fn vector_search(&self, embedding: Vec<f64>, label: &str, top_k: usize) -> DatabaseResult<Vec<(NodeData, f64)>>;
}
