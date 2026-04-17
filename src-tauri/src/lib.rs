use arboard::Clipboard;
use chrono::{SecondsFormat, Utc};
use log::{debug, info, warn};
use memory::{
    append_session_entries, delete_entry, delete_session_file, get_entries_for_session,
    get_timeline_summaries, save_session_entries, ChatEntry, SessionSummary,
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
mod skills;
mod persona;
mod settings;
mod state;
use persona::{PersonaConfig, PersonaSnapshot};
use serde::Deserialize;
use settings::{save_astrocyte_config, AstrocyteConfig, ProviderConfig};
use skills::SkillDefinition;
use state::{AstrocyteState, Message};

/// Payload for syncing session history with persistence (includes ids for delete/edit).
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

#[tauri::command]
async fn save_provider(
    provider: ProviderConfig,
    state: tauri::State<'_, AstrocyteState>,
) -> Result<(), String> {
    let normalized = provider.normalized();
    normalized.validate()?;

    let mut config = state.config.read().await.clone();
    if let Some(existing) = config
        .providers
        .iter_mut()
        .find(|existing| existing.id == normalized.id)
    {
        *existing = normalized.clone();
    } else {
        config.providers.push(normalized.clone());
    }
    if config.active_provider_id.is_none() {
        config.active_provider_id = Some(normalized.id.clone());
    }

    save_astrocyte_config(&config)?;
    let mut guard = state.config.write().await;
    *guard = config;
    Ok(())
}

#[tauri::command]
async fn delete_provider(id: String, state: tauri::State<'_, AstrocyteState>) -> Result<(), String> {
    let provider_id = id.trim();
    if provider_id.is_empty() {
        return Err("provider id is empty".to_string());
    }

    let mut config = state.config.read().await.clone();
    let before = config.providers.len();
    config.providers.retain(|provider| provider.id != provider_id);
    if config.providers.len() == before {
        return Err(format!("provider '{}' not found", provider_id));
    }

    if config.active_provider_id.as_deref() == Some(provider_id) {
        config.active_provider_id = config.providers.first().map(|provider| provider.id.clone());
    }

    save_astrocyte_config(&config)?;
    let mut guard = state.config.write().await;
    *guard = config;
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

    let mut config = state.config.read().await.clone();
    if !config
        .providers
        .iter()
        .any(|provider| provider.id == provider_id)
    {
        return Err(format!("provider '{}' not found", provider_id));
    }
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
async fn get_available_skills() -> Result<Vec<SkillDefinition>, String> {
    Ok(skills::load_all_skills())
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
) -> ChatEntry {
    ChatEntry {
        id,
        timestamp: now_timestamp(),
        role: role.to_string(),
        content,
        session_id: session_id.to_string(),
        persona: persona.map(|value| value.to_string()),
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
            Ok(Err(e)) => eprintln!("[astrocyte] memory append failed: {}", e),
            Err(e) => eprintln!("[astrocyte] memory append task join failed: {}", e),
        }
    });
}

