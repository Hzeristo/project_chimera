use arboard::Clipboard;
use chrono::{SecondsFormat, Utc};
use log::{debug, info, warn};
use memory::{
    append_session_entries, delete_entry, delete_session_file, get_entries_for_session,
    get_timeline_summaries, save_session_entries, Artifact, ChatEntry, SessionSummary,
};
use reqwest::header::AUTHORIZATION;
use std::time::Duration;
use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

mod llm_client;
mod memory;
mod scratchpad;
mod scratchpad_commands;
mod skill_stats;
mod skills;
mod stage_stats;
mod persona;
mod task_stream;
pub mod config;
pub mod platform;
mod settings;
mod state;
use persona::{PersonaConfig, PersonaSnapshot};
use serde::{Deserialize, Serialize};
use config::ChimeraConfig;
use settings::{
    load_astrocyte_config, normalize_astrocyte_with_chimera, resolve_active_provider_runtime,
    save_astrocyte_config,
    AstrocyteConfig,
};
use skills::SkillWithStats;
use state::{AstrocyteState, Message};

/// Payload for syncing session history with persistence (includes ids for delete/edit).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubmitSegmentFeedback {
    conversation_id: String,
    stage: String,
    tool_name: Option<String>,
    decision: Option<String>,
    rating: String,
    reason: Option<String>,
}

#[tauri::command]
fn submit_segment_feedback(payload: SubmitSegmentFeedback) -> Result<(), String> {
    stage_stats::record_segment_feedback(
        &payload.conversation_id,
        &payload.stage,
        payload.tool_name.as_deref(),
        payload.decision.as_deref(),
        &payload.rating,
        payload.reason.as_deref(),
    )
}

#[derive(Deserialize)]
struct SyncEntry {
    id: String,
    role: String,
    content: String,
    #[serde(default)]
    timestamp: Option<String>, 
    #[serde(default)]
    persona: Option<String>,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn process_signal(payload: String) -> String {
    format!(
        "[BB Intercepted]: 你的微弱脑电波已收到。你刚才说的是 -> {}",
        payload
    )
}

#[tauri::command]
async fn abort_generation(state: tauri::State<'_, AstrocyteState>) -> Result<(), String> {
    let mut guard = state.abort_token.write().await;
    if let Some(token) = guard.as_ref() {
        token.cancel();
    }
    *guard = None;
    Ok(())
}

#[tauri::command]
async fn get_config(state: tauri::State<'_, AstrocyteState>) -> Result<AstrocyteConfig, String> {
    Ok(state.config.read().await.clone())
}

/// 与 `crucible_core` `TaskService` 落盘 JSON 对齐；供 Miner 任务轮询（避免再走 Oligo 流式通道）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackgroundTaskStatus {
    id: String,
    #[serde(rename = "type")]
    task_type: String,
    status: String,
    progress: f64,
    result: Option<String>,
    error: Option<String>,
}

#[tauri::command]
fn get_background_task_status(task_id: String) -> Result<BackgroundTaskStatus, String> {
    let tid = task_id.trim();
    if tid.is_empty() {
        return Err("task_id is empty".to_string());
    }
    if tid.len() != 8 || !tid.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("invalid task_id".to_string());
    }
    let path = platform::get_chimera_root()?.join("tasks").join(format!("{tid}.json"));
    if !path.is_file() {
        return Err(format!("task file not found: {tid}"));
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let id = v
        .get("id")
        .and_then(|x| x.as_str())
        .unwrap_or(tid)
        .to_string();
    let task_type = v
        .get("type")
        .and_then(|x| x.as_str())
        .unwrap_or("unknown")
        .to_string();
    let status = v
        .get("status")
        .and_then(|x| x.as_str())
        .unwrap_or("unknown")
        .to_string();
    let progress = v
        .get("progress")
        .and_then(|x| x.as_f64())
        .unwrap_or(0.0);
    let result = v
        .get("result")
        .and_then(|x| x.as_str())
        .map(String::from);
    let error = v
        .get("error")
        .and_then(|x| x.as_str())
        .map(String::from);
    Ok(BackgroundTaskStatus {
        id,
        task_type,
        status,
        progress,
        result,
        error,
    })
}

/// 与 `~/.chimera/config.toml` `[llm.providers.*]` 对齐，供 HUD 展示（含环境变量解析后的 apiKey）。
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AvailableProvider {
    id: String,
    name: String,
    api_key: String,
    base_url: String,
    model: String,
    temperature: f64,
    timeout_seconds: u64,
}

#[tauri::command]
fn get_available_providers(state: tauri::State<'_, AstrocyteState>) -> Vec<AvailableProvider> {
    let chimera = state.chimera.read().expect("chimera lock poisoned");
    let merged = chimera.llm.merged_providers();
    let mut out: Vec<AvailableProvider> = Vec::new();
    for id in ["openai", "deepseek", "anthropic"] {
        if let Some(slot) = merged.get(id) {
            out.push(AvailableProvider {
                id: id.to_string(),
                name: if slot.name.trim().is_empty() {
                    id.to_string()
                } else {
                    slot.name.clone()
                },
                api_key: settings::resolved_provider_api_key(id, slot),
                base_url: slot.base_url.clone(),
                model: slot.model.clone(),
                temperature: slot.temperature,
                timeout_seconds: slot.timeout_seconds,
            });
        }
    }
    for (id, slot) in &merged {
        if matches!(id.as_str(), "openai" | "deepseek" | "anthropic") {
            continue;
        }
        out.push(AvailableProvider {
            id: id.clone(),
            name: if slot.name.trim().is_empty() {
                id.clone()
            } else {
                slot.name.clone()
            },
            api_key: settings::resolved_provider_api_key(id, slot),
            base_url: slot.base_url.clone(),
            model: slot.model.clone(),
            temperature: slot.temperature,
            timeout_seconds: slot.timeout_seconds,
        });
    }
    out
}

