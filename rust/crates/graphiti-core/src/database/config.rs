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

//! Database configuration types

use serde::{Deserialize, Serialize};

/// Supported database types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DatabaseType {
    Neo4j,
    Kuzu,
}

impl std::fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseType::Neo4j => write!(f, "neo4j"),
            DatabaseType::Kuzu => write!(f, "kuzu"),
        }
    }
}

impl std::str::FromStr for DatabaseType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "neo4j" => Ok(DatabaseType::Neo4j),
            "kuzu" => Ok(DatabaseType::Kuzu),
            _ => Err(format!("Unknown database type: {}", s)),
        }
    }
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub database_type: DatabaseType,
    pub uri: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub database: Option<String>,
    pub pool_size: Option<usize>,
    pub timeout_seconds: Option<u64>,
    pub additional_config: std::collections::HashMap<String, String>,
}

impl DatabaseConfig {
    /// Create a new Neo4j configuration
    pub fn neo4j(uri: String, username: String, password: String) -> Self {
        Self {
            database_type: DatabaseType::Neo4j,
            uri,
            username: Some(username),
            password: Some(password),
            database: Some("neo4j".to_string()),
            pool_size: None,
            timeout_seconds: None,
            additional_config: std::collections::HashMap::new(),
        }
    }

    /// Create a new KuzuDB configuration
    pub fn kuzu(database_path: String) -> Self {
        Self {
            database_type: DatabaseType::Kuzu,
            uri: database_path,
            username: None,
            password: None,
            database: None,
            pool_size: None,
            timeout_seconds: None,
            additional_config: std::collections::HashMap::new(),
        }
    }

    /// Set the database name
    pub fn with_database(mut self, database: String) -> Self {
        self.database = Some(database);
        self
    }

    /// Set the connection pool size
    pub fn with_pool_size(mut self, pool_size: usize) -> Self {
        self.pool_size = Some(pool_size);
        self
    }

    /// Set the timeout in seconds
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = Some(timeout_seconds);
        self
    }

    /// Add additional configuration
    pub fn with_config(mut self, key: String, value: String) -> Self {
        self.additional_config.insert(key, value);
        self
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self::neo4j(
            "bolt://localhost:7687".to_string(),
            "neo4j".to_string(),
            "password".to_string(),
        )
    }
}
