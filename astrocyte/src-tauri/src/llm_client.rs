//! 双引擎发射层：No-Agent 直连 API / Agentic 代理 Oligo。
//! 使用 reqwest-eventsource 处理 SSE，杜绝 UTF-8 截断与 O(n²) 拷贝。

use futures_util::StreamExt;
use reqwest_eventsource::{Event, RequestBuilderExt};
use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;

use crate::settings::ProviderConfig;
use crate::state::Message;

/// Sentinel returned from stream functions when the user aborts generation (`evaluate_payload` matches on this).
pub const GENERATION_ABORTED: &str = "__ASTROCYTE_GENERATION_ABORTED__";

/// 将 Oligo `__SYS_TOOL_CALL__` 后缀解析为带 `stage` 的 JSON（兼容旧式纯文本）。
fn normalize_bb_sys_payload(detail: &str) -> serde_json::Value {
    let detail = detail.trim();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(detail) {
        if v.get("stage").and_then(|s| s.as_str()).is_some() {
            return v;
        }
    }
    if detail.starts_with("parallel::") {
        return json!({
            "stage": "router",
            "content": detail,
            "decision": "parallel"
        });
    }
    if detail.starts_with("wash::") {
        let rest = detail.strip_prefix("wash::").unwrap_or("").trim();
        let tool_name = rest.split("::").next().unwrap_or("").to_string();
        return json!({
            "stage": "wash",
            "content": detail,
            "tool_name": tool_name
        });
    }
    if detail.starts_with("denied::") {
        let tn = detail.strip_prefix("denied::").unwrap_or("").trim().to_string();
        return json!({
            "stage": "tool",
            "content": detail,
            "tool_name": tn
        });
    }
    if detail.starts_with("completed::") {
        let mut parts = detail.split("::");
        let _ = parts.next();
        let tool_name = parts.next().unwrap_or("").to_string();
        return json!({
            "stage": "tool",
            "content": detail,
            "tool_name": tool_name
        });
    }
    if let Some(idx) = detail.find("::") {
        let tool = detail[..idx].to_string();
        return json!({
            "stage": "router",
            "content": detail,
            "tool_name": tool,
            "decision": tool
        });
    }
    json!({
        "stage": "router",
        "content": detail
    })
}

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

/// 与 Oligo `AgentInvokeRequest`（Pydantic）JSON 同构：键名为 snake_case，与 Rust 字段名一致。
/// `Option` 为 `None` 的字段不序列化（`skip_serializing_if`），与省略键的 JSON 解析一致。
#[derive(Serialize)]
struct OligoAgentRequest {
    api_key: String,
    base_url: String,
    model_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    persona_id: Option<String>,
    system_core: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    skill_override: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skill_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    persona: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    authors_note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
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

fn build_oligo_invoke_url(oligo_base_url: &str) -> Result<String, String> {
    let trimmed = oligo_base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("oligo_base_url is empty".to_string());
    }
    Ok(format!("{}/v1/agent/invoke", trimmed))
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
/// `oligo_base_url`：Oligo 网关根地址（如 ``http://127.0.0.1:33333``），由 Astrocyte 配置或 ``OLIGO_BASE_URL`` 提供。
/// `api_key` / `base_url` / `model_name` 来自当前 Provider；`system_core` / skill / `persona` / `authors_note` 由调用方按注入协议装配；
/// `messages` 仅含 user/assistant 与本轮 user，禁止预拼 System。
///
/// - ``Ok(Some(text))``：正常结束（由上层再发 ``bb-stream-done`` DONE）。
/// - ``Ok(None)``：已收到服务端 ``event: bb-stream-done`` 并已 ``emit``，上层不得再发 DONE。
pub async fn stream_oligo_agent(
    oligo_base_url: &str,
    api_key: String,
    base_url: String,
    model_name: String,
    persona_id: Option<String>,
    system_core: String,
    skill_override: Option<String>,
    skill_id: Option<String>,
    allowed_tools: Option<Vec<String>>,
    persona: Option<String>,
    authors_note: Option<String>,
    // None: JSON 省略该字段，Oligo 用 Chimera 默认 working 温度
    temperature: Option<f64>,
    messages: Vec<Message>,
    app_handle: &AppHandle,
    cancel_token: CancellationToken,
) -> Result<Option<String>, String> {
    let invoke_url = build_oligo_invoke_url(oligo_base_url)
        .map_err(|e| format!("Invalid Oligo gateway URL: {}", e))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("reqwest client build failed: {}", e))?;

    let body = OligoAgentRequest {
        api_key,
        base_url,
        model_name,
        persona_id,
        system_core,
        skill_override,
        skill_id,
        allowed_tools,
        persona,
        authors_note,
        temperature,
        messages: messages
            .iter()
            .map(|m| OligoMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect(),
    };

    let mut es = client
        .post(&invoke_url)
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
                        if msg.event == "bb-stream-done" {
                            let payload = if data.is_empty() {
                                serde_json::json!({ "error": true, "message": "empty bb-stream-done" })
                            } else {
                                serde_json::from_str::<serde_json::Value>(data).unwrap_or_else(|_| {
                                    serde_json::json!({ "error": true, "message": data })
                                })
                            };
                            app_handle
                                .emit("bb-stream-done", payload)
                                .map_err(|e| format!("emit failed: {}", e))?;
                            return Ok(None);
                        }
                        if msg.event == "bb-tool-start" || msg.event == "bb-tool-done" {
                            let payload: serde_json::Value = if data.is_empty() {
                                serde_json::json!({})
                            } else {
                                serde_json::from_str(data).unwrap_or_else(|_| {
                                    serde_json::json!({ "raw": data })
                                })
                            };
                            let ev = if msg.event == "bb-tool-start" {
                                "bb-tool-start"
                            } else {
                                "bb-tool-done"
                            };
                            app_handle
                                .emit(ev, payload)
                                .map_err(|e| format!("emit {} failed: {}", ev, e))?;
                            continue;
                        }
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
                                    let payload = normalize_bb_sys_payload(action_detail);
                                    app_handle
                                        .emit("bb-sys-event", payload)
                                        .map_err(|e| format!("emit failed: {}", e))?;
                                } else {
                                    full_text.push_str(content_str);
                                    app_handle
                                        .emit("bb-stream-chunk", content_str)
                                        .map_err(|e| format!("emit failed: {}", e))?;
                                }
                            }
                        } else if let Some(action_detail) = data.strip_prefix("__SYS_TOOL_CALL__") {
                            let payload = normalize_bb_sys_payload(action_detail);
                            app_handle
                                .emit("bb-sys-event", payload)
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
                            return Ok(Some(full_text));
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

    Ok(Some(full_text))
}