/// 自磁盘重读 `~/.chimera/config.toml` 并刷新内存中的 Chimera 快照；必要时校正 `active_provider_id` 并写回 JSON。
#[tauri::command]
async fn reload_chimera_config(state: tauri::State<'_, AstrocyteState>) -> Result<(), String> {
    let fresh = config::load_config()?;
    {
        let mut w = state
            .chimera
            .write()
            .map_err(|_| "chimera config lock poisoned".to_string())?;
        *w = fresh;
    }
    let chimera = state.chimera.read().expect("chimera lock poisoned").clone();
    let mut ui = state.config.read().await.clone();
    let before = ui.clone();
    normalize_astrocyte_with_chimera(&mut ui, &chimera);
    if ui != before {
        save_astrocyte_config(&ui)?;
        *state.config.write().await = ui;
    }
    Ok(())
}

#[tauri::command]
async fn set_active_provider(
    id: String,
    state: tauri::State<'_, AstrocyteState>,
) -> Result<(), String> {
    let provider_id = id.trim();
    if provider_id.is_empty() {
        return Err("provider id is empty".to_string());
    }

    let chimera_snap = state.chimera.read().expect("chimera lock poisoned").clone();
    if !chimera_snap
        .llm
        .merged_providers()
        .contains_key(provider_id)
    {
        return Err(format!(
            "provider '{}' not found under [llm.providers] in ~/.chimera/config.toml",
            provider_id
        ));
    }

    let mut config = state.config.read().await.clone();
    config.active_provider_id = Some(provider_id.to_string());

    save_astrocyte_config(&config)?;
    let mut guard = state.config.write().await;
    *guard = config;
    Ok(())
}

#[tauri::command]
async fn set_is_oligo_mode(
    enabled: bool,
    state: tauri::State<'_, AstrocyteState>,
) -> Result<(), String> {
    let mut config = state.config.read().await.clone();
    config.is_oligo_mode = enabled;
    save_astrocyte_config(&config)?;
    let mut guard = state.config.write().await;
    *guard = config;
    Ok(())
}

#[tauri::command]
async fn set_active_skill_id(
    skill_id: Option<String>,
    state: tauri::State<'_, AstrocyteState>,
) -> Result<(), String> {
    let mut config = state.config.read().await.clone();
    config.active_skill_id = skill_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    save_astrocyte_config(&config)?;
    let mut guard = state.config.write().await;
    *guard = config;
    Ok(())
}

#[tauri::command]
async fn get_available_skills() -> Result<Vec<SkillWithStats>, String> {
    Ok(skills::load_all_skills_with_stats())
}

/// 用户在前端对 BB 回复标记满意/不满意时写入 `~/.chimera/skill_stats.json`（与 Python 服务同结构）。
#[tauri::command]
fn submit_skill_feedback(skill_id: String, success: bool, tokens: i32) -> Result<(), String> {
    skill_stats::record_skill_feedback(&skill_id, success, tokens)
}

fn json_number_as_f64(v: &serde_json::Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_u64().map(|n| n as f64))
        .or_else(|| v.as_i64().map(|n| n as f64))
}

