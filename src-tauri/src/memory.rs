//! Session-scoped memory persistence.
//! Each session is stored as an independent `{session_id}.jsonl` file under `sessions/`.

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

const MEMORY_PATH_ENV_KEY: &str = "ASTROCYTE_MEMORY_PATH";
const DEFAULT_APP_DIR: &str = "chimera";
const DEFAULT_HISTORY_SUBDIR: &str = "history";
const SESSIONS_SUBDIR: &str = "sessions";
const LEGACY_MEMORY_FILE: &str = "astrocyte_memory.jsonl";

/// Base directory for all memory data (sessions/ lives here).
fn get_memory_base_dir() -> Result<PathBuf, String> {
    if let Ok(val) = env::var(MEMORY_PATH_ENV_KEY) {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            fs::create_dir_all(&path)
                .map_err(|e| format!("failed to create memory directory: {}", e))?;
            return Ok(path);
        }
    }

    let mut default_path = dirs::data_local_dir()
        .ok_or_else(|| "cannot find OS local data directory".to_string())?;
    default_path.push(DEFAULT_APP_DIR);
    default_path.push(DEFAULT_HISTORY_SUBDIR);
    fs::create_dir_all(&default_path)
        .map_err(|e| format!("failed to create default memory directory: {}", e))?;
    Ok(default_path)
}

/// Path to the sessions directory.
fn get_sessions_dir() -> Result<PathBuf, String> {
    let mut path = get_memory_base_dir()?;
    path.push(SESSIONS_SUBDIR);
    fs::create_dir_all(&path)
        .map_err(|e| format!("failed to create sessions directory: {}", e))?;
    Ok(path)
}

/// Path to a specific session file. Session IDs are sanitized to avoid path traversal.
pub fn get_session_file_path(session_id: &str) -> Result<PathBuf, String> {
    let sanitized = session_id
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect::<String>();
    if sanitized.is_empty() {
        return Err("session_id is empty after sanitization".to_string());
    }
    let mut path = get_sessions_dir()?;
    path.push(format!("{}.jsonl", sanitized));
    Ok(path)
}

/// Legacy monolithic file path (for migration only).
fn get_legacy_file_path() -> Result<PathBuf, String> {
    let mut path = get_memory_base_dir()?;
    path.push(LEGACY_MEMORY_FILE);
    Ok(path)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatEntry {
    pub id: String,
    pub timestamp: String,
    pub role: String, // "user", "bb", "assistant", "system"
    pub content: String,
    #[serde(default)]
    pub session_id: String,
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

fn normalize_raw_entry(raw: RawChatEntry, line_index: usize, session_id: &str) -> Option<ChatEntry> {
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
        session_id: session_id.to_string(),
        persona: raw.persona,
    })
}

/// Migrate legacy monolithic file to session-scoped files (one-time).
fn migrate_legacy_if_exists() -> Result<(), String> {
    let legacy_path = get_legacy_file_path()?;
    if !legacy_path.exists() {
        return Ok(());
    }

    let raw_content = fs::read_to_string(&legacy_path)
        .map_err(|e| format!("failed to read legacy memory file: {}", e))?;

    let mut buckets: std::collections::HashMap<String, Vec<ChatEntry>> =
        std::collections::HashMap::new();

    for (line_index, line) in raw_content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let raw_entry = match serde_json::from_str::<RawChatEntry>(trimmed) {
            Ok(value) => value,
            Err(err) => {
                eprintln!(
                    "[astrocyte] skip malformed legacy line {}: {}",
                    line_index + 1,
                    err
                );
                continue;
            }
        };

        let session_id = raw_entry
            .session_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
            .or_else(|| {
                raw_entry.timestamp.as_ref().and_then(|ts| {
                    extract_day_from_timestamp(ts).map(|day| format!("legacy-day-{}", day))
                })
            })
            .unwrap_or_else(|| format!("legacy-line-{:06}", line_index + 1));

        if let Some(entry) = normalize_raw_entry(raw_entry, line_index, &session_id) {
            buckets.entry(session_id).or_default().push(entry);
        }
    }

    for (session_id, mut entries) in buckets {
        entries.sort_by(|a, b| {
            let a_dt = parse_rfc3339_to_utc(&a.timestamp);
            let b_dt = parse_rfc3339_to_utc(&b.timestamp);
            match (a_dt, b_dt) {
                (Some(left), Some(right)) => left.cmp(&right),
                _ => a.timestamp.cmp(&b.timestamp),
            }
        });
        save_session_entries(&session_id, &entries)?;
    }

    fs::remove_file(&legacy_path)
        .map_err(|e| format!("failed to remove legacy file after migration: {}", e))?;
    Ok(())
}

/// Ensures migration has run once. Call at app startup.
pub fn ensure_migration() -> Result<(), String> {
    migrate_legacy_if_exists()
}

