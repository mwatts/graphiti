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

//! Core prompt models and types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A message in a conversation with an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new("system", content)
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new("user", content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new("assistant", content)
    }
}

/// Type alias for prompt functions that take context and return messages
pub type PromptFunction = fn(&HashMap<String, serde_json::Value>) -> Vec<Message>;

/// Trait for prompt versions that can be called with context
pub trait PromptVersion {
    fn call(&self, context: &HashMap<String, serde_json::Value>) -> Vec<Message>;
}

impl PromptVersion for PromptFunction {
    fn call(&self, context: &HashMap<String, serde_json::Value>) -> Vec<Message> {
        self(context)
    }
}

/// Context type for prompts
pub type PromptContext = HashMap<String, serde_json::Value>;
