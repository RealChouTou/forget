use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub provider: String,
    pub model: String,
    pub theme: String,
}

fn state_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "forget")
        .map(|d| d.config_dir().join("state.json"))
        .unwrap_or_else(|| PathBuf::from("state.json"))
}

impl AppState {
    pub fn load() -> Option<Self> {
        let path = state_path();
        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        } else {
            None
        }
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            if let Some(dir) = state_path().parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(state_path(), json);
        }
    }
}
