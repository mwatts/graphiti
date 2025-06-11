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

//! Edge invalidation prompts

use std::collections::HashMap;
use crate::prompts::models::{Message, PromptFunction};

/// Identify edges that should be invalidated
pub fn invalidate(context: &HashMap<String, serde_json::Value>) -> Vec<Message> {
    let sys_prompt = "You are an AI assistant that identifies edges that are no longer valid and should be invalidated.";

    let edges = context.get("edges")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "[]".to_string());

    let new_content = context.get("new_content")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let user_prompt = format!(r#"
<EXISTING EDGES>
{edges}
</EXISTING EDGES>

<NEW CONTENT>
{new_content}
</NEW CONTENT>

Given the existing edges and new content, identify any edges that are contradicted by the new content
and should be invalidated (marked as no longer true).
"#);

    vec![
        Message::system(sys_prompt),
        Message::user(user_prompt),
    ]
}

/// Available prompt versions for edge invalidation
pub struct InvalidateEdgesPrompt {
    pub invalidate: PromptFunction,
}

impl Default for InvalidateEdgesPrompt {
    fn default() -> Self {
        Self { invalidate }
    }
}
