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

use serde::{Deserialize, Serialize};

pub const DEFAULT_MAX_TOKENS: u32 = 8192;
pub const DEFAULT_TEMPERATURE: f32 = 0.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ModelSize {
    #[serde(rename = "small")]
    Small,
    #[serde(rename = "medium")]
    Medium,
}

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub small_model: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: None,
            base_url: None,
            temperature: DEFAULT_TEMPERATURE,
            max_tokens: DEFAULT_MAX_TOKENS,
            small_model: None,
        }
    }
}

impl LlmConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = Some(base_url);
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn with_small_model(mut self, small_model: String) -> Self {
        self.small_model = Some(small_model);
        self
    }
}
