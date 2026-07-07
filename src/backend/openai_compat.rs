use std::sync::RwLock;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::models::Message;

use super::ChatBackend;

pub struct OpenAiCompatBackend {
    #[allow(dead_code)]
    pub provider_name: String,
    pub api_key: String,
    pub base_url: String,
    models: RwLock<Vec<String>>,
    client: reqwest::Client,
}

impl OpenAiCompatBackend {
    pub fn new(
        provider_name: String,
        api_key: String,
        base_url: String,
        models: Vec<String>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to build reqwest client");

        Self {
            provider_name,
            api_key,
            base_url: base_url.trim_end_matches('/').to_string(),
            models: RwLock::new(models),
            client,
        }
    }
}

#[async_trait]
impl ChatBackend for OpenAiCompatBackend {
    fn name(&self) -> &str {
        &self.provider_name
    }

    fn models(&self) -> Vec<String> {
        self.models.read().unwrap().clone()
    }

    fn set_models(&self, models: Vec<String>) {
        *self.models.write().unwrap() = models;
    }

    async fn fetch_models(&self) -> anyhow::Result<Vec<String>> {
        let url = format!("{}/models", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;
        let json: serde_json::Value = resp.json().await?;
        let models = json["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["id"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Ok(models)
    }

    async fn chat_stream(
        &self,
        model: &str,
        messages: &[Message],
        tx: mpsc::UnboundedSender<String>,
    ) -> anyhow::Result<()> {
        let url = format!("{}/chat/completions", self.base_url);

        let body = serde_json::json!({
            "model": model,
            "messages": messages.iter().map(|m| {
                serde_json::json!({
                    "role": m.role.as_str(),
                    "content": m.content,
                })
            }).collect::<Vec<_>>(),
            "stream": true,
        });

        let mut event_source = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&body)
            .send()
            .await?
            .bytes_stream();

        let mut buffer = String::new();
        while let Some(chunk) = futures::StreamExt::next(&mut event_source).await {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            buffer.push_str(&text);

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                if line == "data: [DONE]" {
                    return Ok(());
                }

                if let Some(json_str) = line.strip_prefix("data: ") {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                        if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                            let _ = tx.send(content.to_string());
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