/// 与 `crucible_core` `MetricsService.get_summary` 对齐：从落盘 `metrics.json` 生成面板用摘要。
fn summarize_chimera_metrics(raw: &serde_json::Value) -> serde_json::Value {
    let total = raw
        .get("total_requests")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let success = raw
        .get("successful_requests")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let success_rate = if total > 0 {
        success as f64 / total as f64
    } else {
        0.0
    };
    let avg_latency_ms = raw
        .get("latencies")
        .and_then(|v| v.as_array())
        .map(|arr| {
            let nums: Vec<f64> = arr.iter().filter_map(json_number_as_f64).collect();
            if nums.is_empty() {
                0.0
            } else {
                nums.iter().sum::<f64>() / nums.len() as f64
            }
        })
        .unwrap_or(0.0);
    let total_tokens = raw
        .get("total_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let mut top: Vec<(String, u64, u64)> = Vec::new();
    if let Some(skills) = raw.get("skills").and_then(|v| v.as_object()) {
        for (id, stats) in skills {
            let count = stats
                .get("count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let tokens = stats
                .get("tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            top.push((id.clone(), count, tokens));
        }
    }
    top.sort_by(|a, b| b.1.cmp(&a.1));
    let top_skills: Vec<serde_json::Value> = top
        .into_iter()
        .take(5)
        .map(|(id, count, tokens)| {
            serde_json::json!({
                "id": id,
                "count": count,
                "tokens": tokens,
            })
        })
        .collect();

    let tool_stats = tool_stats_from_raw(raw);
    let wash_stats = wash_stats_from_raw(raw);

    serde_json::json!({
        "total_requests": total,
        "success_rate": success_rate,
        "avg_latency_ms": avg_latency_ms,
        "total_tokens": total_tokens,
        "top_skills": top_skills,
        "tool_stats": tool_stats,
        "wash_stats": wash_stats,
    })
}

/// 与 `MetricsService.get_wash_stats` 对齐。
fn wash_stats_from_raw(raw: &serde_json::Value) -> serde_json::Value {
    let wash = raw.get("wash");
    let Some(wash_obj) = wash.and_then(|v| v.as_object()) else {
        return serde_json::json!({
            "total_washes": 0u64,
            "avg_compression_rate": 0.0,
            "by_tool": serde_json::json!({}),
        });
    };
    let total_original = wash_obj
        .get("total_original_chars")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let total_washed = wash_obj
        .get("total_washed_chars")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let total_washes = wash_obj
        .get("total_washes")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let avg_compression_rate = if total_original > 0 {
        1.0 - (total_washed as f64 / total_original as f64)
    } else {
        0.0
    };
    let by_tool = wash_obj
        .get("by_tool")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    serde_json::json!({
        "total_washes": total_washes,
        "avg_compression_rate": avg_compression_rate,
        "by_tool": by_tool,
    })
}

/// 与 `MetricsService.get_tool_stats` 对齐。
fn tool_stats_from_raw(raw: &serde_json::Value) -> Vec<serde_json::Value> {
    let Some(tools) = raw.get("tools").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    let mut rows: Vec<(u64, serde_json::Value)> = Vec::new();
    for (tool_name, stats) in tools {
        let count = stats.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
        let succ = stats.get("success").and_then(|v| v.as_u64()).unwrap_or(0);
        let success_rate = if count > 0 {
            succ as f64 / count as f64
        } else {
            0.0
        };
        let avg_latency_ms = stats
            .get("latencies")
            .and_then(|v| v.as_array())
            .map(|arr| {
                let nums: Vec<f64> = arr.iter().filter_map(json_number_as_f64).collect();
                if nums.is_empty() {
                    0.0
                } else {
                    nums.iter().sum::<f64>() / nums.len() as f64
                }
            })
            .unwrap_or(0.0);
        rows.push((
            count,
            serde_json::json!({
                "name": tool_name,
                "count": count,
                "success_rate": success_rate,
                "avg_latency_ms": avg_latency_ms,
            }),
        ));
    }
    rows.sort_by(|a, b| b.0.cmp(&a.0));
    rows.into_iter().map(|(_, v)| v).collect()
}

#[tauri::command]
async fn get_system_metrics() -> Result<serde_json::Value, String> {
    let metrics_path = crate::platform::get_chimera_root()?.join("metrics.json");
    if !metrics_path.exists() {
        return Ok(summarize_chimera_metrics(&serde_json::json!({})));
    }
    let content =
        std::fs::read_to_string(&metrics_path).map_err(|e| format!("Failed to read metrics: {}", e))?;
    let raw: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse metrics: {}", e))?;
    Ok(summarize_chimera_metrics(&raw))
}

#[tauri::command]
fn get_personas() -> Result<PersonaSnapshot, String> {
    persona::get_personas_snapshot()
}

#[tauri::command]
async fn save_persona(
    persona: PersonaConfig,
    state: tauri::State<'_, AstrocyteState>,
) -> Result<PersonaSnapshot, String> {
    let snapshot = persona::save_persona_config(persona)?;
    if let Some(active) = snapshot
        .personas
        .iter()
        .find(|item| item.id == snapshot.active_persona_id)
        .cloned()
    {
        let mut guard = state.active_persona.write().await;
        *guard = active;
    }
    Ok(snapshot)
}

#[tauri::command]
async fn delete_persona(
    id: String,
    state: tauri::State<'_, AstrocyteState>,
) -> Result<PersonaSnapshot, String> {
    let snapshot = persona::delete_persona_config(&id)?;
    if let Some(active) = snapshot
        .personas
        .iter()
        .find(|item| item.id == snapshot.active_persona_id)
        .cloned()
    {
        let mut guard = state.active_persona.write().await;
        *guard = active;
    }
    Ok(snapshot)
}

#[tauri::command]
async fn set_active_persona(
    id: String,
    state: tauri::State<'_, AstrocyteState>,
) -> Result<(), String> {
    let active = persona::set_active_persona_id(&id)?;
    let mut guard = state.active_persona.write().await;
    *guard = active;
    Ok(())
}

fn build_models_ping_url(base_url: &str) -> Option<String> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.ends_with("/models") {
        return Some(trimmed.to_string());
    }
    if let Some(prefix) = trimmed.strip_suffix("/chat/completions") {
        return Some(format!("{}/models", prefix.trim_end_matches('/')));
    }
    if let Some(prefix) = trimmed.strip_suffix("/v1/chat/completions") {
        return Some(format!("{}/v1/models", prefix.trim_end_matches('/')));
    }
    if trimmed.ends_with("/v1") {
        return Some(format!("{}/models", trimmed));
    }
    if let Ok(url) = reqwest::Url::parse(trimmed) {
        if url.path().is_empty() || url.path() == "/" {
            return Some(format!("{}/v1/models", trimmed));
        }
    }
    Some(format!("{}/models", trimmed))
}

#[tauri::command]
async fn ping_provider(base_url: String, api_key: String) -> bool {
    let endpoint = match build_models_ping_url(&base_url) {
        Some(url) => url,
        None => {
            warn!("[ping_provider] invalid base_url (empty or malformed): {:?}", base_url);
            return false;
        }
    };
    let key = api_key.trim();
    if key.is_empty() {
        warn!("[ping_provider] api_key is empty");
        return false;
    }

    debug!("[ping_provider] probing endpoint: {}", endpoint);

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            warn!("[ping_provider] failed to build reqwest client: {}", e);
            return false;
        }
    };

    match client
        .get(&endpoint)
        .header(AUTHORIZATION, format!("Bearer {}", key))
        .send()
        .await
    {
        Ok(response) => {
            let ok = response.status().is_success();
            if ok {
                debug!("[ping_provider] {} -> LINK_ESTABLISHED", endpoint);
            } else {
                warn!("[ping_provider] {} -> HTTP {} LINK_DEAD", endpoint, response.status());
            }
            ok
        }
        Err(e) => {
            warn!("[ping_provider] {} -> request failed: {}", endpoint, e);
            false
        }
    }
}

#[tauri::command]
async fn sync_session_history(
    session_id: String,
    entries: Vec<SyncEntry>,
    state: tauri::State<'_, AstrocyteState>,
) -> Result<(), String> {
    let normalized_session_id = normalize_session_id(Some(session_id));
    state.create_session(normalized_session_id.clone()).await;

    let chat_entries: Vec<ChatEntry> = entries
        .into_iter()
        .map(|e| ChatEntry {
            id: e.id,
            timestamp: e.timestamp.unwrap_or_else(now_timestamp),
            role: e.role,
            content: e.content,
            session_id: normalized_session_id.clone(),
            persona: e.persona,
            artifacts: None,
        })
        .collect();

    let messages: Vec<Message> = chat_entries
        .iter()
        .map(|e| Message {
            role: e.role.clone(),
            content: e.content.clone(),
        })
        .collect();

    tauri::async_runtime::spawn_blocking({
        let sid = normalized_session_id.clone();
        let entries = chat_entries.clone();
        move || save_session_entries(&sid, &entries)
    })
    .await
    .map_err(|e| format!("sync task join failed: {}", e))??;

    state
        .set_history_for_session(&normalized_session_id, messages)
        .await?;
    Ok(())
}

fn now_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn normalize_session_id(raw: Option<String>) -> String {
    raw.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "default_session".to_string())
}

