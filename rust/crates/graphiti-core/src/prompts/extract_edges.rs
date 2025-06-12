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

//! Edge extraction prompts

use crate::prompts::models::{Message, PromptFunction};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An extracted edge/relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub relation_type: String,
    pub source_entity_id: i32,
    pub target_entity_id: i32,
    pub fact: String,
    pub valid_at: Option<String>,
    pub invalid_at: Option<String>,
}

/// Collection of extracted edges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEdges {
    pub edges: Vec<Edge>,
}

/// Facts that were missed during extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingFacts {
    pub missing_facts: Vec<String>,
}

/// Extract relationships/edges between entities
pub fn edge(context: &HashMap<String, serde_json::Value>) -> Vec<Message> {
    let previous_episodes = context
        .get("previous_episodes")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "[]".to_string());

    let episode_content = context
        .get("episode_content")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let nodes = context
        .get("nodes")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "[]".to_string());

    let reference_time = context
        .get("reference_time")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let edge_types = context
        .get("edge_types")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "[]".to_string());

    let sys_prompt = "You are an expert fact extractor that extracts fact triples from text. \
        1. Extracted fact triples should also be extracted with relevant date information.\
        2. Treat the CURRENT TIME as the time the CURRENT MESSAGE was sent. All temporal information should be extracted relative to this time.";

    let user_prompt = format!(
        r#"
<PREVIOUS_MESSAGES>
{previous_episodes}
</PREVIOUS_MESSAGES>

<CURRENT_MESSAGE>
{episode_content}
</CURRENT_MESSAGE>

<ENTITIES>
{nodes}
</ENTITIES>

<REFERENCE_TIME>
{reference_time}  # ISO 8601 (UTC); used to resolve relative time mentions
</REFERENCE_TIME>

<FACT TYPES>
{edge_types}
</FACT TYPES>

# TASK
Extract all factual relationships between the given ENTITIES based on the CURRENT MESSAGE.
Only extract facts that:
- involve two DISTINCT ENTITIES from the ENTITIES list,
- are clearly stated or unambiguously implied in the CURRENT MESSAGE,
    and can be represented as edges in a knowledge graph.
- The FACT TYPES provide a list of the most important types of facts, make sure to extract facts of these types
- The FACT TYPES are not an exhaustive list, extract all facts from the message even if they do not fit into one
    of the FACT TYPES

## GUIDELINES

### Edge Creation
1. **Entity References**: Only use entities from the ENTITIES list. Use the entity's `id` field for `source_entity_id` and `target_entity_id`.

2. **Relation Types**:
   - Use descriptive, factual relation types in SCREAMING_SNAKE_CASE (e.g., WORKS_AT, LIVES_IN, MARRIED_TO).
   - Relations should be predicates that make the edge readable as "source [relation] target".
   - Be specific and descriptive (e.g., "GRADUATED_FROM" not just "ATTENDED").

3. **Fact Descriptions**:
   - Write clear, factual descriptions of the relationship.
   - Include relevant context from the message.
   - Be concise but informative.

4. **Temporal Information**:
   - Extract **valid_at** when the relationship began or was established.
   - Extract **invalid_at** when the relationship ended (if mentioned).
   - Use REFERENCE_TIME to resolve relative time expressions.
   - Use ISO 8601 format (YYYY-MM-DDTHH:MM:SS.SSSSSSZ).
   - If only a date is mentioned, assume start of day (00:00:00) for valid_at and end of day for invalid_at.

### What to Extract
- **Direct statements**: "Alice works at Google", "Bob married Carol in 2020"
- **Implied relationships**: If someone mentions "my wife", extract the marriage relationship
- **Ongoing states**: Current jobs, relationships, locations
- **Past events**: Previous jobs, completed actions, historical facts
- **Temporal bounds**: When relationships started and ended

### What NOT to Extract
- Relationships between entities not in the ENTITIES list
- Vague or uncertain statements without clear factual basis
- Temporary interactions that don't represent lasting relationships
- Self-loops (entity related to itself)

### Examples
If the message is "Alice started working at Google last month and Bob graduated from MIT in 2019":
- Extract: Alice WORKS_AT Google (valid_at: calculated from "last month")
- Extract: Bob GRADUATED_FROM MIT (valid_at: 2019-01-01T00:00:00.000000Z)
"#
    );

    vec![Message::system(sys_prompt), Message::user(user_prompt)]
}

/// Reflexion prompt to identify missed facts
pub fn reflexion(context: &HashMap<String, serde_json::Value>) -> Vec<Message> {
    let sys_prompt = "You are an AI assistant that determines which facts have not been extracted from the given context";

    let previous_episodes = context
        .get("previous_episodes")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "[]".to_string());

    let episode_content = context
        .get("episode_content")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let extracted_edges = context
        .get("extracted_edges")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "[]".to_string());

    let user_prompt = format!(
        r#"
<PREVIOUS MESSAGES>
{previous_episodes}
</PREVIOUS MESSAGES>
<CURRENT MESSAGE>
{episode_content}
</CURRENT MESSAGE>

<EXTRACTED EDGES>
{extracted_edges}
</EXTRACTED EDGES>

Given the above previous messages, current message, and list of extracted edges; determine if any facts haven't been
extracted.
"#
    );

    vec![Message::system(sys_prompt), Message::user(user_prompt)]
}

/// Extract attributes for edges
pub fn extract_attributes(context: &HashMap<String, serde_json::Value>) -> Vec<Message> {
    let previous_episodes = context
        .get("previous_episodes")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "[]".to_string());

    let episode_content = context
        .get("episode_content")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "{}".to_string());

    let edge = context
        .get("edge")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "{}".to_string());

    let user_content = format!(
        r#"

        <MESSAGES>
        {previous_episodes}
        {episode_content}
        </MESSAGES>

        Given the above MESSAGES and the following EDGE, update any of its attributes based on the information provided
        in MESSAGES. Use the provided attribute descriptions to better understand how each attribute should be determined.

        Guidelines:
        1. Do not hallucinate edge property values if they cannot be found in the current context.
        2. Only use the provided MESSAGES and EDGE to set attribute values.
        3. Update temporal information (valid_at, invalid_at) if mentioned in the messages.

        <EDGE>
        {edge}
        </EDGE>
        "#
    );

    vec![
        Message::system(
            "You are a helpful assistant that extracts edge properties from the provided text.",
        ),
        Message::user(user_content),
    ]
}

/// Available prompt versions for edge extraction
pub struct ExtractEdgesPrompt {
    pub edge: PromptFunction,
    pub reflexion: PromptFunction,
    pub extract_attributes: PromptFunction,
}

impl Default for ExtractEdgesPrompt {
    fn default() -> Self {
        Self {
            edge,
            reflexion,
            extract_attributes,
        }
    }
}
