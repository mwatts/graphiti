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
use serde_json::{json, Value};
use tracing::warn;

use crate::{
    cross_encoder::client::CrossEncoderClient,
    errors::GraphitiError,
    llm_client::{client::LlmClient, config::LlmConfig, models::Message},
};

const DEFAULT_MODEL: &str = "gpt-4.1-nano";

/// OpenAI-based reranker that uses the OpenAI API to run a simple boolean classifier 
/// prompt concurrently for each passage. Log-probabilities are used to rank the passages.
pub struct OpenAIRerankerClient {
    llm_client: Box<dyn LlmClient>,
}

impl OpenAIRerankerClient {
    pub fn new(config: Option<LlmConfig>) -> Result<Self, GraphitiError> {
        let config = config.unwrap_or_default();
        let llm_client = crate::llm_client::openai_client::OpenAiClient::new(config, false)?;
        
        Ok(Self {
            llm_client: Box::new(llm_client),
        })
    }

    pub fn with_client(llm_client: Box<dyn LlmClient>) -> Self {
        Self { llm_client }
    }
}

#[async_trait]
impl CrossEncoderClient for OpenAIRerankerClient {
    async fn rank(
        &self,
        query: &str,
        passages: &[String],
    ) -> Result<Vec<(String, f64)>, GraphitiError> {
        if passages.is_empty() {
            return Ok(Vec::new());
        }

        // Create messages for each passage
        let message_sets: Vec<Vec<Message>> = passages
            .iter()
            .map(|passage| {
                vec![
                    Message {
                        role: "system".to_string(),
                        content: "You are an expert tasked with determining whether the passage is relevant to the query".to_string(),
                    },
                    Message {
                        role: "user".to_string(),
                        content: format!(
                            r#"Respond with "True" if PASSAGE is relevant to QUERY and "False" otherwise.
<PASSAGE>
{}
</PASSAGE>
<QUERY>
{}
</QUERY>"#,
                            passage, query
                        ),
                    },
                ]
            })
            .collect();

        // Process requests concurrently
        let mut tasks = Vec::new();
        for messages in message_sets {
            let client = &self.llm_client;
            
            // Create request parameters for logit bias and logprobs
            let mut params = serde_json::Map::new();
            params.insert("model".to_string(), json!(DEFAULT_MODEL));
            params.insert("messages".to_string(), json!(messages));
            params.insert("temperature".to_string(), json!(0));
            params.insert("max_tokens".to_string(), json!(1));
            
            // Logit bias for True/False tokens
            let mut logit_bias = HashMap::new();
            logit_bias.insert("6432".to_string(), 1); // "True" token
            logit_bias.insert("7983".to_string(), 1); // "False" token
            params.insert("logit_bias".to_string(), json!(logit_bias));
            params.insert("logprobs".to_string(), json!(true));
            params.insert("top_logprobs".to_string(), json!(2));

            let task = async move {
                client.chat_completion(&messages, Some(Value::Object(params.into()))).await
            };
            
            tasks.push(task);
        }

        // Execute all requests concurrently
        let responses = futures::future::try_join_all(tasks).await?;

        // Extract scores from logprobs
        let mut results = Vec::new();
        for (i, response) in responses.iter().enumerate() {
            let passage = &passages[i];
            
            // Extract logprobs from response
            let score = extract_score_from_response(response)
                .unwrap_or_else(|| {
                    warn!("Failed to extract score for passage {}, using default 0.0", i);
                    0.0
                });
            
            results.push((passage.clone(), score));
        }

        // Sort by score in descending order
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results)
    }
}

/// Extract relevance score from OpenAI response logprobs
fn extract_score_from_response(response: &Value) -> Option<f64> {
    let choices = response.get("choices")?.as_array()?;
    let first_choice = choices.first()?;
    let logprobs = first_choice.get("logprobs")?;
    let content = logprobs.get("content")?.as_array()?;
    let first_content = content.first()?;
    let top_logprobs = first_content.get("top_logprobs")?.as_array()?;
    
    if top_logprobs.is_empty() {
        return None;
    }
    
    let top_logprob = &top_logprobs[0];
    let logprob = top_logprob.get("logprob")?.as_f64()?;
    let token = top_logprob.get("token")?.as_str()?;
    
    let norm_logprob = logprob.exp();
    
    // If the token indicates relevance, use the probability directly
    // Otherwise, use 1 - probability
    let score = if token.to_lowercase().contains("true") || 
                  token.to_lowercase().contains("yes") ||
                  token.to_lowercase().contains("relevant") {
        norm_logprob
    } else {
        1.0 - norm_logprob
    };
    
    Some(score)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_score_from_response() {
        let response = json!({
            "choices": [{
                "logprobs": {
                    "content": [{
                        "top_logprobs": [{
                            "token": "True",
                            "logprob": -0.5
                        }]
                    }]
                }
            }]
        });

        let score = extract_score_from_response(&response).unwrap();
        assert!((score - (-0.5_f64).exp()).abs() < 1e-10);
    }

    #[test]
    fn test_extract_score_false_token() {
        let response = json!({
            "choices": [{
                "logprobs": {
                    "content": [{
                        "top_logprobs": [{
                            "token": "False", 
                            "logprob": -0.5
                        }]
                    }]
                }
            }]
        });

        let score = extract_score_from_response(&response).unwrap();
        assert!((score - (1.0 - (-0.5_f64).exp())).abs() < 1e-10);
    }
}
