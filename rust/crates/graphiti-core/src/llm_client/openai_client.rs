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

use std::collections::HashMap;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{error, warn};

use crate::errors::{LlmError, LlmResult};
use super::client::{BaseLlmClient, LlmClient};
use super::config::{LlmConfig, ModelSize};
use super::models::Message;

const DEFAULT_MODEL: &str = "gpt-4o-mini";
const DEFAULT_SMALL_MODEL: &str = "gpt-4o-mini";
const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    temperature: f32,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponseMessage {
    content: Option<String>,
    refusal: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
    #[serde(default)]
    error: Option<OpenAiError>,
}

#[derive(Debug, Deserialize)]
struct OpenAiError {
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
    code: Option<String>,
}

pub struct OpenAiClient {
    base_client: BaseLlmClient,
    http_client: Client,
    base_url: String,
    api_key: String,
    max_retries: usize,
}

impl OpenAiClient {
    pub fn new(config: LlmConfig, cache_enabled: bool) -> LlmResult<Self> {
        let api_key = config.api_key.clone().ok_or_else(|| LlmError::Authentication {
            message: "OpenAI API key is required".to_string(),
        })?;
        
        let base_url = config.base_url.clone()
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
            
        let base_client = BaseLlmClient::new(config, cache_enabled)?;
        
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| LlmError::InvalidConfig {
                message: format!("Failed to create HTTP client: {}", e),
            })?;
        
        Ok(Self {
            base_client,
            http_client,
            base_url,
            api_key,
            max_retries: 2,
        })
    }
    
    async fn generate_response_internal(
        &self,
        messages: &[Message],
        response_model: Option<&str>,
        max_tokens: Option<u32>,
        model_size: ModelSize,
    ) -> LlmResult<HashMap<String, Value>> {
        let model = match model_size {
            ModelSize::Small => self.base_client.config.small_model.as_deref()
                .unwrap_or(DEFAULT_SMALL_MODEL),
            ModelSize::Medium => self.base_client.config.model.as_deref()
                .unwrap_or(DEFAULT_MODEL),
        };
        
        let openai_messages: Vec<OpenAiMessage> = messages.iter()
            .map(|m| OpenAiMessage {
                role: m.role.clone(),
                content: self.base_client.clean_input(&m.content),
            })
            .collect();
        
        let mut request = OpenAiChatRequest {
            model: model.to_string(),
            messages: openai_messages,
            temperature: self.base_client.config.temperature,
            max_tokens: max_tokens.unwrap_or(self.base_client.config.max_tokens),
            response_format: None,
        };
        
        // Add JSON schema if response model is provided
        if let Some(schema) = response_model {
            if let Ok(schema_value) = serde_json::from_str::<Value>(schema) {
                request.response_format = Some(json!({
                    "type": "json_schema",
                    "json_schema": schema_value
                }));
            }
        }
        
        let url = format!("{}/chat/completions", self.base_url);
        
        let response = self.http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::Http(e))?;
        
        if response.status() == 429 {
            return Err(LlmError::RateLimit);
        }
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::Authentication {
                message: format!("HTTP {} - {}", status, error_text),
            });
        }
        
        let chat_response: OpenAiChatResponse = response.json().await
            .map_err(|e| LlmError::NetworkError {
                message: format!("Failed to parse JSON response: {}", e),
            })?;
        
        if let Some(error) = chat_response.error {
            return Err(LlmError::Authentication {
                message: error.message,
            });
        }
        
        let choice = chat_response.choices.into_iter().next()
            .ok_or_else(|| LlmError::EmptyResponse {
                message: "No choices in response".to_string(),
            })?;
        
        if let Some(refusal) = choice.message.refusal {
            return Err(LlmError::Refusal { message: refusal });
        }
        
        let content = choice.message.content.ok_or_else(|| LlmError::EmptyResponse {
            message: "No content in response".to_string(),
        })?;
        
        // Try to parse as JSON first, if that fails return as plain text
        let mut result = HashMap::new();
        if let Ok(json_value) = serde_json::from_str::<Value>(&content) {
            if let Some(obj) = json_value.as_object() {
                result.extend(obj.iter().map(|(k, v)| (k.clone(), v.clone())));
            } else {
                result.insert("content".to_string(), json_value);
            }
        } else {
            result.insert("content".to_string(), Value::String(content));
        }
        
        Ok(result)
    }
}

#[async_trait]
impl LlmClient for OpenAiClient {
    async fn generate_response(
        &self,
        messages: &[Message],
        response_model: Option<&str>,
        max_tokens: Option<u32>,
        model_size: ModelSize,
    ) -> LlmResult<HashMap<String, Value>> {
        // Check cache first
        if self.base_client.cache_enabled {
            let cache_key = self.base_client.get_cache_key(messages);
            if let Some(cached_response) = self.base_client.get_cached_response(&cache_key).await {
                return Ok(cached_response);
            }
        }
        
        // Prepare messages with schema and multilingual instructions
        let prepared_messages = self.base_client.prepare_messages(messages.to_vec(), response_model);
        
        let mut retry_count = 0;
        let mut last_error = None;
        let mut current_messages = prepared_messages;
        
        while retry_count <= self.max_retries {
            match self.generate_response_internal(
                &current_messages,
                response_model,
                max_tokens,
                model_size,
            ).await {
                Ok(response) => {
                    // Cache the response if caching is enabled
                    if self.base_client.cache_enabled {
                        let cache_key = self.base_client.get_cache_key(messages);
                        self.base_client.cache_response(&cache_key, &response).await;
                    }
                    return Ok(response);
                }
                Err(LlmError::RateLimit | LlmError::Refusal { .. }) => {
                    // Don't retry these errors
                    return Err(last_error.unwrap_or_else(|| LlmError::RateLimit));
                }
                Err(e) => {
                    // Store error details as string to avoid clone issues
                    let error_details = format!("{:?}", e);
                    
                    if retry_count >= self.max_retries {
                        error!("Max retries ({}) exceeded. Last error: {:?}", self.max_retries, e);
                        return Err(e);
                    }
                    
                    last_error = Some(LlmError::EmptyResponse { 
                        message: error_details.clone() 
                    });
                    
                    retry_count += 1;
                    
                    // Add error context for retry
                    let error_context = format!(
                        "The previous response attempt was invalid. \
                        Error type: {}. \
                        Please try again with a valid response, ensuring the output matches \
                        the expected format and constraints.",
                        std::any::type_name_of_val(&e)
                    );
                    
                    current_messages.push(Message::user(error_context));
                    warn!("Retrying after application error (attempt {}/{}): {:?}", 
                          retry_count, self.max_retries, e);
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| LlmError::EmptyResponse {
            message: "Max retries exceeded with no specific error".to_string(),
        }))
    }
    
    async fn chat_completion(
        &self,
        messages: &[Message],
        _json_params: Option<Value>,
    ) -> LlmResult<Value> {
        // Convert to the format expected by generate_response
        let response = self.generate_response(messages, None, None, ModelSize::Medium).await?;
        Ok(Value::Object(response.into_iter().collect()))
    }
}
