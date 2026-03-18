use futures_util::StreamExt;
use regex::Regex;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::settings::ProviderConfig;
use crate::state::Message;
use crate::tools;

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

fn build_chat_completions_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else {
        format!("{}/chat/completions", trimmed)
    }
}

async fn request_openai_compatible_completion(
    client: &reqwest::Client,
    provider: &ProviderConfig,
    messages: Vec<OpenAiMessage>,
) -> Result<reqwest::Response, String> {
    let body = OpenAiChatRequest {
        model: provider.model_name.clone(),
        messages,
        stream: true,
    };
    let endpoint = build_chat_completions_url(&provider.base_url);

    let response = client
        .post(endpoint)
        .bearer_auth(&provider.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("「BB」provider request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let err_body = response
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read error body>".to_string());
        return Err(format!("「BB」provider API error {}: {}", status, err_body));
    }

    Ok(response)
}

fn extract_fragment_from_sse_line(line: &str) -> Result<Option<String>, String> {
    let Some(data) = line.strip_prefix("data:") else {
        return Ok(None);
    };

    let trimmed = data.trim();
    if trimmed == "[DONE]" || trimmed.is_empty() {
        return Ok(None);
    }

    let json: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|e| format!("「BB」provider stream JSON parse failed: {}", e))?;

    let Some(fragment) = json
        .get("choices")
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("delta"))
        .and_then(|v| v.get("content"))
        .and_then(|v| v.as_str())
    else {
        return Ok(None);
    };

    if fragment.is_empty() {
        return Ok(None);
    }

    Ok(Some(fragment.to_string()))
}

fn longest_cmd_prefix_suffix_len(buffer: &str, cmd_prefix: &str) -> usize {
    let max = buffer.len().min(cmd_prefix.len());
    for len in (1..=max).rev() {
        if buffer.ends_with(&cmd_prefix[..len]) {
            return len;
        }
    }
    0
}

fn phase_one_fragment_step(
    fragment: &str,
    app: &AppHandle,
    cmd_regex: &Regex,
    intercept_buffer: &mut String,
    full_response_text: &mut String,
    bypass_mode: &mut bool,
) -> Result<Option<String>, String> {
    const NORMAL_TOLERANCE_CHARS: usize = 10;
    const CMD_PREFIX: &str = r#"<CMD:search_vault(""#;

    // 保留参数以兼容调用签名；新的阶段一策略不再进入“永久旁路”。
    *bypass_mode = false;

    intercept_buffer.push_str(fragment);

    if let Some(captures) = cmd_regex.captures(intercept_buffer) {
        let extracted = captures
            .get(1)
            .map(|m| m.as_str().trim().to_string())
            .filter(|s| !s.is_empty());
        if let Some(keyword) = extracted {
            return Ok(Some(keyword));
        }
    }

    // 1) 如果已出现完整命令前缀但尚未闭合 ")>"，则从前缀起全部冻结。
    let mut hold_from = intercept_buffer.len();
    if let Some(start_idx) = intercept_buffer.rfind(CMD_PREFIX) {
        let tail = &intercept_buffer[start_idx..];
        if !tail.contains(")>") {
            hold_from = start_idx;
        }
    }

    // 2) 否则仅冻结末尾可能构成命令前缀的残片（处理跨 chunk 拆分）。
    if hold_from == intercept_buffer.len() {
        let hold_tail_len = longest_cmd_prefix_suffix_len(intercept_buffer, CMD_PREFIX);
        hold_from = intercept_buffer.len().saturating_sub(hold_tail_len);
    }

    let releasable_len = hold_from;
    if releasable_len == 0 {
        return Ok(None);
    }

    let releasable_preview = &intercept_buffer[..releasable_len];
    if releasable_preview.chars().count() < NORMAL_TOLERANCE_CHARS
        && hold_from == intercept_buffer.len()
    {
        return Ok(None);
    }

    let to_emit = intercept_buffer[..releasable_len].to_string();
    // let broken_by_newline = to_emit.contains('\n');
    // let too_long_without_closure = false;
    // if broken_by_newline || too_long_without_closure {
    // }
    // let trimmed_buf = to_emit.trim_start();
    intercept_buffer.drain(..releasable_len);
    full_response_text.push_str(&to_emit);
    app.emit("bb-stream-chunk", to_emit)
        .map_err(|e| format!("「BB」emit bb-stream-chunk failed: {}", e))?;

    Ok(None)
}

