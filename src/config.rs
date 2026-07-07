use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    pub api_key: Option<String>,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub models: Vec<String>,
}

fn default_base_url() -> String {
    String::new()
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_theme")]
    pub theme: String,
    pub default_provider: String,
    pub default_model: String,
    #[serde(flatten)]
    pub providers: HashMap<String, ProviderConfig>,
}

fn default_theme() -> String {
    "dark".into()
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let path = config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let mut config: Config = toml::from_str(&content)?;
            inject_env_api_keys(&mut config);
            clean_placeholder_keys(&mut config);
            Ok(config)
        } else {
            let dir = path.parent().unwrap();
            std::fs::create_dir_all(dir)?;
            std::fs::write(&path, DEFAULT_CONFIG)?;
            eprintln!(
                "Created default config at {}\nEdit it to add your API keys, then run again.",
                path.display()
            );
            std::process::exit(0);
        }
    }

    pub fn provider(&self, name: &str) -> Option<&ProviderConfig> {
        self.providers.get(name)
    }
}

fn config_path() -> PathBuf {
    std::env::var("FORGET_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            directories::ProjectDirs::from("", "", "forget")
                .map(|d| d.config_dir().join("config.toml"))
                .unwrap_or_else(|| PathBuf::from("config.toml"))
        })
}

fn inject_env_api_keys(config: &mut Config) {
    let env_map: HashMap<&str, &str> = HashMap::from([
        ("openai", "OPENAI_API_KEY"),
        ("deepseek", "DEEPSEEK_API_KEY"),
        ("qwen", "QWEN_API_KEY"),
        ("anthropic", "ANTHROPIC_API_KEY"),
    ]);

    for (provider_name, env_var) in &env_map {
        if let Some(provider) = config.providers.get_mut(*provider_name) {
            if provider.api_key.is_none() {
                if let Ok(key) = std::env::var(env_var) {
                    provider.api_key = Some(key);
                }
            }
        }
    }
}

fn clean_placeholder_keys(config: &mut Config) {
    for (_, provider) in config.providers.iter_mut() {
        if let Some(ref key) = provider.api_key {
            if key.is_empty() || key == "sk-..." || key == "sk-ant-..." {
                provider.api_key = None;
            }
        }
    }
}

const DEFAULT_CONFIG: &str = r##"# Forget TUI Chat Configuration
# Fill in the api_key for providers you want to use.
# Models are fetched automatically from the API on startup.
# Theme: dark | light | dracula | nord

theme = "dark"
default_provider = "deepseek"
default_model = "deepseek-chat"

[openai]
api_key = "sk-..."
base_url = "https://api.openai.com/v1"
models = ["gpt-4o", "gpt-4o-mini"]

[deepseek]
api_key = "sk-..."
base_url = "https://api.deepseek.com/v1"
models = ["deepseek-chat", "deepseek-reasoner"]

[qwen]
api_key = "sk-..."
base_url = "https://dashscope.aliyuncs.com/compatible-mode/v1"
models = ["qwen-turbo", "qwen-plus", "qwen-max"]

[anthropic]
api_key = "sk-ant-..."
models = ["claude-sonnet-4-20250514"]

[ollama]
base_url = "http://localhost:11434"
models = ["llama3:8b"]
"##;