fn build_entry(
    id: String,
    role: &str,
    content: String,
    persona: Option<&str>,
    session_id: &str,
    artifacts: Option<Vec<Artifact>>,
) -> ChatEntry {
    ChatEntry {
        id,
        timestamp: now_timestamp(),
        role: role.to_string(),
        content,
        session_id: session_id.to_string(),
        persona: persona.map(|value| value.to_string()),
        artifacts,
    }
}

async fn clear_abort_slot(app: &tauri::AppHandle) {
    let st = app.state::<AstrocyteState>();
    *st.abort_token.write().await = None;
}

/// JSONL / timeline persistence is strictly **user + bb (assistant)** turns only.
/// `bb-sys-event` tool traces exist only in the webview; they are never appended to
/// `AstrocyteState` sessions and must never appear in `ChatEntry` batches below.
fn persist_chat_entries_non_blocking(session_id: String, entries: Vec<ChatEntry>) {
    tauri::async_runtime::spawn(async move {
        let write_result = tauri::async_runtime::spawn_blocking(move || {
            append_session_entries(&session_id, &entries)
        })
        .await;
        match write_result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => eprintln!("[Astrocyte] memory append failed: {}", e),
            Err(e) => eprintln!("[Astrocyte] memory append task join failed: {}", e),
        }
    });
}

/// 与 Oligo `ChimeraAgent` 的 Final System 一致：L1 基座 + skill，L2 Persona，L3 Author's Note（直连模式单条 system）。
fn compose_prompt_injection_system(
    system_core: &str,
    skill_override: Option<&str>,
    persona: Option<&str>,
    authors_note: Option<&str>,
) -> String {
    let core = system_core.trim_end();
    let sk = skill_override.map(str::trim).filter(|s| !s.is_empty());
    let base = match &sk {
        Some(s) if !core.is_empty() => format!("{core}\n\n{s}"),
        Some(s) => s.to_string(),
        None => core.to_string(),
    };
    let mut layers: Vec<String> = Vec::new();
    if !base.is_empty() {
        layers.push(base);
    }
    let p = persona.map(str::trim).filter(|s| !s.is_empty());
    let core_norm = system_core.trim();
    if let Some(text) = p {
        if text != core_norm {
            layers.push(format!("[PERSONA OVERRIDE]\n{text}"));
        }
    }
    if let Some(note) = authors_note.map(str::trim).filter(|s| !s.is_empty()) {
        layers.push(format!("[AUTHOR'S NOTE]\n{note}"));
    }
    layers.join("\n\n")
}

/// 双模式分流：is_oligo_mode → Oligo Agent；否则直连 API。
#[tauri::command]
async fn evaluate_payload(
    payload: String,
    session_id: Option<String>,
    skill_id: Option<String>,
    persona: Option<String>,
    user_message_id: Option<String>,
    assistant_message_id: Option<String>,
    state: tauri::State<'_, AstrocyteState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let user_input = payload.trim().to_string();
    if user_input.is_empty() {
        return Err("payload is empty".to_string());
    }

    let active_persona = state.active_persona.read().await.clone();
    let active_persona_id = active_persona.id.clone();
    let config = state.config.read().await.clone();
    let chimera_snapshot = state.chimera.read().expect("chimera lock poisoned").clone();
    let is_oligo_mode = config.is_oligo_mode;
    if is_oligo_mode && resolve_active_provider_runtime(&config, &chimera_snapshot).is_none() {
        return Err(
            "No active provider or missing API key (configure ~/.chimera/config.toml [llm.providers] or env)"
                .into(),
        );
    }
    let effective_skill_id = skill_id
        .as_deref()
        .and_then(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        })
        .or_else(|| config.active_skill_id.clone());

    let session_id = normalize_session_id(session_id);
    state.create_session(session_id.clone()).await;

    let history = state.get_history(&session_id).await.unwrap_or_default();
    state
        .append_message_to_session(&session_id, "user", user_input.clone())
        .await?;

    let system_core = active_persona.system_prompt.clone();

    let session_persona = persona
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    // L3：仅随 Persona 持久化；不再接受 HUD 侧独立会话输入，避免与 active_persona.authors_note 竞态。
    let effective_authors_note = active_persona
        .authors_note
        .as_ref()
        .and_then(|n| {
            let t = n.trim();
            if t.is_empty() {
                None
            } else {
                Some(n.clone())
            }
        });

    let (skill_override, allowed_tools) =
        match effective_skill_id.as_deref().and_then(skills::load_skill) {
            Some(skill) => {
                let allowed_tools = skill.allowed_tools;
                let override_txt = {
                    let t = skill.system_override.trim();
                    if t.is_empty() {
                        None
                    } else {
                        Some(skill.system_override)
                    }
                };
                (override_txt, allowed_tools)
            }
            None => (None, None),
        };

    if is_oligo_mode {
        info!(
            "[Astrocyte] Oligo skill payload: effective_skill_id={:?} skill_override={} allowed_tools={:?}",
            effective_skill_id.as_deref(),
            skill_override.is_some(),
            allowed_tools
        );
    }

    let oligo_messages = build_oligo_transcript_messages(&history, &user_input);

    let direct_messages = build_direct_mode_messages(
        &system_core,
        skill_override.as_deref(),
        session_persona.as_deref(),
        effective_authors_note.as_deref(),
        &history,
        &user_input,
    );
    let provider = resolve_active_provider_runtime(&config, &chimera_snapshot);
    let oligo_api_key = provider
        .as_ref()
        .map(|p| p.api_key.clone())
        .unwrap_or_default();
    let provider_llm_base_url = provider
        .as_ref()
        .map(|p| p.base_url.clone())
        .unwrap_or_default();
    let oligo_model_name = provider
        .as_ref()
        .map(|p| p.model_name.clone())
        .unwrap_or_default();
    let oligo_temperature = provider.as_ref().and_then(|p| p.temperature);

    let app_handle = app.clone();
    let persona_id = active_persona.id.clone();
    let system_core_for_oligo = system_core;
    let chimera = chimera_snapshot;
    let oligo_gateway_base_url = settings::effective_oligo_base_url(&config, &chimera);

    let cancel_token = CancellationToken::new();
    {
        let mut guard = state.abort_token.write().await;
        *guard = Some(cancel_token.clone());
    }

    tauri::async_runtime::spawn(async move {
        let model_reply: Result<Option<(String, Option<Vec<Artifact>>)>, String> = if is_oligo_mode {
            llm_client::stream_oligo_agent(
                oligo_gateway_base_url.as_str(),
                oligo_api_key,
                provider_llm_base_url,
                oligo_model_name,
                Some(persona_id),
                system_core_for_oligo,
                skill_override,
                effective_skill_id.clone(),
                allowed_tools,
                session_persona,
                effective_authors_note,
                oligo_temperature,
                oligo_messages,
                &app_handle,
                cancel_token.clone(),
            )
            .await
        } else {
            let provider = match provider {
                Some(p) => p,
                None => {
                    clear_abort_slot(&app_handle).await;
                    let e = "no active provider configured".to_string();
                    let _ = app_handle.emit("bb-stream-chunk", e.clone());
                    let _ = app_handle.emit(
                        "bb-stream-done",
                        serde_json::json!({ "error": true, "message": e }),
                    );
                    return;
                }
            };
            llm_client::stream_direct_api(
                direct_messages,
                provider,
                &app_handle,
                cancel_token.clone(),
            )
            .await
            .map(|s| Some((s, None)))
        };

        let (model_reply, model_artifacts) = match model_reply {
            Ok(None) => {
                clear_abort_slot(&app_handle).await;
                let user_id = user_message_id.unwrap_or_else(|| Uuid::new_v4().to_string());
                let user_entries = vec![build_entry(
                    user_id,
                    "user",
                    user_input.clone(),
                    None,
                    &session_id,
                    None,
                )];
                persist_chat_entries_non_blocking(session_id.clone(), user_entries);
                return;
            }
            Ok(Some((r, arts))) => (r, arts),
            Err(e) if e == llm_client::GENERATION_ABORTED => {
                clear_abort_slot(&app_handle).await;
                let user_id = user_message_id.unwrap_or_else(|| Uuid::new_v4().to_string());
                let user_entries = vec![build_entry(
                    user_id,
                    "user",
                    user_input.clone(),
                    None,
                    &session_id,
                    None,
                )];
                persist_chat_entries_non_blocking(session_id.clone(), user_entries);
                let _ = app_handle.emit(
                    "bb-stream-done",
                    serde_json::json!({ "aborted": true }),
                );
                return;
            }
            Err(e) => {
                warn!("[Astrocyte] stream request failed: {}", e);
                clear_abort_slot(&app_handle).await;
                let _ = app_handle.emit("bb-stream-chunk", e.clone());
                let _ = app_handle.emit(
                    "bb-stream-done",
                    serde_json::json!({ "error": true, "message": e }),
                );
                return;
            }
        };

        let app_state = app_handle.state::<AstrocyteState>();
        if let Err(e) = app_state
            .append_message_to_session(&session_id, "assistant", model_reply.clone())
            .await
        {
            warn!("[Astrocyte] append assistant message failed: {}", e);
        }

        let user_id = user_message_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let assistant_id = assistant_message_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let entries = vec![
            build_entry(user_id, "user", user_input, None, &session_id, None),
            build_entry(
                assistant_id,
                "bb",
                model_reply,
                Some(&active_persona_id),
                &session_id,
                model_artifacts,
            ),
        ];
        persist_chat_entries_non_blocking(session_id.clone(), entries);

        clear_abort_slot(&app_handle).await;
        let _ = app_handle.emit("bb-stream-done", "DONE");
    });

    Ok(())
}

