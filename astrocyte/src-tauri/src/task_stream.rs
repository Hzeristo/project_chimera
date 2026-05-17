use futures_util::StreamExt;
use log::{info, warn};
use reqwest_eventsource::{Event, RequestBuilderExt};
use serde_json::Value;
use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration};

const RECONNECT_BACKOFF_MS: [u64; 5] = [500, 1000, 2000, 5000, 10000];

pub async fn run_task_stream_loop(app: AppHandle, oligo_base_url: String) {
    let url = format!("{}/v1/tasks/stream", oligo_base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let mut backoff_idx = 0usize;

    loop {
        info!("[TaskStream] Connecting to {}", url);
        let mut es = match client.get(&url).eventsource() {
            Ok(es) => {
                backoff_idx = 0;
                es
            }
            Err(e) => {
                warn!("[TaskStream] Build EventSource failed: {}", e);
                let delay_ms = RECONNECT_BACKOFF_MS[backoff_idx];
                sleep(Duration::from_millis(delay_ms)).await;
                backoff_idx = (backoff_idx + 1).min(RECONNECT_BACKOFF_MS.len() - 1);
                continue;
            }
        };

        while let Some(event) = es.next().await {
            match event {
                Ok(Event::Open) => {
                    info!("[TaskStream] Connected");
                    backoff_idx = 0;
                }
                Ok(Event::Message(msg)) => {
                    let event_name = msg.event.as_str();
                    if event_name == "task-heartbeat" || event_name == "task-stream-hello" {
                        continue;
                    }
                    if !event_name.starts_with("task-") {
                        continue;
                    }
                    match serde_json::from_str::<Value>(&msg.data) {
                        Ok(payload) => {
                            if let Err(e) = app.emit(
                                "bb-task-event",
                                serde_json::json!({
                                    "event_type": event_name,
                                    "payload": payload
                                }),
                            ) {
                                warn!("[TaskStream] emit bb-task-event failed: {}", e);
                            }
                        }
                        Err(e) => {
                            warn!(
                                "[TaskStream] Invalid JSON payload for event {}: {}",
                                event_name, e
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!("[TaskStream] Stream error: {}, reconnecting", e);
                    break;
                }
            }
        }

        let delay_ms = RECONNECT_BACKOFF_MS[backoff_idx];
        sleep(Duration::from_millis(delay_ms)).await;
        backoff_idx = (backoff_idx + 1).min(RECONNECT_BACKOFF_MS.len() - 1);
    }
}
