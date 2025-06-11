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

//! Node extraction prompts

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::prompts::models::{Message, PromptFunction};

/// Extracted entity from text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type_id: i32,
}

/// Collection of extracted entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntities {
    pub extracted_entities: Vec<ExtractedEntity>,
}

/// Entities that were missed during extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissedEntities {
    pub missed_entities: Vec<String>,
}

/// Entity classification triple
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityClassificationTriple {
    pub uuid: String,
    pub name: String,
    pub entity_type: Option<String>,
}

/// Collection of entity classifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityClassification {
    pub entity_classifications: Vec<EntityClassificationTriple>,
}

/// Extract entities from conversational messages
pub fn extract_message(context: &HashMap<String, serde_json::Value>) -> Vec<Message> {
    let sys_prompt = "You are an AI assistant that extracts entity nodes from conversational messages. \
        Your primary task is to extract and classify the speaker and other significant entities mentioned in the conversation.";

    let previous_episodes = context.get("previous_episodes")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "[]".to_string());

    let episode_content = context.get("episode_content")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let entity_types = context.get("entity_types")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "{}".to_string());

    let custom_prompt = context.get("custom_prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let user_prompt = format!(r#"
<PREVIOUS MESSAGES>
{previous_episodes}
</PREVIOUS MESSAGES>

<CURRENT MESSAGE>
{episode_content}
</CURRENT MESSAGE>

<ENTITY TYPES>
{entity_types}
</ENTITY TYPES>

Instructions:

You are given a conversation context and a CURRENT MESSAGE. Your task is to extract **entity nodes** mentioned **explicitly or implicitly** in the CURRENT MESSAGE.

1. **Speaker Extraction**: Always extract the speaker (the part before the colon `:` in each dialogue line) as the first entity node.
   - If the speaker is mentioned again in the message, treat both mentions as a **single entity**.

2. **Entity Identification**:
   - Extract all significant entities, concepts, or actors that are **explicitly or implicitly** mentioned in the CURRENT MESSAGE.
   - **Exclude** entities mentioned only in the PREVIOUS MESSAGES (they are for context only).

3. **Entity Classification**:
   - Use the descriptions in ENTITY TYPES to classify each extracted entity.
   - Assign the appropriate `entity_type_id` for each one.

4. **Exclusions**:
   - Do NOT extract entities representing relationships or actions.
   - Do NOT extract dates, times, or other temporal information—these will be handled separately.

5. **Formatting**:
   - Be **explicit and unambiguous** in naming entities (e.g., use full names when available).

{custom_prompt}
"#);

    vec![
        Message::system(sys_prompt),
        Message::user(user_prompt),
    ]
}

/// Extract entities from JSON content
pub fn extract_json(context: &HashMap<String, serde_json::Value>) -> Vec<Message> {
    let sys_prompt = "You are an AI assistant that extracts entity nodes from JSON. \
        Your primary task is to extract and classify relevant entities from JSON files";

    let source_description = context.get("source_description")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let episode_content = context.get("episode_content")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "{}".to_string());

    let entity_types = context.get("entity_types")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "{}".to_string());

    let custom_prompt = context.get("custom_prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let user_prompt = format!(r#"
<SOURCE DESCRIPTION>:
{source_description}
</SOURCE DESCRIPTION>
<JSON>
{episode_content}
</JSON>
<ENTITY TYPES>
{entity_types}
</ENTITY TYPES>

{custom_prompt}

Given the above source description and JSON, extract relevant entities from the provided JSON.
For each entity extracted, also determine its entity type based on the provided ENTITY TYPES and their descriptions.
Indicate the classified entity type by providing its entity_type_id.

Guidelines:
1. Always try to extract an entities that the JSON represents. This will often be something like a "name" or "user field
2. Do NOT extract any properties that contain dates
"#);

    vec![
        Message::system(sys_prompt),
        Message::user(user_prompt),
    ]
}

/// Extract entities from plain text
pub fn extract_text(context: &HashMap<String, serde_json::Value>) -> Vec<Message> {
    let sys_prompt = "You are an AI assistant that extracts entity nodes from text. \
        Your primary task is to extract and classify the speaker and other significant entities mentioned in the provided text.";

    let episode_content = context.get("episode_content")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let entity_types = context.get("entity_types")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "{}".to_string());

    let custom_prompt = context.get("custom_prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let user_prompt = format!(r#"
<TEXT>
{episode_content}
</TEXT>
<ENTITY TYPES>
{entity_types}
</ENTITY TYPES>

Given the above text, extract entities from the TEXT that are explicitly or implicitly mentioned.
For each entity extracted, also determine its entity type based on the provided ENTITY TYPES and their descriptions.
Indicate the classified entity type by providing its entity_type_id.

{custom_prompt}

Guidelines:
1. Extract significant entities, concepts, or actors mentioned in the conversation.
2. Avoid creating nodes for relationships or actions.
3. Avoid creating nodes for temporal information like dates, times or years (these will be added to edges later).
4. Be as explicit as possible in your node names, using full names and avoiding abbreviations.
"#);

    vec![
        Message::system(sys_prompt),
        Message::user(user_prompt),
    ]
}