fn normalize_role(role: &str) -> String {
    match role {
        "bb" => "assistant".to_string(),
        r => r.to_string(),
    }
}

/// Oligo：`messages` 仅含 user/assistant 历史与本轮 user，禁止在此注入 system。
fn build_oligo_transcript_messages(history: &[Message], user_input: &str) -> Vec<Message> {
    let mut msgs = Vec::with_capacity(history.len() + 1);
    msgs.extend(history.iter().map(|m| Message {
        role: normalize_role(&m.role),
        content: m.content.clone(),
    }));
    msgs.push(Message {
        role: "user".to_string(),
        content: user_input.to_string(),
    });
    msgs
}

fn build_direct_mode_messages(
    system_core: &str,
    skill_override: Option<&str>,
    persona: Option<&str>,
    authors_note: Option<&str>,
    history: &[Message],
    user_input: &str,
) -> Vec<Message> {
    let mut msgs = Vec::with_capacity(history.len() + 2);
    msgs.push(Message {
        role: "system".to_string(),
        content: compose_prompt_injection_system(system_core, skill_override, persona, authors_note),
    });
    msgs.extend(
        history
            .iter()
            .map(|m| Message {
                role: normalize_role(&m.role),
                content: m.content.clone(),
            }),
    );
    msgs.push(Message {
        role: "user".to_string(),
        content: user_input.to_string(),
    });
    msgs
}

#[tauri::command]
async fn delete_session_history(
    session_id: String,
    state: tauri::State<'_, AstrocyteState>,
) -> Result<(), String> {
    let sid = normalize_session_id(Some(session_id));

    tauri::async_runtime::spawn_blocking({
        let s = sid.clone();
        move || delete_session_file(&s)
    })
    .await
    .map_err(|e| format!("delete session file task join failed: {}", e))??;

    state.remove_session(&sid).await;
    Ok(())
}

#[tauri::command]
async fn get_session_history() -> Result<Vec<SessionSummary>, String> {
    let result = tauri::async_runtime::spawn_blocking(get_timeline_summaries)
        .await
        .map_err(|e| format!("failed to join session summary task: {}", e))?;
    result
}

