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

//! Database abstraction layer for Graphiti
//!
//! This module provides a database-agnostic interface for graph operations.
//! It supports multiple database backends including Neo4j and KuzuDB.

use std::sync::Arc;

pub mod traits;
pub mod neo4j;
pub mod kuzu;
pub mod config;
pub mod types;

pub use traits::{GraphDatabase, QueryResult, QueryParameter, NodeData, EdgeData, Transaction};
pub use config::{DatabaseConfig, DatabaseType};
pub use types::{DatabaseError, DatabaseResult};

/// Factory function to create a database instance based on configuration
pub async fn create_database(config: DatabaseConfig) -> DatabaseResult<Arc<dyn GraphDatabase + Send + Sync>> {
    match config.database_type {
        DatabaseType::Neo4j => {
            let db = neo4j::Neo4jDatabase::new(config).await?;
            Ok(Arc::new(db))
        }
        DatabaseType::Kuzu => {
            let db = kuzu::KuzuDatabase::new(config).await?;
            Ok(Arc::new(db))
        }
    }
}
