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

//! Prompt library for LLM interactions
//!
//! This module contains all the prompts used for entity extraction, edge extraction,
//! deduplication, and other LLM-driven operations in Graphiti.

pub mod dedupe_edges;
pub mod dedupe_nodes;
pub mod eval;
pub mod extract_edge_dates;
pub mod extract_edges;
pub mod extract_nodes;
pub mod helpers;
pub mod invalidate_edges;
pub mod lib;
pub mod models;
pub mod summarize_nodes;

pub use lib::{get_prompt_library, PromptLibrary};
pub use models::{Message, PromptFunction};

/// Unicode escape prevention string
pub const DO_NOT_ESCAPE_UNICODE: &str = "\nDo not escape unicode characters.\n";
