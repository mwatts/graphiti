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

//! Node summarization prompts

use crate::prompts::models::{Message, PromptFunction};
use std::collections::HashMap;

/// Summarize node information
pub fn summarize(context: &HashMap<String, serde_json::Value>) -> Vec<Message> {
    let sys_prompt =
        "You are an AI assistant that creates concise summaries of entity information.";

    let nodes = context
        .get("nodes")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "[]".to_string());

    let context_messages = context
        .get("context_messages")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "[]".to_string());

    let user_prompt = format!(
        r#"
<NODES>
{nodes}
</NODES>

<CONTEXT MESSAGES>
{context_messages}
</CONTEXT MESSAGES>

Given the above nodes and context messages, create concise summaries for each node
that capture the most important information about the entity. Summaries should be
no longer than 250 words.
"#
    );

    vec![Message::system(sys_prompt), Message::user(user_prompt)]
}

/// Available prompt versions for node summarization
pub struct SummarizeNodesPrompt {
    pub summarize: PromptFunction,
}

impl Default for SummarizeNodesPrompt {
    fn default() -> Self {
        Self { summarize }
    }
}
