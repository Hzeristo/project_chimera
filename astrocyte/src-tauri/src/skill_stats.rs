//! 与 `crucible_core` `SkillStatsService` 同结构的 `~/.chimera/skill_stats.json` 追加写入。

use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use serde_json::{json, Map, Number, Value};
use std::fs;

use crate::platform;

/// 与 Python `SkillStatsService.get_stats` 返回语义一致的聚合视图（供 HUD 卡片展示）。
#[derive(Debug, Clone, Serialize, Default)]
pub struct AggregatedSkillStats {
    pub usage_count: i64,
    pub success_rate: f64,
    pub avg_tokens: i64,
}

/// 读取完整统计表（失败或缺文件时返回空 Map）。
pub fn load_skill_stats_map() -> Map<String, Value> {
    let Ok(path) = platform::get_skill_stats_path() else {
        return Map::new();
    };
    if !path.is_file() {
        return Map::new();
    }
    let Ok(raw) = fs::read_to_string(&path) else {
        return Map::new();
    };
    match serde_json::from_str::<Value>(&raw) {
        Ok(Value::Object(m)) => m,
        _ => Map::new(),
    }
}

fn i64_from_val(v: Option<&Value>) -> i64 {
    v.and_then(|x| x.as_i64())
        .or_else(|| v.and_then(|x| x.as_u64().map(|u| u as i64)))
        .unwrap_or(0)
}

/// 从 `skill_stats.json` 中单条记录算出聚合统计。
pub fn aggregated_from_value(v: &Value) -> AggregatedSkillStats {
    let Some(obj) = v.as_object() else {
        return AggregatedSkillStats::default();
    };
    let usage = i64_from_val(obj.get("usage_count"));
    let success_count = i64_from_val(obj.get("success_count"));
    let total_tokens = i64_from_val(obj.get("total_tokens"));
    AggregatedSkillStats {
        usage_count: usage,
        success_rate: if usage > 0 {
            success_count as f64 / usage as f64
        } else {
            0.0
        },
        avg_tokens: if usage > 0 {
            total_tokens / usage
        } else {
            0
        },
    }
}

fn utc_iso_z() -> String {
    Utc::now()
        .to_rfc3339_opts(SecondsFormat::Secs, true)
        .replace("+00:00", "Z")
}

/// 记录一条用户反馈（满意/不满意），字段语义与 Python `SkillStatsService.record_usage` 对齐。
pub fn record_skill_feedback(skill_id: &str, success: bool, tokens: i32) -> Result<(), String> {
    let sid = skill_id.trim();
    if sid.is_empty() {
        return Ok(());
    }
    let tok = tokens.max(0) as i64;

    let path = platform::get_skill_stats_path()?;
    let mut stats: Map<String, Value> = if path.is_file() {
        let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        match serde_json::from_str::<Value>(&raw) {
            Ok(Value::Object(m)) => m,
            Ok(_) => Map::new(),
            Err(_) => Map::new(),
        }
    } else {
        Map::new()
    };

    let entry = stats.entry(sid.to_string()).or_insert_with(|| {
        json!({
            "usage_count": 0,
            "success_count": 0,
            "total_tokens": 0,
            "feedback_history": [],
        })
    });

    let obj = entry
        .as_object_mut()
        .ok_or_else(|| "corrupt skill_stats entry".to_string())?;

    let usage = obj
        .get("usage_count")
        .and_then(|v| v.as_i64())
        .or_else(|| obj.get("usage_count").and_then(|v| v.as_u64().map(|u| u as i64)))
        .unwrap_or(0);
    let success_count = obj
        .get("success_count")
        .and_then(|v| v.as_i64())
        .or_else(|| obj.get("success_count").and_then(|v| v.as_u64().map(|u| u as i64)))
        .unwrap_or(0);
    let total_tokens = obj
        .get("total_tokens")
        .and_then(|v| v.as_i64())
        .or_else(|| obj.get("total_tokens").and_then(|v| v.as_u64().map(|u| u as i64)))
        .unwrap_or(0);

    obj.insert(
        "usage_count".to_string(),
        Value::Number(Number::from(usage + 1)),
    );
    obj.insert(
        "success_count".to_string(),
        Value::Number(Number::from(success_count + if success { 1 } else { 0 })),
    );
    obj.insert(
        "total_tokens".to_string(),
        Value::Number(Number::from(total_tokens + tok)),
    );
    obj.insert(
        "last_used".to_string(),
        Value::String(utc_iso_z()),
    );

    let mut hist: Vec<Value> = obj
        .get("feedback_history")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    hist.push(json!({
        "timestamp": utc_iso_z(),
        "success": success,
        "tokens": tok,
    }));
    if hist.len() > 100 {
        hist = hist.split_off(hist.len() - 100);
    }
    obj.insert("feedback_history".to_string(), Value::Array(hist));

    let parent = path
        .parent()
        .ok_or_else(|| "skill_stats path has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;

    let fname = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "invalid skill_stats file name".to_string())?;
    let tmp_path = parent.join(format!("{}.tmp", fname));

    let body = serde_json::to_string_pretty(&Value::Object(stats)).map_err(|e| e.to_string())?;
    fs::write(&tmp_path, format!("{body}\n")).map_err(|e| e.to_string())?;
    fs::rename(&tmp_path, &path).map_err(|e| e.to_string())?;
    Ok(())
}
