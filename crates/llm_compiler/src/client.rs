use crate::LlmProvider;
use anyhow::{Result, anyhow};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::{Value, json};

pub struct LlmClient {
    client: reqwest::Client,
}

impl LlmClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn query(
        &self,
        provider: &LlmProvider,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String> {
        match provider {
            LlmProvider::OpenAi {
                api_key,
                model,
                base_url,
            } => {
                let mut url = base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string());
                if !url.contains("/chat/completions") {
                    url = format!("{}/chat/completions", url.trim_end_matches('/'));
                }
                let model_name = model.clone().unwrap_or_else(|| "gpt-4o-mini".to_string());

                let mut headers = HeaderMap::new();
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&format!("Bearer {}", api_key))?,
                );
                headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

                let body = json!({
                    "model": model_name,
                    "messages": [
                        { "role": "system", "content": system_prompt },
                        { "role": "user", "content": user_prompt }
                    ],
                    "temperature": 0.1,
                    "response_format": { "type": "json_object" }
                });

                let response = self
                    .client
                    .post(&url)
                    .headers(headers)
                    .json(&body)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    let status = response.status();
                    let err_text = response.text().await?;
                    return Err(anyhow!(
                        "OpenAI API error: {} (status: {})",
                        err_text,
                        status
                    ));
                }

                let res_json: Value = response.json().await?;
                let content = res_json["choices"][0]["message"]["content"]
                    .as_str()
                    .ok_or_else(|| {
                        anyhow!(
                            "Failed to parse content from OpenAI response: {:?}",
                            res_json
                        )
                    })?;

                Ok(content.to_string())
            }

            LlmProvider::Anthropic { api_key, model } => {
                let url = "https://api.anthropic.com/v1/messages";
                let model_name = model
                    .clone()
                    .unwrap_or_else(|| "claude-3-5-sonnet-20241022".to_string());

                let mut headers = HeaderMap::new();
                headers.insert("x-api-key", HeaderValue::from_str(api_key)?);
                headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
                headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

                let body = json!({
                    "model": model_name,
                    "max_tokens": 4096,
                    "system": system_prompt,
                    "messages": [
                        { "role": "user", "content": user_prompt }
                    ],
                    "temperature": 0.1
                });

                let response = self
                    .client
                    .post(url)
                    .headers(headers)
                    .json(&body)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    let status = response.status();
                    let err_text = response.text().await?;
                    return Err(anyhow!(
                        "Anthropic API error: {} (status: {})",
                        err_text,
                        status
                    ));
                }

                let res_json: Value = response.json().await?;
                let content = res_json["content"][0]["text"].as_str().ok_or_else(|| {
                    anyhow!(
                        "Failed to parse content from Anthropic response: {:?}",
                        res_json
                    )
                })?;

                Ok(content.to_string())
            }

            LlmProvider::Ollama { base_url, model } => {
                let url = format!("{}/api/chat", base_url.trim_end_matches('/'));

                let body = json!({
                    "model": model,
                    "messages": [
                        { "role": "system", "content": system_prompt },
                        { "role": "user", "content": user_prompt }
                    ],
                    "stream": false,
                    "options": {
                        "temperature": 0.1
                    }
                });

                let response = self.client.post(&url).json(&body).send().await?;

                if !response.status().is_success() {
                    let status = response.status();
                    let err_text = response.text().await?;
                    return Err(anyhow!(
                        "Ollama API error: {} (status: {})",
                        err_text,
                        status
                    ));
                }

                let res_json: Value = response.json().await?;
                let content = res_json["message"]["content"].as_str().ok_or_else(|| {
                    anyhow!(
                        "Failed to parse content from Ollama response: {:?}",
                        res_json
                    )
                })?;

                Ok(content.to_string())
            }
        }
    }
}
