//! 分段反馈：仅聚合用户点选 good/bad；按 stage 与按工具名分桶。NDJSON 审计日志见 `segment_feedback_log.jsonl`。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;

use crate::platform;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RatioBucket {
    pub good: i64,
    pub bad: i64,
}

/// `stage_stats.json`：用户反馈计数（无向量化语义分）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StageStatsFile {
    /// 各 stage 的 good/bad，键为 `router` | `tool` | `wash` | `final`。
    #[serde(default)]
    pub stage_feedback: BTreeMap<String, RatioBucket>,
    /// `tool` 评价按工具名再分桶（`stage == "tool"` 且提供了 `tool_name` 时写入）。
    #[serde(default)]
    pub tool_breakdown: BTreeMap<String, RatioBucket>,
}

/// 历史格式（以「语义化」键名存四维；现已废弃，由 `load_stats` 迁移到新结构）。
#[derive(Debug, Clone, Deserialize, Default)]
struct LegacyStageStatsFile {
    #[serde(default)]
    router_accuracy: RatioBucket,
    #[serde(default)]
    tool_relevance: RatioBucket,
    #[serde(default)]
    wash_quality: RatioBucket,
    #[serde(default)]
    final_coherence: RatioBucket,
}

fn from_legacy(legacy: LegacyStageStatsFile) -> StageStatsFile {
    let mut stage_feedback: BTreeMap<String, RatioBucket> = BTreeMap::new();
    stage_feedback.insert("router".to_string(), legacy.router_accuracy);
    stage_feedback.insert("tool".to_string(), legacy.tool_relevance);
    stage_feedback.insert("wash".to_string(), legacy.wash_quality);
    stage_feedback.insert("final".to_string(), legacy.final_coherence);
    StageStatsFile {
        stage_feedback,
        tool_breakdown: BTreeMap::new(),
    }
}

fn stats_path() -> Result<std::path::PathBuf, String> {
    Ok(platform::get_chimera_root()?.join("stage_stats.json"))
}

fn log_path() -> Result<std::path::PathBuf, String> {
    Ok(platform::get_chimera_root()?.join("segment_feedback_log.jsonl"))
}

fn valid_stage_key(stage: &str) -> Option<&'static str> {
    match stage.trim() {
        "router" => Some("router"),
        "tool" => Some("tool"),
        "wash" => Some("wash"),
        "final" => Some("final"),
        _ => None,
    }
}

fn load_stats() -> StageStatsFile {
    let Ok(path) = stats_path() else {
        return StageStatsFile::default();
    };
    if !path.is_file() {
        return StageStatsFile::default();
    }
    let Ok(raw) = fs::read_to_string(&path) else {
        return StageStatsFile::default();
    };
    if let Ok(s) = serde_json::from_str::<StageStatsFile>(&raw) {
        return s;
    }
    if let Ok(legacy) = serde_json::from_str::<LegacyStageStatsFile>(&raw) {
        return from_legacy(legacy);
    }
    StageStatsFile::default()
}

fn save_stats(s: &StageStatsFile) -> Result<(), String> {
    let path = stats_path()?;
    let parent = path
        .parent()
        .ok_or_else(|| "stage_stats path has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let fname = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "invalid stage_stats file name".to_string())?;
    let tmp = parent.join(format!("{}.tmp", fname));
    let body = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    fs::write(&tmp, format!("{body}\n")).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

/// 记录一条分段反馈，并仅更新 `stage_feedback` / `tool_breakdown` 的计数（无隐式「语义分」计算）。
pub fn record_segment_feedback(
    conversation_id: &str,
    stage: &str,
    tool_name: Option<&str>,
    decision: Option<&str>,
    rating: &str,
    reason: Option<&str>,
) -> Result<(), String> {
    let cid = conversation_id.trim();
    if cid.is_empty() {
        return Err("conversation_id is empty".to_string());
    }
    let r = rating.trim();
    if r != "good" && r != "bad" {
        return Err("rating must be good or bad".to_string());
    }
    let good = r == "good";

    let log_file = log_path()?;
    if let Some(parent) = log_file.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let line = serde_json::json!({
        "conversation_id": cid,
        "stage": stage,
        "tool_name": tool_name,
        "decision": decision,
        "rating": r,
        "reason": reason,
    });
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| e.to_string())?;
    writeln!(f, "{}", line).map_err(|e| e.to_string())?;

    let Some(stage_key) = valid_stage_key(stage) else {
        return Ok(());
    };

    let mut stats = load_stats();
    {
        let bucket = stats
            .stage_feedback
            .entry(stage_key.to_string())
            .or_default();
        if good {
            bucket.good += 1;
        } else {
            bucket.bad += 1;
        }
    }
    if stage_key == "tool" {
        if let Some(tn) = tool_name {
            let name = tn.trim();
            if !name.is_empty() {
                let bucket = stats.tool_breakdown.entry(name.to_string()).or_default();
                if good {
                    bucket.good += 1;
                } else {
                    bucket.bad += 1;
                }
            }
        }
    }

    save_stats(&stats)
}