#[tauri::command]
async fn delete_chat_message(
    session_id: String,
    msg_id: String,
    state: tauri::State<'_, AstrocyteState>,
) -> Result<(), String> {
    let sid = normalize_session_id(Some(session_id));
    let mid = msg_id.trim();
    if mid.is_empty() {
        return Err("msg_id is empty".to_string());
    }

    let session_file = memory::get_session_file_path(&sid)
        .map_err(|e| format!("session path failed: {}", e))?;

    // 仅当 session 文件存在时才执行删除，避免未持久化会话产生空文件
    if !session_file.exists() {
        return Ok(());
    }

    tauri::async_runtime::spawn_blocking({
        let s = sid.clone();
        let m = mid.to_string();
        move || delete_entry(&s, &m)
    })
    .await
    .map_err(|e| format!("delete task join failed: {}", e))??;

    {
        let entries = tauri::async_runtime::spawn_blocking({
            let s = sid.clone();
            move || get_entries_for_session(&s)
        })
        .await
        .map_err(|e| format!("load entries task join failed: {}", e))??;

        let messages: Vec<Message> = entries
            .iter()
            .map(|e| Message {
                role: e.role.clone(),
                content: e.content.clone(),
            })
            .collect();

        state.create_session(sid.clone()).await;
        state.set_history_for_session(&sid, messages).await?;
    }

    Ok(())
}

/// Validates that `raw` resolves to a path canonically inside `vault_root`.
/// Rejects traversal components (`..`) before touching the filesystem and
/// symlink escapes via canonical-path prefix check after resolving symlinks.
fn vault_contains_path(
    vault_root: &std::path::Path,
    raw: &str,
) -> Result<std::path::PathBuf, String> {
    use std::path::{Component, Path};

    let candidate = Path::new(raw);
    if candidate.components().any(|c| c == Component::ParentDir) {
        return Err(format!("[vault_contains_path] traversal rejected: {}", raw));
    }

    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        vault_root.join(candidate)
    };

    let canon_root = std::fs::canonicalize(vault_root)
        .map_err(|e| format!("[vault_contains_path] vault_root canonicalize failed: {}", e))?;
    let canon_candidate = std::fs::canonicalize(&joined)
        .map_err(|e| format!("[vault_contains_path] candidate canonicalize failed: {}", e))?;

    if !canon_candidate.starts_with(&canon_root) {
        return Err(format!("[vault_contains_path] path outside vault root: {}", raw));
    }

    Ok(canon_candidate)
}

