//! `~/.chimera/config.toml` — 与 `crucible_core` Python `get_config_path()` 同一文件。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use toml::Value as TomlValue;

use crate::platform::get_config_path;

fn default_oligo_host() -> String {
    "127.0.0.1".to_string()
}

fn default_oligo_port() -> u16 {
    33333
}

fn default_temperature() -> f64 {
    0.7
}

fn default_timeout_seconds() -> u64 {
    90
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct SystemConfig {
    #[serde(default)]
    pub vault_root: Option<String>,
    #[serde(default)]
    pub skills_dir: Option<String>,
    #[serde(default)]
    pub log_level: Option<String>,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            vault_root: None,
            skills_dir: None,
            log_level: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct OligoConfig {
    #[serde(default = "default_oligo_host")]
    pub host: String,
    #[serde(default = "default_oligo_port")]
    pub port: u16,
}

impl Default for OligoConfig {
    fn default() -> Self {
        Self {
            host: default_oligo_host(),
            port: default_oligo_port(),
        }
    }
}

/// `[astrocyte]` 表；与 `settings::AstrocyteConfig`（UI JSON）区分。
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ChimeraAstrocyteSection {
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub enable_clipboard_capture: Option<bool>,
}

impl Default for ChimeraAstrocyteSection {
    fn default() -> Self {
        Self {
            theme: None,
            enable_clipboard_capture: None,
        }
    }
}

/// 与 Python `LLMProviderSlotConfig` / `[llm.providers.*]` 对齐。
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(default)]
pub struct LlmProviderConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
}

impl Default for LlmProviderConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            api_key: String::new(),
            base_url: String::new(),
            model: String::new(),
            temperature: default_temperature(),
            timeout_seconds: default_timeout_seconds(),
        }
    }
}

/// `[llm]`：吞掉 `working` / `wash` / `router` 等 Python 专用表，仅解析 `providers`。
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct LlmConfig {
    #[serde(default)]
    pub working: Option<TomlValue>,
    #[serde(default)]
    pub wash: Option<TomlValue>,
    #[serde(default)]
    pub router: Option<TomlValue>,
    #[serde(default)]
    pub providers: HashMap<String, LlmProviderConfig>,
}

fn builtin_provider(id: &str) -> Option<LlmProviderConfig> {
    match id {
        "openai" => Some(LlmProviderConfig {
            name: "OpenAI".into(),
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".into(),
            model: "gpt-4o".into(),
            temperature: 0.7,
            timeout_seconds: 90,
        }),
        "deepseek" => Some(LlmProviderConfig {
            name: "DeepSeek".into(),
            api_key: String::new(),
            base_url: "https://api.deepseek.com".into(),
            model: "deepseek-chat".into(),
            temperature: 0.7,
            timeout_seconds: 90,
        }),
        "anthropic" => Some(LlmProviderConfig {
            name: "Anthropic".into(),
            api_key: String::new(),
            base_url: "https://api.anthropic.com/v1".into(),
            model: "claude-3-5-sonnet-20241022".into(),
            temperature: 0.7,
            timeout_seconds: 90,
        }),
        _ => None,
    }
}

impl LlmConfig {
    /// 内置三槽默认值，再由 `config.toml` 中 `[llm.providers.*]` 覆盖同名键。
    pub fn merged_providers(&self) -> HashMap<String, LlmProviderConfig> {
        let mut m = HashMap::new();
        for id in ["openai", "deepseek", "anthropic"] {
            if let Some(p) = builtin_provider(id) {
                m.insert(id.to_string(), p);
            }
        }
        for (k, v) in &self.providers {
            m.insert(k.clone(), v.clone());
        }
        m
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ChimeraConfig {
    #[serde(default)]
    pub system: SystemConfig,
    #[serde(default)]
    pub oligo: OligoConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub astrocyte: ChimeraAstrocyteSection,
}

impl Default for ChimeraConfig {
    fn default() -> Self {
        Self {
            system: SystemConfig::default(),
            oligo: OligoConfig::default(),
            llm: LlmConfig::default(),
            astrocyte: ChimeraAstrocyteSection::default(),
        }
    }
}

impl ChimeraConfig {
    /// `http://{host}:{port}`，与 `llm_client::build_oligo_invoke_url` 期望的根 URL 一致。
    pub fn oligo_base_url(&self) -> String {
        format!(
            "http://{}:{}",
            self.oligo.host.trim(),
            self.oligo.port
        )
    }

    /// 调试用：完整 invoke URL（与 Python Oligo 路由一致）。
    pub fn oligo_agent_invoke_url(&self) -> String {
        format!(
            "http://{}:{}/v1/agent/invoke",
            self.oligo.host.trim(),
            self.oligo.port
        )
    }
}

/// 从 `get_config_path()` 读取并解析 TOML。
pub fn load_config() -> Result<ChimeraConfig, String> {
    let config_path = get_config_path()?;
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read {}: {}", config_path.display(), e))?;
    toml::from_str(&content).map_err(|e| format!("TOML parse error in {}: {}", config_path.display(), e))
}
