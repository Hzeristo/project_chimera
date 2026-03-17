use arboard::Clipboard;
use chrono::{SecondsFormat, Utc};
use log::{debug, warn};
use memory::{append_entries, get_entries_for_session, get_session_summaries, ChatEntry, SessionSummary};
use reqwest::header::AUTHORIZATION;
use std::time::Duration;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use uuid::Uuid;

mod llm_client;
mod memory;
mod persona;
mod settings;
mod state;
mod tools;
use persona::{PersonaConfig, PersonaSnapshot};
use settings::{save_astrocyte_config, AstrocyteConfig, ProviderConfig};
use state::{AstrocyteState, Message};

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
    messages: Vec<Message>,
    state: tauri::State<'_, AstrocyteState>,
) -> Result<(), String> {
    let normalized_session_id = normalize_session_id(Some(session_id));
    state.create_session(normalized_session_id.clone()).await;
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

fn build_entry(role: &str, content: String, persona: Option<&str>, session_id: &str) -> ChatEntry {
    ChatEntry {
        id: Uuid::new_v4().to_string(),
        timestamp: now_timestamp(),
        role: role.to_string(),
        content,
        session_id: session_id.to_string(),
        persona: persona.map(|value| value.to_string()),
    }
}

fn persist_chat_entries_non_blocking(entries: Vec<ChatEntry>) {
    tauri::async_runtime::spawn(async move {
        let write_result = tauri::async_runtime::spawn_blocking(move || append_entries(&entries)).await;
        match write_result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => eprintln!("[astrocyte] memory append failed: {}", e),
            Err(e) => eprintln!("[astrocyte] memory append task join failed: {}", e),
        }
    });
}

async fn build_session_messages(
    state: &AstrocyteState,
    session_id: &str,
) -> Result<Vec<Message>, String> {
    let history = state.get_history(session_id).await.unwrap_or_default();
    let active_persona = state.active_persona.read().await.clone();

    let mut messages = Vec::with_capacity(history.len() + 2);
    messages.push(Message {
        role: "system".to_string(),
        content: active_persona.system_prompt,
    });
    if let Some(note) = active_persona.authors_note {
        if !note.trim().is_empty() {
            messages.push(Message {
                role: "system".to_string(),
                content: format!("[AUTHOR_NOTE]: {}", note),
            });
        }
    }
    messages.extend(history);
    Ok(messages)
}

#[tauri::command]
async fn evaluate_payload(
    payload: String,
    session_id: Option<String>,
    state: tauri::State<'_, AstrocyteState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let user_input = payload.trim().to_string();
    if user_input.is_empty() {
        return Err("payload is empty".to_string());
    }

    let active_persona = state.active_persona.read().await.clone();
    let active_persona_id = active_persona.id.clone();

    let session_id = normalize_session_id(session_id);
    state.create_session(session_id.clone()).await;
    state
        .append_message_to_session(&session_id, "user", user_input.clone())
        .await?;

    let outbound_messages = build_session_messages(&state, &session_id).await?;
    let config = state.config.read().await.clone();
    let provider = config
        .active_provider()
        .cloned()
        .ok_or_else(|| "no active provider configured".to_string())?;
    let app_handle = app.clone();

    tauri::async_runtime::spawn(async move {
        let model_reply =
            match llm_client::stream_llm_response(outbound_messages, provider, &app_handle).await {
            Ok(reply) => reply,
            Err(e) => {
                warn!("[astrocyte] stream request failed: {}", e);
                if let Err(emit_err) = app_handle.emit("bb-stream-chunk", e.clone()) {
                    warn!("[astrocyte] emit bb-stream-chunk failed after error: {}", emit_err);
                }
                if let Err(emit_err) =
                    app_handle.emit("bb-stream-done", serde_json::json!({ "error": true, "message": e }))
                {
                    warn!("[astrocyte] emit bb-stream-done failed after error: {}", emit_err);
                }
                return;
            }
        };

        let state = app_handle.state::<AstrocyteState>();
        if let Err(e) = state
            .append_message_to_session(&session_id, "assistant", model_reply.clone())
            .await
        {
            warn!("[astrocyte] append assistant message to state failed: {}", e);
        }

        let entries = vec![
            build_entry("user", user_input, None, &session_id),
            build_entry("bb", model_reply, Some(&active_persona_id), &session_id),
        ];
        persist_chat_entries_non_blocking(entries);

        if let Err(e) = app_handle.emit("bb-stream-done", "DONE") {
            warn!("[astrocyte] emit bb-stream-done failed: {}", e);
        }
    });

    Ok(())
}

#[tauri::command]
async fn get_session_history() -> Result<Vec<SessionSummary>, String> {
    let result = tauri::async_runtime::spawn_blocking(get_session_summaries)
        .await
        .map_err(|e| format!("failed to join session summary task: {}", e))?;
    result
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AstrocyteState::new())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
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
            evaluate_payload,
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
            load_session_archive
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
