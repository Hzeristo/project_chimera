use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

const DEFAULT_APP_DIR: &str = "chimera";
const PROVIDER_CONFIG_FILE: &str = "provider_config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub api_key: String,
    pub base_url: String,
    pub model_name: String,
}

impl ProviderConfig {
    pub fn normalized(mut self) -> Self {
        self.id = self.id.trim().to_string();
        self.name = self.name.trim().to_string();
        self.api_key = self.api_key.trim().to_string();
        self.base_url = self.base_url.trim().to_string();
        self.model_name = self.model_name.trim().to_string();
        if self.id.is_empty() {
            self.id = Uuid::new_v4().to_string();
        }
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("provider id cannot be empty".to_string());
        }
        if self.name.trim().is_empty() {
            return Err("provider name cannot be empty".to_string());
        }
        if self.base_url.trim().is_empty() {
            return Err("base_url cannot be empty".to_string());
        }
        if self.api_key.trim().is_empty() {
            return Err("api_key cannot be empty".to_string());
        }
        if self.model_name.trim().is_empty() {
            return Err("model_name cannot be empty".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AstrocyteConfig {
    pub active_provider_id: Option<String>,
    pub providers: Vec<ProviderConfig>,
    #[serde(default)]
    pub is_oligo_mode: bool,
    #[serde(default)]
    pub active_skill_id: Option<String>,
}

impl AstrocyteConfig {
    pub fn normalized(mut self) -> Self {
        self.providers = self
            .providers
            .into_iter()
            .map(ProviderConfig::normalized)
            .filter(|provider| !provider.id.is_empty())
            .collect();
        self.active_provider_id = self
            .active_provider_id
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty());

        if let Some(active_id) = &self.active_provider_id {
            if !self.providers.iter().any(|provider| &provider.id == active_id) {
                self.active_provider_id = None;
            }
        }
        self
    }

    pub fn active_provider(&self) -> Option<&ProviderConfig> {
        let active_id = self.active_provider_id.as_ref()?;
        self.providers.iter().find(|provider| &provider.id == active_id)
    }
}

pub fn provider_config_path() -> Result<PathBuf, String> {
    let mut base = dirs::data_local_dir()
        .ok_or_else(|| "cannot find OS local data directory".to_string())?;
    base.push(DEFAULT_APP_DIR);
    fs::create_dir_all(&base).map_err(|e| format!("failed to create app data directory: {}", e))?;
    base.push(PROVIDER_CONFIG_FILE);
    Ok(base)
}

pub fn load_astrocyte_config() -> AstrocyteConfig {
    let path = match provider_config_path() {
        Ok(path) => path,
        Err(_) => return AstrocyteConfig::default(),
    };

    let raw = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return AstrocyteConfig::default(),
    };

    serde_json::from_str::<AstrocyteConfig>(&raw)
        .map(AstrocyteConfig::normalized)
        .unwrap_or_default()
}

pub fn save_astrocyte_config(config: &AstrocyteConfig) -> Result<(), String> {
    let normalized = config.clone().normalized();
    let path = provider_config_path()?;
    let payload = serde_json::to_string_pretty(&normalized)
        .map_err(|e| format!("failed to serialize astrocyte config: {}", e))?;
    fs::write(path, payload).map_err(|e| format!("failed to write astrocyte config: {}", e))
}
