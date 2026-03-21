//! 双引擎发射层：No-Agent 直连 API / Agentic 代理 Oligo。
//! 使用 reqwest-eventsource 处理 SSE，杜绝 UTF-8 截断与 O(n²) 拷贝。

use futures_util::StreamExt;
use reqwest_eventsource::{Event, RequestBuilderExt};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;

use crate::settings::ProviderConfig;
use crate::state::Message;

const OLIGO_INVOKE_URL: &str = "http://127.0.0.1:33333/v1/agent/invoke";

/// Sentinel returned from stream functions when the user aborts generation (`evaluate_payload` matches on this).
pub const GENERATION_ABORTED: &str = "__ASTROCYTE_GENERATION_ABORTED__";

#[derive(Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    stream: bool,
}

#[derive(Serialize)]
struct OligoAgentRequest {
    persona_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    skill_id: Option<String>,
    messages: Vec<OligoMessage>,
}

#[derive(Serialize)]
struct OligoMessage {
    role: String,
    content: String,
}

fn build_chat_completions_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else {
        format!("{}/chat/completions", trimmed)
    }
}

/// 从 OpenAI 格式 data 中提取 choices[0].delta.content，遇 [DONE] 返回 None。
fn extract_openai_content(data: &str) -> Option<Option<String>> {
    let trimmed = data.trim();
    if trimmed.is_empty() {
        return Some(None);
    }
    if trimmed == "[DONE]" {
        return None;
    }
    let json = serde_json::from_str::<serde_json::Value>(trimmed).ok()?;
    let content = json
        .get("choices")?
        .get(0)?
        .get("delta")?
        .get("content")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);
    Some(content)
}

/// No-Agent 模式：EventSource 解析 SSE，直连 OpenAI/DeepSeek。
pub async fn stream_direct_api(
    messages: Vec<Message>,
    provider: ProviderConfig,
    app_handle: &AppHandle,
    cancel_token: CancellationToken,
) -> Result<String, String> {
    provider.validate()?;
    let client = reqwest::Client::new();
    let body = OpenAiChatRequest {
        model: provider.model_name,
        messages: messages
            .iter()
            .map(|m| OpenAiMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect(),
        stream: true,
    };
    let url = build_chat_completions_url(&provider.base_url);

    let mut es = client
        .post(&url)
        .bearer_auth(&provider.api_key)
        .json(&body)
        .eventsource()
        .map_err(|e| format!("EventSource init failed: {}", e))?;

    let mut full_text = String::new();

    loop {
        tokio::select! {
            item = es.next() => {
                let Some(item) = item else { break };
                match item {
                    Ok(Event::Open) => {}
                    Ok(Event::Message(msg)) => {
                        let data = msg.data.trim();
                        match extract_openai_content(data) {
                            None => break,
                            Some(None) => {}
                            Some(Some(content)) => {
                                full_text.push_str(&content);
                                app_handle
                                    .emit("bb-stream-chunk", content)
                                    .map_err(|e| format!("emit failed: {}", e))?;
                            }
                        }
                    }
                    Err(e) => {
                        if !full_text.is_empty() {
                            return Ok(full_text);
                        }
                        return Err(format!("SSE stream error: {}", e));
                    }
                }
            }
            _ = cancel_token.cancelled() => {
                let _ = app_handle.emit("bb-sys-event", "[Generation Aborted by User]");
                return Err(GENERATION_ABORTED.to_string());
            }
        }
    }

    Ok(full_text)
}

/// Agentic 模式：EventSource 解析 SSE，data 帧内容全透传 emit。
pub async fn stream_oligo_agent(
    persona_id: String,
    skill_id: Option<String>,
    messages: Vec<Message>,
    app_handle: &AppHandle,
    cancel_token: CancellationToken,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("reqwest client build failed: {}", e))?;

    let body = OligoAgentRequest {
        persona_id,
        skill_id,
        messages: messages
            .iter()
            .map(|m| OligoMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect(),
    };

    let mut es = client
        .post(OLIGO_INVOKE_URL)
        .json(&body)
        .eventsource()
        .map_err(|e| format!("Oligo EventSource init failed: {}", e))?;

    let mut full_text = String::new();

    loop {
        tokio::select! {
            item = es.next() => {
                let Some(item) = item else { break };
                match item {
                    Ok(Event::Open) => {}
                    Ok(Event::Message(msg)) => {
                        let data = msg.data.trim();
                        if data == "[DONE]" {
                            break;
                        }
                        if data.is_empty() {
                            continue;
                        }

                        if let Ok(json_obj) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(content_str) = json_obj.get("content").and_then(|v| v.as_str()) {
                                if content_str.starts_with("__SYS_TOOL_CALL__") {
                                    let action_detail =
                                        content_str.trim_start_matches("__SYS_TOOL_CALL__");
                                    app_handle
                                        .emit("bb-sys-event", action_detail)
                                        .map_err(|e| format!("emit failed: {}", e))?;
                                } else {
                                    full_text.push_str(content_str);
                                    app_handle
                                        .emit("bb-stream-chunk", content_str)
                                        .map_err(|e| format!("emit failed: {}", e))?;
                                }
                            }
                        } else if let Some(action_detail) = data.strip_prefix("__SYS_TOOL_CALL__") {
                            app_handle
                                .emit("bb-sys-event", action_detail)
                                .map_err(|e| format!("emit failed: {}", e))?;
                        } else {
                            full_text.push_str(data);
                            app_handle
                                .emit("bb-stream-chunk", data)
                                .map_err(|e| format!("emit failed: {}", e))?;
                        }
                    }
                    Err(e) => {
                        if !full_text.is_empty() {
                            return Ok(full_text);
                        }
                        return Err(format!("Oligo SSE stream error: {}", e));
                    }
                }
            }
            _ = cancel_token.cancelled() => {
                let _ = app_handle.emit("bb-sys-event", "[Generation Aborted by User]");
                return Err(GENERATION_ABORTED.to_string());
            }
        }
    }

    Ok(full_text)
}