pub async fn stream_llm_response(
    messages: Vec<Message>,
    provider: ProviderConfig,
    app_handle: &AppHandle,
) -> Result<String, String> {
    provider.validate()?;
    let client = reqwest::Client::new();
    let cmd_regex = Regex::new(r#"<CMD:search_vault\("([^"]+)"\)>"#)
        .map_err(|e| format!("「BB」invalid command regex: {}", e))?;

    let openai_messages = messages
        .iter()
        .map(|m| OpenAiMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect::<Vec<_>>();

    let response =
        request_openai_compatible_completion(&client, &provider, openai_messages).await?;
    let mut stream = response.bytes_stream();
    let mut carry = String::new();
    let mut intercept_buffer = String::new();
    let mut full_response_text = String::new();
    let mut bypass_mode = false;
    let mut extracted_keyword: Option<String> = None;

    'phase_one: while let Some(next_chunk) = stream.next().await {
        let chunk = next_chunk
            .map_err(|e| format!("「BB」provider stream read failed (phase1): {}", e))?;
        carry.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = carry.find('\n') {
            let line = carry[..pos].trim_end_matches('\r').to_string();
            carry.drain(..=pos);
            if let Some(fragment) = extract_fragment_from_sse_line(&line)? {
                if let Some(keyword) = phase_one_fragment_step(
                    &fragment,
                    app_handle,
                    &cmd_regex,
                    &mut intercept_buffer,
                    &mut full_response_text,
                    &mut bypass_mode,
                )? {
                    extracted_keyword = Some(keyword);
                    break 'phase_one;
                }
            }
        }
    }

    if extracted_keyword.is_none() {
        let tail = carry.trim();
        if !tail.is_empty() {
            if let Some(fragment) = extract_fragment_from_sse_line(tail)? {
                if let Some(keyword) = phase_one_fragment_step(
                    &fragment,
                    app_handle,
                    &cmd_regex,
                    &mut intercept_buffer,
                    &mut full_response_text,
                    &mut bypass_mode,
                )? {
                    extracted_keyword = Some(keyword);
                }
            }
        }
    }

    if let Some(keyword) = extracted_keyword {

        let tool_result = tools::delegate_search_vault(&keyword).await;

        let mut reroll_messages = messages
            .iter()
            .map(|m| OpenAiMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect::<Vec<_>>();
        reroll_messages.push(OpenAiMessage {
            role: "assistant".to_string(),
            content: format!(r#"<CMD:search_vault("{}")>"#, keyword),
        });
        reroll_messages.push(OpenAiMessage {
            role: "user".to_string(),
            content: format!(
                "[SYSTEM TOOL RESULT]: External Exocortex returned this data for your query. \
                You MUST now provide the final answer maintaining your persona. DO NOT output the <CMD> tag again.\n\nData:\n{}",
                tool_result
            ),
        });

        let second_response =
            request_openai_compatible_completion(&client, &provider, reroll_messages).await?;
        let mut second_stream = second_response.bytes_stream();
        let mut second_carry = String::new();
        let mut rerolled_full_response = String::new();

        while let Some(next_chunk) = second_stream.next().await {
            let chunk = next_chunk
                .map_err(|e| format!("「BB」provider stream read failed (phase2): {}", e))?;
            second_carry.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = second_carry.find('\n') {
                let line = second_carry[..pos].trim_end_matches('\r').to_string();
                second_carry.drain(..=pos);
                if let Some(fragment) = extract_fragment_from_sse_line(&line)? {
                    rerolled_full_response.push_str(&fragment);
                    app_handle
                        .emit("bb-stream-chunk", fragment)
                        .map_err(|e| format!("「BB」emit bb-stream-chunk failed: {}", e))?;
                }
            }
        }

        let second_tail = second_carry.trim();
        if !second_tail.is_empty() {
            if let Some(fragment) = extract_fragment_from_sse_line(second_tail)? {
                rerolled_full_response.push_str(&fragment);
                app_handle
                    .emit("bb-stream-chunk", fragment)
                    .map_err(|e| format!("「BB」emit bb-stream-chunk failed: {}", e))?;
            }
        }

        return Ok(rerolled_full_response);
    }

    if !intercept_buffer.is_empty() {
        full_response_text.push_str(&intercept_buffer);
        app_handle
            .emit("bb-stream-chunk", intercept_buffer)
            .map_err(|e| format!("「BB」emit bb-stream-chunk failed: {}", e))?;
    }

    Ok(full_response_text)
}
