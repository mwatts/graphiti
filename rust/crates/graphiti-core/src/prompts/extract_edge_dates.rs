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

//! Edge date extraction prompts

use std::collections::HashMap;
use crate::prompts::models::{Message, PromptFunction};

/// Extract dates from edge content
pub fn extract_dates(context: &HashMap<String, serde_json::Value>) -> Vec<Message> {
    let sys_prompt = "You are an AI assistant that extracts temporal information from edge facts.";

    let edges = context.get("edges")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "[]".to_string());

    let reference_time = context.get("reference_time")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let user_prompt = format!(r#"
<EDGES>
{edges}
</EDGES>

<REFERENCE TIME>
{reference_time}
</REFERENCE TIME>

Given the above edges and reference time, extract valid_at and invalid_at dates for each edge
based on temporal information in the edge facts. Use ISO 8601 format.
"#);

    vec![
        Message::system(sys_prompt),
        Message::user(user_prompt),
    ]
}

/// Available prompt versions for edge date extraction
pub struct ExtractEdgeDatesPrompt {
    pub extract_dates: PromptFunction,
}

impl Default for ExtractEdgeDatesPrompt {
    fn default() -> Self {
        Self { extract_dates }
    }
}