/// 双模式分流：is_oligo_mode → Oligo Agent；否则直连 API。
#[tauri::command]
async fn evaluate_payload(
    payload: String,
    session_id: Option<String>,
    skill_id: Option<String>,
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
    let is_oligo_mode = config.is_oligo_mode;
    if is_oligo_mode && config.active_provider().is_none() {
        return Err("No active provider selected".into());
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

    let mut system_core = active_persona.system_prompt.clone();
    if let Some(note) = &active_persona.authors_note {
        if !note.trim().is_empty() {
            system_core.push_str("\n\n[AUTHOR_NOTE]: ");
            system_core.push_str(note);
        }
    }

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
            "[astrocyte] Oligo skill payload: effective_skill_id={:?} skill_override={} allowed_tools={:?}",
            effective_skill_id.as_deref(),
            skill_override.is_some(),
            allowed_tools
        );
    }

    let oligo_messages = build_oligo_transcript_messages(&history, &user_input);

    let direct_messages =
        build_direct_mode_messages(&active_persona, &history, &user_input);
    let provider = config.active_provider().cloned();
    let oligo_api_key = provider
        .as_ref()
        .map(|p| p.api_key.clone())
        .unwrap_or_default();
    let oligo_base_url = provider
        .as_ref()
        .map(|p| p.base_url.clone())
        .unwrap_or_default();
    let oligo_model_name = provider
        .as_ref()
        .map(|p| p.model_name.clone())
        .unwrap_or_default();

    let app_handle = app.clone();
    let persona_id = active_persona.id.clone();
    let system_core_for_oligo = system_core.clone();

    let cancel_token = CancellationToken::new();
    {
        let mut guard = state.abort_token.write().await;
        *guard = Some(cancel_token.clone());
    }

    tauri::async_runtime::spawn(async move {
        let model_reply = if is_oligo_mode {
            llm_client::stream_oligo_agent(
                oligo_api_key,
                oligo_base_url,
                oligo_model_name,
                Some(persona_id),
                system_core_for_oligo,
                skill_override,
                allowed_tools,
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
        };

        let model_reply = match model_reply {
            Ok(r) => r,
            Err(e) if e == llm_client::GENERATION_ABORTED => {
                clear_abort_slot(&app_handle).await;
                let user_id = user_message_id.unwrap_or_else(|| Uuid::new_v4().to_string());
                let user_entries = vec![build_entry(
                    user_id,
                    "user",
                    user_input.clone(),
                    None,
                    &session_id,
                )];
                persist_chat_entries_non_blocking(session_id.clone(), user_entries);
                let _ = app_handle.emit(
                    "bb-stream-done",
                    serde_json::json!({ "aborted": true }),
                );
                return;
            }
            Err(e) => {
                warn!("[astrocyte] stream request failed: {}", e);
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
            warn!("[astrocyte] append assistant message failed: {}", e);
        }

        let user_id = user_message_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let assistant_id = assistant_message_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let entries = vec![
            build_entry(user_id, "user", user_input, None, &session_id),
            build_entry(
                assistant_id,
                "bb",
                model_reply,
                Some(&active_persona_id),
                &session_id,
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
    persona: &PersonaConfig,
    history: &[Message],
    user_input: &str,
) -> Vec<Message> {
    let mut msgs = Vec::with_capacity(history.len() + 3);
    msgs.push(Message {
        role: "system".to_string(),
        content: persona.system_prompt.clone(),
    });
    msgs.extend(
        history
            .iter()
            .map(|m| Message {
                role: normalize_role(&m.role),
                content: m.content.clone(),
            }),
    );
    if let Some(note) = &persona.authors_note {
        if !note.trim().is_empty() {
            msgs.push(Message {
                role: "system".to_string(),
                content: format!("[AUTHOR_NOTE]: {}", note),
            });
        }
    }
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
    let mut path = dirs::data_local_dir().ok_or("No local data dir found")?;
    path.push("chimera");
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    path.push("scratchpad.md");
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn load_scratchpad() -> Result<String, String> {
    let mut path = dirs::data_local_dir().ok_or("No local data dir found")?;
    path.push("chimera");
    path.push("scratchpad.md");
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
    tauri::Builder::default()
        .manage(AstrocyteState::new())
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
            if let Err(e) = memory::ensure_migration() {
                eprintln!("[astrocyte] legacy memory migration failed (non-fatal): {}", e);
            }
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
                                    eprintln!("[astrocyte] window.hide() failed: {}", e);
                                }
                            }
                            Ok(false) => {
                                if let Err(e) = window.show() {
                                    eprintln!("[astrocyte] window.show() failed: {}", e);
                                } else if let Err(e) = window.set_focus() {
                                    eprintln!("[astrocyte] window.set_focus() failed: {}", e);
                                }
                            }
                            Err(e) => eprintln!("[astrocyte] window.is_visible() failed: {}", e),
                        }
                    }
                },
            ) {
                eprintln!(
                    "[astrocyte] Failed to register global shortcut (Cmd/Ctrl+Space): {}. \
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
                        eprintln!("[astrocyte] main window not found for clipboard hijack");
                        return;
                    };

                    if let Err(e) = window.show() {
                        eprintln!("[astrocyte] window.show() failed: {}", e);
                    }
                    if let Err(e) = window.set_focus() {
                        eprintln!("[astrocyte] window.set_focus() failed: {}", e);
                    }

                    let payload = match Clipboard::new() {
                        Ok(mut clipboard) => match clipboard.get_text() {
                            Ok(text) => {
                                let trimmed = text.trim().to_string();
                                if trimmed.is_empty() {
                                    eprintln!("[astrocyte] clipboard text is empty, skip emit");
                                    return;
                                }
                                trimmed
                            }
                            Err(e) => {
                                eprintln!("[astrocyte] clipboard.get_text() failed: {}", e);
                                return;
                            }
                        },
                        Err(e) => {
                            eprintln!("[astrocyte] Clipboard::new() failed: {}", e);
                            return;
                        }
                    };

                    if let Err(e) = window.emit("clipboard-hijack", payload) {
                        eprintln!("[astrocyte] window.emit(clipboard-hijack) failed: {}", e);
                    }
                },
            ) {
                eprintln!(
                    "[astrocyte] Failed to register global shortcut (Ctrl+Alt+Shift+C): {}. \
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
            set_is_oligo_mode,
            set_active_skill_id,
            get_available_skills,
            get_config,
            save_provider,
            delete_provider,
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
            sublimate_scratchpad
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
