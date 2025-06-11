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

//! Prompt library implementation

use std::collections::HashMap;
use crate::prompts::{
    models::{Message, PromptFunction},
    extract_nodes::ExtractNodesPrompt,
    extract_edges::ExtractEdgesPrompt,
    dedupe_nodes::DedupeNodesPrompt,
    dedupe_edges::DedupeEdgesPrompt,
    invalidate_edges::InvalidateEdgesPrompt,
    extract_edge_dates::ExtractEdgeDatesPrompt,
    summarize_nodes::SummarizeNodesPrompt,
    eval::EvalPrompt,
    helpers::DO_NOT_ESCAPE_UNICODE,
};

/// Main prompt library interface
pub struct PromptLibrary {
    pub extract_nodes: ExtractNodesPrompt,
    pub extract_edges: ExtractEdgesPrompt,
    pub dedupe_nodes: DedupeNodesPrompt,
    pub dedupe_edges: DedupeEdgesPrompt,
    pub invalidate_edges: InvalidateEdgesPrompt,
    pub extract_edge_dates: ExtractEdgeDatesPrompt,
    pub summarize_nodes: SummarizeNodesPrompt,
    pub eval: EvalPrompt,
}

impl Default for PromptLibrary {
    fn default() -> Self {
        Self {
            extract_nodes: ExtractNodesPrompt::default(),
            extract_edges: ExtractEdgesPrompt::default(),
            dedupe_nodes: DedupeNodesPrompt::default(),
            dedupe_edges: DedupeEdgesPrompt::default(),
            invalidate_edges: InvalidateEdgesPrompt::default(),
            extract_edge_dates: ExtractEdgeDatesPrompt::default(),
            summarize_nodes: SummarizeNodesPrompt::default(),
            eval: EvalPrompt::default(),
        }
    }
}

/// Wrapper that adds unicode prevention to system messages
pub struct VersionWrapper {
    func: PromptFunction,
}

impl VersionWrapper {
    pub fn new(func: PromptFunction) -> Self {
        Self { func }
    }

    pub fn call(&self, context: &HashMap<String, serde_json::Value>) -> Vec<Message> {
        let mut messages = (self.func)(context);
        for message in &mut messages {
            if message.role == "system" {
                message.content.push_str(DO_NOT_ESCAPE_UNICODE);
            }
        }
        messages
    }
}

/// Get the default prompt library instance
pub fn get_prompt_library() -> PromptLibrary {
    PromptLibrary::default()
}
