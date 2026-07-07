use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "system")]
    System,
}

impl Role {
    pub fn as_str(&self) -> &str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn new(role: Role, content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            content,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub session_id: Uuid,
    pub provider: String,
    pub model: String,
    pub user_message: Message,
    pub ai_message: Message,
}