/// Reflexion prompt to identify missed entities
pub fn reflexion(context: &HashMap<String, serde_json::Value>) -> Vec<Message> {
    let sys_prompt = "You are an AI assistant that determines which entities have not been extracted from the given context";

    let previous_episodes = context.get("previous_episodes")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "[]".to_string());

    let episode_content = context.get("episode_content")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let extracted_entities = context.get("extracted_entities")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "[]".to_string());

    let user_prompt = format!(r#"
<PREVIOUS MESSAGES>
{previous_episodes}
</PREVIOUS MESSAGES>
<CURRENT MESSAGE>
{episode_content}
</CURRENT MESSAGE>

<EXTRACTED ENTITIES>
{extracted_entities}
</EXTRACTED ENTITIES>

Given the above previous messages, current message, and list of extracted entities; determine if any entities haven't been
extracted.
"#);

    vec![
        Message::system(sys_prompt),
        Message::user(user_prompt),
    ]
}

/// Classify extracted entities
pub fn classify_nodes(context: &HashMap<String, serde_json::Value>) -> Vec<Message> {
    let sys_prompt = "You are an AI assistant that classifies entity nodes given the context from which they were extracted";

    let previous_episodes = context.get("previous_episodes")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "[]".to_string());

    let episode_content = context.get("episode_content")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let extracted_entities = context.get("extracted_entities")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "[]".to_string());

    let entity_types = context.get("entity_types")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "{}".to_string());

    let user_prompt = format!(r#"
    <PREVIOUS MESSAGES>
    {previous_episodes}
    </PREVIOUS MESSAGES>
    <CURRENT MESSAGE>
    {episode_content}
    </CURRENT MESSAGE>

    <EXTRACTED ENTITIES>
    {extracted_entities}
    </EXTRACTED ENTITIES>

    <ENTITY TYPES>
    {entity_types}
    </ENTITY TYPES>

    Given the above conversation, extracted entities, and provided entity types and their descriptions, classify the extracted entities.

    Guidelines:
    1. Each entity must have exactly one type
    2. Only use the provided ENTITY TYPES as types, do not use additional types to classify entities.
    3. If none of the provided entity types accurately classify an extracted node, the type should be set to None
"#);

    vec![
        Message::system(sys_prompt),
        Message::user(user_prompt),
    ]
}

/// Extract attributes for entities
pub fn extract_attributes(context: &HashMap<String, serde_json::Value>) -> Vec<Message> {
    let previous_episodes = context.get("previous_episodes")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "[]".to_string());

    let episode_content = context.get("episode_content")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "{}".to_string());

    let node = context.get("node")
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| "{}".to_string());

    let user_content = format!(r#"

        <MESSAGES>
        {previous_episodes}
        {episode_content}
        </MESSAGES>

        Given the above MESSAGES and the following ENTITY, update any of its attributes based on the information provided
        in MESSAGES. Use the provided attribute descriptions to better understand how each attribute should be determined.

        Guidelines:
        1. Do not hallucinate entity property values if they cannot be found in the current context.
        2. Only use the provided MESSAGES and ENTITY to set attribute values.
        3. The summary attribute represents a summary of the ENTITY, and should be updated with new information about the Entity from the MESSAGES.
            Summaries must be no longer than 250 words.

        <ENTITY>
        {node}
        </ENTITY>
        "#);

    vec![
        Message::system("You are a helpful assistant that extracts entity properties from the provided text."),
        Message::user(user_content),
    ]
}

/// Available prompt versions for node extraction
pub struct ExtractNodesPrompt {
    pub extract_message: PromptFunction,
    pub extract_json: PromptFunction,
    pub extract_text: PromptFunction,
    pub reflexion: PromptFunction,
    pub classify_nodes: PromptFunction,
    pub extract_attributes: PromptFunction,
}

impl Default for ExtractNodesPrompt {
    fn default() -> Self {
        Self {
            extract_message,
            extract_json,
            extract_text,
            reflexion,
            classify_nodes,
            extract_attributes,
        }
    }
}
