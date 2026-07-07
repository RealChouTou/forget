use async_trait::async_trait;

use crate::models::Message;

pub mod anthropic;
pub mod ollama;
pub mod openai_compat;

#[async_trait]
pub trait ChatBackend: Send + Sync {
    #[allow(dead_code)]
    fn name(&self) -> &str;

    fn models(&self) -> Vec<String>;

    async fn fetch_models(&self) -> anyhow::Result<Vec<String>>;

    fn set_models(&self, models: Vec<String>);

    async fn chat_stream(
        &self,
        model: &str,
        messages: &[Message],
        tx: tokio::sync::mpsc::UnboundedSender<String>,
    ) -> anyhow::Result<()>;
}
