use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

const MEMORY_PATH_ENV_KEY: &str = "ASTROCYTE_MEMORY_PATH";
const DEFAULT_APP_DIR: &str = "chimera";
const DEFAULT_HISTORY_SUBDIR: &str = "history";
const DEFAULT_MEMORY_FILE: &str = "astrocyte_memory.jsonl";

fn get_history_file_path() -> Result<PathBuf, String> {
    if let Ok(val) = env::var(MEMORY_PATH_ENV_KEY) {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    let mut default_path = dirs::data_local_dir()
        .ok_or_else(|| "cannot find OS local data directory".to_string())?;
    default_path.push(DEFAULT_APP_DIR);
    default_path.push(DEFAULT_HISTORY_SUBDIR);
    fs::create_dir_all(&default_path)
        .map_err(|e| format!("failed to create default memory directory: {}", e))?;
    default_path.push(DEFAULT_MEMORY_FILE);

    Ok(default_path)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatEntry {
    pub id: String, // uuid
    pub timestamp: String,
    pub role: String, // "user", "bb", "system"
    pub content: String,
    #[serde(default)]
    pub session_id: String,
    // 预留的扩展口，比如当前是哪个人格
    #[serde(default)]
    pub persona: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SessionSummary {
    pub id: String,
    pub timestamp: String,
    pub first_user_message_snippet: String,
}

#[derive(Deserialize)]
struct RawChatEntry {
    id: Option<String>,
    timestamp: Option<String>,
    role: Option<String>,
    content: Option<String>,
    session_id: Option<String>,
    persona: Option<String>,
}

fn now_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn parse_rfc3339_to_utc(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn extract_day_from_timestamp(raw: &str) -> Option<String> {
    parse_rfc3339_to_utc(raw).map(|dt| dt.format("%Y-%m-%d").to_string())
}

fn normalize_snippet(raw: &str) -> String {
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut snippet = collapsed;
    let max_chars = 96usize;
    if snippet.chars().count() > max_chars {
        snippet = snippet.chars().take(max_chars).collect::<String>();
        snippet.push('…');
    }
    snippet
}

fn normalize_raw_entry(raw: RawChatEntry, line_index: usize) -> Option<ChatEntry> {
    let role = raw.role?.trim().to_string();
    if role.is_empty() {
        return None;
    }

    let content = raw.content.unwrap_or_default();
    let timestamp = raw
        .timestamp
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(now_timestamp);

    let session_id = raw
        .session_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .or_else(|| extract_day_from_timestamp(&timestamp).map(|day| format!("legacy-day-{}", day)))
        .unwrap_or_else(|| format!("legacy-line-{:06}", line_index + 1));

    let id = raw
        .id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(|| format!("legacy-entry-{:06}", line_index + 1));

    Some(ChatEntry {
        id,
        timestamp,
        role,
        content,
        session_id,
        persona: raw.persona,
    })
}

pub fn read_all_entries() -> Result<Vec<ChatEntry>, String> {
    let file_path = get_history_file_path()?;
    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let raw_content =
        fs::read_to_string(&file_path).map_err(|e| format!("failed to read memory file: {}", e))?;
    let mut entries = Vec::new();
    for (line_index, line) in raw_content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let raw_entry = match serde_json::from_str::<RawChatEntry>(trimmed) {
            Ok(value) => value,
            Err(err) => {
                eprintln!(
                    "[astrocyte] skip malformed memory line {}: {}",
                    line_index + 1,
                    err
                );
                continue;
            }
        };
        if let Some(entry) = normalize_raw_entry(raw_entry, line_index) {
            entries.push(entry);
        }
    }
    Ok(entries)
}

pub fn get_session_summaries() -> Result<Vec<SessionSummary>, String> {
    let entries = read_all_entries()?;
    if entries.is_empty() {
        return Ok(Vec::new());
    }

    let mut buckets = std::collections::HashMap::<String, Vec<ChatEntry>>::new();
    for entry in entries {
        buckets
            .entry(entry.session_id.clone())
            .or_default()
            .push(entry);
    }

    let mut summaries: Vec<SessionSummary> = buckets
        .into_iter()
        .map(|(session_id, mut items)| {
            items.sort_by(|a, b| {
                let a_dt = parse_rfc3339_to_utc(&a.timestamp);
                let b_dt = parse_rfc3339_to_utc(&b.timestamp);
                match (a_dt, b_dt) {
                    (Some(left), Some(right)) => left.cmp(&right),
                    _ => a.timestamp.cmp(&b.timestamp),
                }
            });

            let timestamp = items
                .first()
                .map(|entry| entry.timestamp.clone())
                .unwrap_or_else(now_timestamp);
            let snippet = items
                .iter()
                .find(|entry| entry.role == "user")
                .map(|entry| normalize_snippet(&entry.content))
                .unwrap_or_else(|| "[NO_USER_MESSAGE]".to_string());

            SessionSummary {
                id: session_id,
                timestamp,
                first_user_message_snippet: snippet,
            }
        })
        .collect();

    summaries.sort_by(|a, b| {
        let a_dt = parse_rfc3339_to_utc(&a.timestamp);
        let b_dt = parse_rfc3339_to_utc(&b.timestamp);
        match (a_dt, b_dt) {
            (Some(left), Some(right)) => left.cmp(&right),
            _ => a.timestamp.cmp(&b.timestamp),
        }
    });

    Ok(summaries)
}

pub fn get_entries_for_session(session_id: &str) -> Result<Vec<ChatEntry>, String> {
    let mut entries: Vec<ChatEntry> = read_all_entries()?
        .into_iter()
        .filter(|entry| entry.session_id == session_id)
        .collect();

    entries.sort_by(|a, b| {
        let a_dt = parse_rfc3339_to_utc(&a.timestamp);
        let b_dt = parse_rfc3339_to_utc(&b.timestamp);
        match (a_dt, b_dt) {
            (Some(left), Some(right)) => left.cmp(&right),
            _ => a.timestamp.cmp(&b.timestamp),
        }
    });
    Ok(entries)
}

pub fn append_entries(entries: &[ChatEntry]) -> Result<(), String> {
    let file_path = get_history_file_path()?;

    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create memory directory: {}", e))?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)
        .map_err(|e| format!("failed to open memory file: {}", e))?;

    for entry in entries {
        let line = serde_json::to_string(entry)
            .map_err(|e| format!("failed to serialize chat entry: {}", e))?;
        file.write_all(line.as_bytes())
            .map_err(|e| format!("failed to write memory line: {}", e))?;
        file.write_all(b"\n")
            .map_err(|e| format!("failed to write newline: {}", e))?;
    }

    file.flush()
        .map_err(|e| format!("failed to flush memory file: {}", e))?;

    Ok(())
}
