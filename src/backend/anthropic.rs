use std::sync::RwLock;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::models::Message;

use super::ChatBackend;

pub struct AnthropicBackend {
    pub api_key: String,
    models: RwLock<Vec<String>>,
    client: reqwest::Client,
}

impl AnthropicBackend {
    pub fn new(api_key: String, models: Vec<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to build reqwest client");

        Self {
            api_key,
            models: RwLock::new(models),
            client,
        }
    }

    pub const BASE_URL: &str = "https://api.anthropic.com/v1";
    pub const ANTHROPIC_VERSION: &str = "2023-06-01";
}

#[async_trait]
impl ChatBackend for AnthropicBackend {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn models(&self) -> Vec<String> {
        self.models.read().unwrap().clone()
    }

    fn set_models(&self, models: Vec<String>) {
        *self.models.write().unwrap() = models;
    }

    async fn fetch_models(&self) -> anyhow::Result<Vec<String>> {
        Ok(self.models.read().unwrap().clone())
    }

    async fn chat_stream(
        &self,
        model: &str,
        messages: &[Message],
        tx: mpsc::UnboundedSender<String>,
    ) -> anyhow::Result<()> {
        let url = format!("{}/messages", Self::BASE_URL);

        let system_contents: Vec<&str> = messages
            .iter()
            .filter(|m| matches!(m.role, crate::models::Role::System))
            .map(|m| m.content.as_str())
            .collect();

        let chat_messages: Vec<&Message> = messages
            .iter()
            .filter(|m| !matches!(m.role, crate::models::Role::System))
            .collect();

        let system_content = if system_contents.is_empty() {
            None
        } else {
            Some(system_contents.join("\n"))
        };

        let messages_payload: Vec<serde_json::Value> = chat_messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": m.role.as_str(),
                    "content": m.content,
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages_payload,
            "stream": true,
            "max_tokens": 8192,
        });

        if let Some(system) = system_content {
            body["system"] = serde_json::json!(system);
        }

        let mut event_source = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", Self::ANTHROPIC_VERSION)
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

                if let Some(json_str) = line.strip_prefix("data: ") {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                        if parsed["type"] == "content_block_delta" {
                            if let Some(text) = parsed["delta"]["text"].as_str() {
                                let _ = tx.send(text.to_string());
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