/// Save all entries for a session to its dedicated file (overwrites).
pub fn save_session_entries(session_id: &str, entries: &[ChatEntry]) -> Result<(), String> {
    let file_path = get_session_file_path(session_id)?;

    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create sessions directory: {}", e))?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&file_path)
        .map_err(|e| format!("failed to open session file: {}", e))?;

    for entry in entries {
        let mut entry_with_session = entry.clone();
        entry_with_session.session_id = session_id.to_string();
        let line = serde_json::to_string(&entry_with_session)
            .map_err(|e| format!("failed to serialize chat entry: {}", e))?;
        file.write_all(line.as_bytes())
            .map_err(|e| format!("failed to write memory line: {}", e))?;
        file.write_all(b"\n")
            .map_err(|e| format!("failed to write newline: {}", e))?;
    }

    file.flush()
        .map_err(|e| format!("failed to flush session file: {}", e))?;

    Ok(())
}

/// Physically delete a session file. Returns Ok if file did not exist.
pub fn delete_session_file(session_id: &str) -> Result<(), String> {
    let path = get_session_file_path(session_id)?;
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| format!("failed to remove session file: {}", e))?;
    }
    Ok(())
}

/// Delete a single entry from a session file by id. Rewrites the file.
pub fn delete_entry(session_id: &str, entry_id: &str) -> Result<(), String> {
    let file_path = get_session_file_path(session_id)?;
    if !file_path.exists() {
        return Ok(());
    }

    let file = fs::File::open(&file_path)
        .map_err(|e| format!("failed to open session file for delete: {}", e))?;
    let reader = BufReader::new(file);

    let mut entries: Vec<ChatEntry> = Vec::new();
    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| format!("failed to read session file line: {}", e))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let raw: RawChatEntry = serde_json::from_str(trimmed)
            .map_err(|e| format!("failed to parse session line {}: {}", line_index + 1, e))?;
        let id = raw
            .id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("");
        if id == entry_id {
            continue;
        }
        if let Some(entry) = normalize_raw_entry(raw, line_index, session_id) {
            entries.push(entry);
        }
    }

    save_session_entries(session_id, &entries)
}

/// Traverse all session files and return summaries, sorted by timestamp descending.
pub fn get_timeline_summaries() -> Result<Vec<SessionSummary>, String> {
    ensure_migration()?;

    let sessions_dir = get_sessions_dir()?;
    let mut summaries: Vec<SessionSummary> = Vec::new();

    let entries = fs::read_dir(&sessions_dir)
        .map_err(|e| format!("failed to read sessions directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read dir entry: {}", e))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension() else {
            continue;
        };
        if ext != "jsonl" {
            continue;
        }
        let Some(stem) = path.file_stem() else {
            continue;
        };
        let session_id = stem.to_string_lossy().to_string();

        let mut entries = get_entries_for_session(&session_id)?;
        if entries.is_empty() {
            continue;
        }

        entries.sort_by(|a, b| {
            let a_dt = parse_rfc3339_to_utc(&a.timestamp);
            let b_dt = parse_rfc3339_to_utc(&b.timestamp);
            match (a_dt, b_dt) {
                (Some(left), Some(right)) => left.cmp(&right),
                _ => a.timestamp.cmp(&b.timestamp),
            }
        });

        let timestamp = entries
            .first()
            .map(|e| e.timestamp.clone())
            .unwrap_or_else(now_timestamp);
        let snippet = entries
            .iter()
            .find(|e| e.role == "user")
            .map(|e| normalize_snippet(&e.content))
            .unwrap_or_else(|| "[NO_USER_MESSAGE]".to_string());

        summaries.push(SessionSummary {
            id: session_id,
            timestamp,
            first_user_message_snippet: snippet,
        });
    }

    summaries.sort_by(|a, b| {
        let a_dt = parse_rfc3339_to_utc(&a.timestamp);
        let b_dt = parse_rfc3339_to_utc(&b.timestamp);
        match (a_dt, b_dt) {
            (Some(left), Some(right)) => right.cmp(&left),
            _ => b.timestamp.cmp(&a.timestamp),
        }
    });

    Ok(summaries)
}

/// Read all entries for a session from its file.
pub fn get_entries_for_session(session_id: &str) -> Result<Vec<ChatEntry>, String> {
    let file_path = get_session_file_path(session_id)?;
    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(&file_path)
        .map_err(|e| format!("failed to open session file: {}", e))?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| format!("failed to read session file: {}", e))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let raw = match serde_json::from_str::<RawChatEntry>(trimmed) {
            Ok(v) => v,
            Err(err) => {
                eprintln!(
                    "[astrocyte] skip malformed session line {}: {}",
                    line_index + 1,
                    err
                );
                continue;
            }
        };
        if let Some(entry) = normalize_raw_entry(raw, line_index, session_id) {
            entries.push(entry);
        }
    }

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

/// Append entries to a session file (used when streaming completes). Merges with existing.
pub fn append_session_entries(session_id: &str, entries: &[ChatEntry]) -> Result<(), String> {
    let mut existing = get_entries_for_session(session_id)?;
    for entry in entries {
        let mut e = entry.clone();
        e.session_id = session_id.to_string();
        existing.push(e);
    }
    existing.sort_by(|a, b| {
        let a_dt = parse_rfc3339_to_utc(&a.timestamp);
        let b_dt = parse_rfc3339_to_utc(&b.timestamp);
        match (a_dt, b_dt) {
            (Some(left), Some(right)) => left.cmp(&right),
            _ => a.timestamp.cmp(&b.timestamp),
        }
    });
    save_session_entries(session_id, &existing)
}
