use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::PathBuf;

use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{ChimeraConfig, LlmProviderConfig};

const PROVIDER_CONFIG_FILE: &str = "provider_config.json";

fn default_oligo_base_url() -> String {
    "http://127.0.0.1:33333".to_string()
}

/// 优先级：`OLIGO_BASE_URL` → `~/.chimera/config.toml` 中 `[oligo]` → ``config.oligo_base_url``（旧 JSON）。
pub fn effective_oligo_base_url(config: &AstrocyteConfig, chimera: &ChimeraConfig) -> String {
    if let Ok(v) = std::env::var("OLIGO_BASE_URL") {
        let t = v.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    let from_chimera = chimera.oligo_base_url();
    let json = config.oligo_base_url.trim();
    if !json.is_empty() && json != from_chimera {
        return json.to_string();
    }
    from_chimera
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub api_key: String,
    pub base_url: String,
    pub model_name: String,
    /// Oligo 每请求温度；`None` 时请求 JSON 省略该键，由 Oligo 使用 Chimera `llm.working` 默认温度。
    #[serde(default)]
    pub temperature: Option<f64>,
}

impl ProviderConfig {
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

/// `provider_config.json`：仅 UI 状态；Provider 本体见 `~/.chimera/config.toml` `[llm.providers.*]`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AstrocyteConfig {
    pub active_provider_id: Option<String>,
    #[serde(default)]
    pub is_oligo_mode: bool,
    #[serde(default)]
    pub active_skill_id: Option<String>,
    #[serde(default = "default_oligo_base_url")]
    pub oligo_base_url: String,
}

impl Default for AstrocyteConfig {
    fn default() -> Self {
        Self {
            active_provider_id: None,
            is_oligo_mode: false,
            active_skill_id: None,
            oligo_base_url: default_oligo_base_url(),
        }
    }
}

impl AstrocyteConfig {
    pub fn normalized(mut self) -> Self {
        self.active_provider_id = self
            .active_provider_id
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty());

        self.oligo_base_url = self.oligo_base_url.trim().to_string();
        if self.oligo_base_url.is_empty() {
            self.oligo_base_url = default_oligo_base_url();
        }
        self
    }
}

pub fn default_active_provider_id(chimera: &ChimeraConfig) -> Option<String> {
    let merged = chimera.llm.merged_providers();
    for key in ["openai", "deepseek", "anthropic"] {
        if merged.contains_key(key) {
            return Some(key.to_string());
        }
    }
    merged.keys().next().cloned()
}

/// 将 `active_provider_id` 约束为 TOML 中存在的槽位；必要时回落到默认槽。
pub fn normalize_astrocyte_with_chimera(config: &mut AstrocyteConfig, chimera: &ChimeraConfig) {
    let merged = chimera.llm.merged_providers();
    if let Some(ref id) = config.active_provider_id {
        if !merged.contains_key(id) {
            config.active_provider_id = None;
        }
    }
    if config.active_provider_id.is_none() {
        config.active_provider_id = default_active_provider_id(chimera);
    }
}

/// 供 UI / `ping` 展示：TOML 明文密钥优先，否则回落环境变量（无则空字符串）。
pub fn resolved_provider_api_key(provider_id: &str, slot: &LlmProviderConfig) -> String {
    resolve_provider_api_key(provider_id, slot).unwrap_or_default()
}

fn resolve_provider_api_key(provider_id: &str, slot: &LlmProviderConfig) -> Option<String> {
    let t = slot.api_key.trim();
    if !t.is_empty() {
        return Some(t.to_string());
    }
    let pid = provider_id.to_lowercase();
    let raw = match pid.as_str() {
        "openai" => std::env::var("OPENAI_API_KEY").ok(),
        "deepseek" => std::env::var("DEEPSEEK_API_KEY").ok(),
        "anthropic" => std::env::var("ANTHROPIC_API_KEY").ok(),
        "gemini" => std::env::var("GEMINI_API_KEY").ok(),
        _ => None,
    }?;
    let t = raw.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

/// 由 UI 状态 + `config.toml` 合并出当前 Working 直连 / Oligo 请求所用凭据。
pub fn resolve_active_provider_runtime(
    ui: &AstrocyteConfig,
    chimera: &ChimeraConfig,
) -> Option<ProviderConfig> {
    let id = ui.active_provider_id.as_ref()?;
    let merged = chimera.llm.merged_providers();
    let slot = merged.get(id)?;
    let api_key = resolve_provider_api_key(id, slot)?;
    let name = if slot.name.trim().is_empty() {
        id.clone()
    } else {
        slot.name.clone()
    };
    let model_name = slot.model.trim();
    if model_name.is_empty() {
        return None;
    }
    let base_url = slot.base_url.trim();
    if base_url.is_empty() {
        return None;
    }
    Some(ProviderConfig {
        id: id.clone(),
        name,
        api_key,
        base_url: base_url.to_string(),
        model_name: model_name.to_string(),
        temperature: Some(slot.temperature),
    })
}

pub fn provider_config_path() -> Result<PathBuf, String> {
    let root = crate::platform::get_chimera_root()?;
    Ok(root.join(PROVIDER_CONFIG_FILE))
}

fn astrocyte_from_json_value(mut v: Value) -> Result<AstrocyteConfig, String> {
    if let Value::Object(ref mut map) = v {
        map.remove("providers");
    }
    serde_json::from_value::<AstrocyteConfig>(v)
        .map(AstrocyteConfig::normalized)
        .map_err(|e| format!("failed to parse astrocyte config: {}", e))
}

/// 读取 `provider_config.json`（共享锁，与写入独占锁协调）。
pub fn load_astrocyte_config() -> Result<AstrocyteConfig, String> {
    let path = provider_config_path()?;

    if !path.is_file() {
        return Ok(AstrocyteConfig::default());
    }

    let mut file = OpenOptions::new()
        .read(true)
        .open(&path)
        .map_err(|e| format!("failed to open astrocyte config: {}", e))?;

    file.lock_shared()
        .map_err(|e| format!("failed to lock astrocyte config (shared): {}", e))?;

    let mut raw = String::new();
    file
        .read_to_string(&mut raw)
        .map_err(|e| format!("failed to read astrocyte config: {}", e))?;

    let value: Value =
        serde_json::from_str(&raw).map_err(|e| format!("failed to parse astrocyte config: {}", e))?;
    astrocyte_from_json_value(value)
}

/// 原子化写入 `provider_config.json`（独占锁；内容写入同一已加锁句柄，避免与 `fs::write` 双句柄竞态）。
pub fn save_astrocyte_config(config: &AstrocyteConfig) -> Result<(), String> {
    let normalized = config.clone().normalized();
    let path = provider_config_path()?;
    let payload = serde_json::to_string_pretty(&normalized)
        .map_err(|e| format!("failed to serialize astrocyte config: {}", e))?;

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .map_err(|e| format!("failed to open astrocyte config: {}", e))?;

    file.lock_exclusive()
        .map_err(|e| format!("failed to lock astrocyte config (exclusive): {}", e))?;

    file.write_all(payload.as_bytes())
        .map_err(|e| format!("failed to write astrocyte config: {}", e))?;
    file.flush()
        .map_err(|e| format!("failed to flush astrocyte config: {}", e))?;
    Ok(())
}
