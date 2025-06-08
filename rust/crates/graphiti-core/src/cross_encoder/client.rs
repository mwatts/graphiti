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

use crate::errors::GraphitiError;

/// Trait for cross-encoder models used for ranking passages based on their relevance to a query.
/// Allows for different implementations of cross-encoder models to be used interchangeably.
#[async_trait]
pub trait CrossEncoderClient: Send + Sync {
    /// Rank the given passages based on their relevance to the query.
    ///
    /// # Arguments
    /// * `query` - The query string
    /// * `passages` - A list of passages to rank
    ///
    /// # Returns
    /// A list of tuples containing the passage and its score,
    /// sorted in descending order of relevance.
    async fn rank(
        &self,
        query: &str,
        passages: &[String],
    ) -> Result<Vec<(String, f64)>, GraphitiError>;
}