/// Opens a vault note in Obsidian after validating the path is inside vault root.
#[tauri::command]
async fn open_vault_note(
    path: String,
    state: tauri::State<'_, AstrocyteState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    let vault_root_str = state
        .chimera
        .read()
        .expect("chimera lock poisoned")
        .system
        .vault_root
        .clone()
        .ok_or_else(|| {
            "[open_vault_note] vault_root not configured in ~/.chimera/config.toml".to_string()
        })?;

    let raw = path.trim();
    vault_contains_path(std::path::Path::new(&vault_root_str), raw)?;

    let mut encoded = String::with_capacity(raw.len() + 16);
    for b in raw.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' | b'/' | b':' => encoded.push(b as char),
            _ => encoded.push_str(&format!("%{:02X}", b)),
        }
    }
    let uri = format!("obsidian://open?path={}", encoded);

    info!("[open_vault_note] opening {}", raw);
    app.opener()
        .open_url(&uri, None::<&str>)
        .map_err(|e| format!("[open_vault_note] opener failed: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn load_session_archive(
    session_id: String,
    state: tauri::State<'_, AstrocyteState>,
) -> Result<Vec<ChatEntry>, String> {
    let normalized_session_id = normalize_session_id(Some(session_id));
    let session_id_for_load = normalized_session_id.clone();
    let entries = tauri::async_runtime::spawn_blocking(move || get_entries_for_session(&session_id_for_load))
        .await
        .map_err(|e| format!("failed to join session archive task: {}", e))??;

    let runtime_history = entries
        .iter()
        .filter_map(|entry| {
            let normalized_role = match entry.role.as_str() {
                "user" => "user",
                "assistant" | "bb" => "assistant",
                _ => return None,
            };
            Some(Message {
                role: normalized_role.to_string(),
                content: entry.content.clone(),
            })
        })
        .collect::<Vec<_>>();

    state.create_session(normalized_session_id.clone()).await;
    state
        .set_history_for_session(&normalized_session_id, runtime_history)
        .await?;

    Ok(entries)
}

#[tauri::command]
async fn save_scratchpad(content: String) -> Result<(), String> {
    let path = crate::platform::get_chimera_root()?.join("scratchpad.md");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn load_scratchpad() -> Result<String, String> {
    let path = crate::platform::get_chimera_root()?.join("scratchpad.md");
    if path.exists() {
        std::fs::read_to_string(&path).map_err(|e| e.to_string())
    } else {
        Ok(String::new())
    }
}

#[tauri::command]
async fn set_phantom_sidebar_visible(app: tauri::AppHandle, visible: bool) -> Result<(), String> {
    set_phantom_sidebar_visible_internal(&app, visible)
}

#[tauri::command]
async fn hide_phantom_sidebar(app: tauri::AppHandle) -> Result<(), String> {
    set_phantom_sidebar_visible_internal(&app, false)
}

#[tauri::command]
async fn set_timeline_visible(app: tauri::AppHandle, visible: bool) -> Result<(), String> {
    set_timeline_visible_internal(&app, visible)
}

#[tauri::command]
async fn hide_timeline(app: tauri::AppHandle) -> Result<(), String> {
    set_timeline_visible_internal(&app, false)
}

#[tauri::command]
fn load_session_into_main(session_id: String, app: tauri::AppHandle) -> Result<(), String> {
    let main_window = app
        .get_webview_window("main")
        .ok_or("main window not found")?;
    main_window
        .emit("load-session", session_id)
        .map_err(|e| format!("failed to emit load-session: {}", e))?;
    Ok(())
}

#[tauri::command]
fn new_signal_in_main(app: tauri::AppHandle) -> Result<(), String> {
    let main_window = app
        .get_webview_window("main")
        .ok_or("main window not found")?;
    main_window
        .emit("new-signal", ())
        .map_err(|e| format!("failed to emit new-signal: {}", e))?;
    Ok(())
}

#[tauri::command]
fn sublimate_scratchpad(content: String, app: tauri::AppHandle) -> Result<(), String> {
    let main_window = app
        .get_webview_window("main")
        .ok_or("main window not found")?;
    main_window
        .emit("sublimate-request", content)
        .map_err(|e| format!("failed to emit sublimate-request: {}", e))?;
    Ok(())
}

const SIDEBAR_WIDTH: u32 = 360;
const TIMELINE_WIDTH: u32 = 320;

fn sync_aux_windows_with_main_geometry(
    app: &tauri::AppHandle<tauri::Wry>,
    main_pos: tauri::PhysicalPosition<i32>,
    main_size: tauri::PhysicalSize<u32>,
) {
    let anchor_y = main_pos.y;
    let sidebar_x = main_pos.x.saturating_add(main_size.width as i32);
    let timeline_x = main_pos.x.saturating_sub(TIMELINE_WIDTH as i32);

    if let Some(sidebar_window) = app.get_webview_window("sidebar") {
        if sidebar_window.is_visible().unwrap_or(false) {
            let _ = sidebar_window.set_size(tauri::PhysicalSize::new(SIDEBAR_WIDTH, main_size.height));
            let _ = sidebar_window.set_position(tauri::PhysicalPosition::new(sidebar_x, anchor_y));
        }
    }

    if let Some(timeline_window) = app.get_webview_window("timeline") {
        if timeline_window.is_visible().unwrap_or(false) {
            let _ = timeline_window.set_size(tauri::PhysicalSize::new(TIMELINE_WIDTH, main_size.height));
            let _ = timeline_window.set_position(tauri::PhysicalPosition::new(timeline_x, anchor_y));
        }
    }
}

fn sync_aux_windows_with_main_window(main_window: &tauri::WebviewWindow<tauri::Wry>) {
    let Ok(main_pos) = main_window.outer_position() else {
        return;
    };
    let Ok(main_size) = main_window.outer_size() else {
        return;
    };
    sync_aux_windows_with_main_geometry(&main_window.app_handle(), main_pos, main_size);
}

fn set_phantom_sidebar_visible_internal(
    app: &tauri::AppHandle<tauri::Wry>,
    visible: bool,
) -> Result<(), String> {
    let main_window = app
        .get_webview_window("main")
        .ok_or("main window not found")?;
    let sidebar_window = app
        .get_webview_window("sidebar")
        .ok_or("sidebar window not found")?;

    let main_pos = main_window
        .outer_position()
        .map_err(|e| format!("failed to read main outer_position: {}", e))?;
    let main_size = main_window
        .outer_size()
        .map_err(|e| format!("failed to read main outer_size: {}", e))?;

    let anchor_x = main_pos.x.saturating_add(main_size.width as i32);
    let anchor_y = main_pos.y;

    sidebar_window
        .set_size(tauri::PhysicalSize::new(SIDEBAR_WIDTH, main_size.height))
        .map_err(|e| format!("failed to set sidebar size: {}", e))?;
    sidebar_window
        .set_position(tauri::PhysicalPosition::new(anchor_x, anchor_y))
        .map_err(|e| format!("failed to set sidebar position: {}", e))?;

    if visible {
        sidebar_window
            .show()
            .map_err(|e| format!("failed to show sidebar: {}", e))?;
        let _ = sidebar_window.set_focus();
    } else {
        sidebar_window
            .hide()
            .map_err(|e| format!("failed to hide sidebar: {}", e))?;
    }
    let _ = main_window.set_focus();
    Ok(())
}

fn set_timeline_visible_internal(
    app: &tauri::AppHandle<tauri::Wry>,
    visible: bool,
) -> Result<(), String> {
    let main_window = app
        .get_webview_window("main")
        .ok_or("main window not found")?;
    let timeline_window = app
        .get_webview_window("timeline")
        .ok_or("timeline window not found")?;

    let main_pos = main_window
        .outer_position()
        .map_err(|e| format!("failed to read main outer_position: {}", e))?;
    let main_size = main_window
        .outer_size()
        .map_err(|e| format!("failed to read main outer_size: {}", e))?;

    let pane_width = TIMELINE_WIDTH;
    let anchor_x = main_pos.x.saturating_sub(pane_width as i32);
    let anchor_y = main_pos.y;

    timeline_window
        .set_size(tauri::PhysicalSize::new(pane_width, main_size.height))
        .map_err(|e| format!("failed to set timeline size: {}", e))?;
    timeline_window
        .set_position(tauri::PhysicalPosition::new(anchor_x, anchor_y))
        .map_err(|e| format!("failed to set timeline position: {}", e))?;

    if visible {
        timeline_window
            .show()
            .map_err(|e| format!("failed to show timeline: {}", e))?;
        let _ = timeline_window.set_focus();
    } else {
        timeline_window
            .hide()
            .map_err(|e| format!("failed to hide timeline: {}", e))?;
    }
    let _ = main_window.set_focus();
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let chimera = match config::load_config() {
        Ok(c) => {
            info!(
                "[Astrocyte] Loaded Chimera TOML ({}) — Oligo {}",
                platform::get_config_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|e| format!("<path error: {}>", e)),
                c.oligo_agent_invoke_url()
            );
            c
        }
        Err(e) => {
            warn!(
                "[Astrocyte] Chimera TOML unavailable ({}); using defaults ({})",
                e,
                ChimeraConfig::default().oligo_agent_invoke_url()
            );
            ChimeraConfig::default()
        }
    };

    tauri::Builder::default()
        .manage(AstrocyteState::new(chimera))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            if matches!(event, WindowEvent::Moved(_) | WindowEvent::Resized(_)) {
                if let Some(main_window) = window.app_handle().get_webview_window("main") {
                    sync_aux_windows_with_main_window(&main_window);
                }
            }
        })
        .setup(|app| {
            if let Err(e) = platform::migrate_legacy_app_data() {
                eprintln!("[Astrocyte] chimera path migration failed (non-fatal): {}", e);
            }
            if let Err(e) = memory::ensure_migration() {
                eprintln!("[Astrocyte] legacy memory migration failed (non-fatal): {}", e);
            }

            let app_handle = app.handle().clone();
            let chimera_snapshot = {
                let app_state = app.state::<AstrocyteState>();
                let chimera_guard = app_state
                    .chimera
                    .read()
                    .expect("chimera config lock poisoned");
                chimera_guard.clone()
            };
            let ui_snapshot = load_astrocyte_config().unwrap_or_default();
            let oligo_base_url =
                settings::effective_oligo_base_url(&ui_snapshot, &chimera_snapshot);
            tauri::async_runtime::spawn(async move {
                task_stream::run_task_stream_loop(app_handle, oligo_base_url).await;
            });

            // 注册全局快捷键 CommandOrControl+Space，失败时仅打印警告，不阻塞启动
            if let Err(e) = app.global_shortcut().on_shortcut(
                "CommandOrControl+Space",
                |app: &tauri::AppHandle<tauri::Wry>, _shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    if let Some(window) = app.get_webview_window("main") {
                        match window.is_visible() {
                            Ok(true) => {
                                if let Err(e) = window.hide() {
                                    eprintln!("[Astrocyte] window.hide() failed: {}", e);
                                }
                            }
                            Ok(false) => {
                                if let Err(e) = window.show() {
                                    eprintln!("[Astrocyte] window.show() failed: {}", e);
                                } else if let Err(e) = window.set_focus() {
                                    eprintln!("[Astrocyte] window.set_focus() failed: {}", e);
                                }
                            }
                            Err(e) => eprintln!("[Astrocyte] window.is_visible() failed: {}", e),
                        }
                    }
                },
            ) {
                eprintln!(
                    "[Astrocyte] Failed to register global shortcut (Cmd/Ctrl+Space): {}. \
                     Hotkey may be in use by another app. Running without hotkey.",
                    e
                );
            }

            // 注册零摩擦剪贴板劫持热键 Ctrl+Alt+Shift+C
            if let Err(e) = app.global_shortcut().on_shortcut(
                "Ctrl+Alt+Shift+C",
                |app: &tauri::AppHandle<tauri::Wry>, _shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }

                    let Some(window) = app.get_webview_window("main") else {
                        eprintln!("[Astrocyte] main window not found for clipboard hijack");
                        return;
                    };

                    if let Err(e) = window.show() {
                        eprintln!("[Astrocyte] window.show() failed: {}", e);
                    }
                    if let Err(e) = window.set_focus() {
                        eprintln!("[Astrocyte] window.set_focus() failed: {}", e);
                    }

                    let payload = match Clipboard::new() {
                        Ok(mut clipboard) => match clipboard.get_text() {
                            Ok(text) => {
                                let trimmed = text.trim().to_string();
                                if trimmed.is_empty() {
                                    eprintln!("[Astrocyte] clipboard text is empty, skip emit");
                                    return;
                                }
                                trimmed
                            }
                            Err(e) => {
                                eprintln!("[Astrocyte] clipboard.get_text() failed: {}", e);
                                return;
                            }
                        },
                        Err(e) => {
                            eprintln!("[Astrocyte] Clipboard::new() failed: {}", e);
                            return;
                        }
                    };

                    if let Err(e) = window.emit("clipboard-hijack", payload) {
                        eprintln!("[Astrocyte] window.emit(clipboard-hijack) failed: {}", e);
                    }
                },
            ) {
                eprintln!(
                    "[Astrocyte] Failed to register global shortcut (Ctrl+Alt+Shift+C): {}. \
                     Clipboard hijack hotkey disabled.",
                    e
                );
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            process_signal,
            abort_generation,
            evaluate_payload,
            get_background_task_status,
            set_is_oligo_mode,
            set_active_skill_id,
            get_available_skills,
            submit_skill_feedback,
            submit_segment_feedback,
            get_config,
            get_system_metrics,
            get_available_providers,
            reload_chimera_config,
            set_active_provider,
            get_personas,
            save_persona,
            delete_persona,
            set_active_persona,
            ping_provider,
            sync_session_history,
            get_session_history,
            load_session_archive,
            delete_chat_message,
            delete_session_history,
            save_scratchpad,
            load_scratchpad,
            scratchpad_commands::get_notes,
            scratchpad_commands::add_note,
            scratchpad_commands::update_note,
            scratchpad_commands::delete_note,
            set_phantom_sidebar_visible,
            hide_phantom_sidebar,
            set_timeline_visible,
            hide_timeline,
            load_session_into_main,
            new_signal_in_main,
            sublimate_scratchpad,
            open_vault_note
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod path_containment_tests {
    use super::vault_contains_path;
    use std::fs;

    fn make_temp_vault() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("note.md"), "# test").expect("write note");
        dir
    }

    #[test]
    fn inside_root_accepted() {
        let vault = make_temp_vault();
        let result = vault_contains_path(vault.path(), "note.md");
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
    }

    #[test]
    fn outside_root_rejected() {
        let vault = make_temp_vault();
        // Absolute path to a sibling directory that exists (system temp root)
        let outside = std::env::temp_dir();
        let result = vault_contains_path(vault.path(), outside.to_str().unwrap());
        assert!(result.is_err(), "expected Err for outside-root path");
    }

    #[test]
    fn traversal_rejected() {
        let vault = make_temp_vault();
        let result = vault_contains_path(vault.path(), "../escape.md");
        assert!(result.is_err(), "expected Err for traversal path");
    }

    #[test]
    fn nested_traversal_rejected() {
        let vault = make_temp_vault();
        let result = vault_contains_path(vault.path(), "subdir/../../escape.md");
        assert!(result.is_err(), "expected Err for nested traversal");
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn symlink_escape_rejected() {
        use std::os::unix::fs::symlink;
        let vault = make_temp_vault();
        let outside = tempfile::tempdir().expect("outside tempdir");
        fs::write(outside.path().join("secret.md"), "secret").expect("write secret");
        let link = vault.path().join("link.md");
        symlink(outside.path().join("secret.md"), &link).expect("symlink");
        let result = vault_contains_path(vault.path(), "link.md");
        assert!(result.is_err(), "expected Err for symlink escaping vault");
    }

    // Windows symlink creation requires Developer Mode or admin rights.
    // The containment helper's canonicalize() call enforces the same
    // invariant on Windows; this test documents the skip reason.
    #[test]
    #[cfg(target_os = "windows")]
    fn symlink_escape_skipped_on_windows_without_devmode() {
        // Symlink creation on Windows requires elevated privileges.
        // canonicalize() in vault_contains_path still resolves symlinks
        // when they exist, so the guard holds at runtime.
    }
}
