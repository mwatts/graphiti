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

//! Helper utilities for Graphiti

use std::env;
use chrono::{DateTime, Utc};
use neo4rs::BoltType;
use tokio::sync::Semaphore;
use futures::future::join_all;

/// Default database name from environment
pub fn default_database() -> Option<String> {
    env::var("DEFAULT_DATABASE").ok()
}

/// Whether to use parallel runtime
pub fn use_parallel_runtime() -> bool {
    env::var("USE_PARALLEL_RUNTIME")
        .map(|v| v.parse().unwrap_or(false))
        .unwrap_or(false)
}

/// Semaphore limit for concurrent operations
pub fn semaphore_limit() -> usize {
    env::var("SEMAPHORE_LIMIT")
        .and_then(|v| v.parse().ok())
        .unwrap_or(20)
}

/// Maximum reflexion iterations
pub fn max_reflexion_iterations() -> usize {
    env::var("MAX_REFLEXION_ITERATIONS")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Default page limit for pagination
pub const DEFAULT_PAGE_LIMIT: usize = 20;

/// Runtime query prefix for parallel execution
pub fn runtime_query() -> &'static str {
    if use_parallel_runtime() {
        "CYPHER runtime = parallel parallelRuntimeSupport=all\n"
    } else {
        ""
    }
}

/// Parse Neo4j DateTime to Rust DateTime
pub fn parse_db_date(neo_date: Option<BoltType>) -> Option<DateTime<Utc>> {
    match neo_date {
        Some(BoltType::DateTime(dt)) => {
            // Convert Neo4j DateTime to chrono DateTime
            // This is a simplified conversion - in practice you'd need proper timezone handling
            DateTime::from_timestamp(dt.seconds(), dt.nanoseconds()).map(|dt| dt.with_timezone(&Utc))
        }
        _ => None,
    }
}

/// Sanitize query string for Lucene search
pub fn lucene_sanitize(query: &str) -> String {
    let special_chars = [
        ('+', r"\+"),
        ('-', r"\-"),
        ('&', r"\&"),
        ('|', r"\|"),
        ('!', r"\!"),
        ('(', r"\("),
        (')', r"\)"),
        ('{', r"\{"),
        ('}', r"\}"),
        ('[', r"\["),
        (']', r"\]"),
        ('^', r"\^"),
        ('"', r#"\""#),
        ('~', r"\~"),
        ('*', r"\*"),
        ('?', r"\?"),
        (':', r"\:"),
        ('\\', r"\\"),
        ('/', r"\/"),
        ('O', r"\O"),
        ('R', r"\R"),
        ('N', r"\N"),
        ('T', r"\T"),
        ('A', r"\A"),
        ('D', r"\D"),
    ];

    let mut sanitized = query.to_string();
    for (char, replacement) in special_chars {
        sanitized = sanitized.replace(char, replacement);
    }
    sanitized
}

/// Normalize embedding vector using L2 norm
pub fn normalize_l2(embedding: &[f32]) -> Vec<f32> {
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm == 0.0 {
        embedding.to_vec()
    } else {
        embedding.iter().map(|x| x / norm).collect()
    }
}

/// Execute futures with semaphore-based concurrency limiting
pub async fn semaphore_gather<T, F>(
    futures: Vec<F>,
    max_concurrent: Option<usize>,
) -> Vec<T>
where
    F: std::future::Future<Output = T> + Send,
    T: Send,
{
    let limit = max_concurrent.unwrap_or_else(semaphore_limit);
    let semaphore = Semaphore::new(limit);

    let tasks: Vec<_> = futures
        .into_iter()
        .map(|future| {
            let semaphore = &semaphore;
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                future.await
            }
        })
        .collect();

    join_all(tasks).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lucene_sanitize() {
        let input = "test+query-with&special|chars";
        let expected = r"test\+query\-with\&special\|chars";
        assert_eq!(lucene_sanitize(input), expected);
    }

    #[test]
    fn test_normalize_l2() {
        let embedding = vec![3.0, 4.0, 0.0];
        let normalized = normalize_l2(&embedding);
        let expected = vec![0.6, 0.8, 0.0];

        for (a, b) in normalized.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn test_normalize_l2_zero_vector() {
        let embedding = vec![0.0, 0.0, 0.0];
        let normalized = normalize_l2(&embedding);
        assert_eq!(normalized, embedding);
    }
}
