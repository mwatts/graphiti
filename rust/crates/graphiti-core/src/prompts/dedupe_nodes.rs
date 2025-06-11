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

//! Node deduplication prompts

use std::collections::HashMap;
use crate::prompts::models::{Message, PromptFunction};

/// Deduplicate similar nodes
pub fn dedupe(context: &HashMap<String, serde_json::Value>) -> Vec<Message> {
    let sys_prompt = "You are an AI assistant that identifies duplicate entities that should be merged.";

    let nodes = context.get("nodes")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "[]".to_string());

    let user_prompt = format!(r#"
<NODES>
{nodes}
</NODES>

Given the above nodes, identify any that represent the same entity and should be merged.
Consider variations in naming, abbreviations, and different ways of referring to the same entity.
"#);

    vec![
        Message::system(sys_prompt),
        Message::user(user_prompt),
    ]
}

/// Available prompt versions for node deduplication
pub struct DedupeNodesPrompt {
    pub dedupe: PromptFunction,
}

impl Default for DedupeNodesPrompt {
    fn default() -> Self {
        Self { dedupe }
    }
}
