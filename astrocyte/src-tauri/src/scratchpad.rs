//! Structured scratchpad notes: domain model and file persistence under local data dir.

use std::fs;
use std::path::PathBuf;

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const SCRATCHPAD_FILE: &str = "scratchpad.json";

/// JSON 与前端契约：camelCase 键名（Tauri IPC / WebView）。`alias` 兼容旧版 `scratchpad.json` 的 snake_case。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScratchpadNote {
    pub id: String, // Uuid::new_v4().to_string()
    pub content: String,
    #[serde(rename = "createdAt", alias = "created_at")]
    pub created_at: String, // ISO 8601
    #[serde(rename = "updatedAt", alias = "updated_at")]
    pub updated_at: String,
    #[serde(rename = "contextId", alias = "context_id", default)]
    pub context_id: Option<String>, // 番茄钟联动；Rust 字段仍 snake_case
    #[serde(rename = "focusDurationMins", alias = "focus_duration_mins", default)]
    pub focus_duration_mins: Option<u32>,
}

/// New scratchpad note id (UUID v4 string).
pub fn new_scratchpad_note_id() -> String {
    Uuid::new_v4().to_string()
}

/// Current time as ISO 8601 (RFC 3339 with millis), suitable for `created_at` / `updated_at`.
pub fn scratchpad_now_iso8601() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn get_scratchpad_file_path() -> Result<PathBuf, String> {
    Ok(crate::platform::get_chimera_root()?.join(SCRATCHPAD_FILE))
}

/// Loads notes from `scratchpad.json`. Returns `[]` if the file is missing or invalid.
pub fn load_scratchpad_notes() -> Vec<ScratchpadNote> {
    let Ok(path) = get_scratchpad_file_path() else {
        return Vec::new();
    };
    if !path.exists() {
        return Vec::new();
    }
    match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

/// Writes the full note list as pretty-printed JSON, overwriting the file.
pub fn save_scratchpad_notes(notes: &[ScratchpadNote]) -> Result<(), String> {
    let path = get_scratchpad_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create scratchpad directory: {}", e))?;
    }
    let json = serde_json::to_string_pretty(notes)
        .map_err(|e| format!("failed to serialize scratchpad: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("failed to write scratchpad: {}", e))?;
    Ok(())
}
