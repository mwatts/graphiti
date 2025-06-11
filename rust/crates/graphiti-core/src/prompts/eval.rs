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

//! Evaluation prompts

use std::collections::HashMap;
use crate::prompts::models::{Message, PromptFunction};

/// Evaluate extraction quality
pub fn evaluate(context: &HashMap<String, serde_json::Value>) -> Vec<Message> {
    let sys_prompt = "You are an AI assistant that evaluates the quality of entity and relationship extraction.";

    let extracted_content = context.get("extracted_content")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "{}".to_string());

    let original_content = context.get("original_content")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let user_prompt = format!(r#"
<ORIGINAL CONTENT>
{original_content}
</ORIGINAL CONTENT>

<EXTRACTED CONTENT>
{extracted_content}
</EXTRACTED CONTENT>

Evaluate the quality of the extraction by comparing the original content with what was extracted.
Assess completeness, accuracy, and relevance of the extracted entities and relationships.
"#);

    vec![
        Message::system(sys_prompt),
        Message::user(user_prompt),
    ]
}

/// Available prompt versions for evaluation
pub struct EvalPrompt {
    pub evaluate: PromptFunction,
}

impl Default for EvalPrompt {
    fn default() -> Self {
        Self { evaluate }
    }
}
