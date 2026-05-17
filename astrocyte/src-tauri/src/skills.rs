//! 本地技能定义：从系统数据目录读取 JSON，供 Astrocyte 组装发往 Oligo 的强类型 Payload。

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::skill_stats::{self, AggregatedSkillStats};

fn default_category() -> String {
    "general".to_string()
}

fn default_version() -> String {
    "1.0.0".to_string()
}

/// 与 `crucible_core` 侧 `SkillDefinition` 契约一致（纯净定义；统计在 `~/.chimera/skill_stats.json`）。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillDefinition {
    /// Skill 唯一标识
    pub id: String,
    /// Skill 显示名称
    pub name: String,
    /// Skill 用途说明
    #[serde(default)]
    pub description: String,
    /// L1 层注入的认知姿态
    pub system_override: String,
    /// 白名单工具；`None` 表示不限制
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    /// 类别：forensic, architectural, mathematical, ontological
    #[serde(default = "default_category")]
    pub category: String,
    /// 适用的论文类型：benchmark, survey, architecture, theory
    #[serde(default)]
    pub target_paper_type: Vec<String>,
    /// 期望的输出格式（如 `bullet_points`, `structured_json`）
    pub expected_output_format: Option<String>,
    /// Skill 版本
    #[serde(default = "default_version")]
    pub version: String,
    /// 最后更新时间
    pub last_updated: Option<String>,
}

/// 技能定义 + `skill_stats.json` 聚合统计（供 Astrocyte 卡片网格）。
#[derive(Debug, Clone, Serialize)]
pub struct SkillWithStats {
    #[serde(flatten)]
    pub definition: SkillDefinition,
    pub stats: AggregatedSkillStats,
}

fn skills_dir() -> Option<PathBuf> {
    crate::platform::get_skills_dir().ok()
}

fn skill_file_path(skill_id: &str) -> Option<PathBuf> {
    let id = skill_id.trim();
    if id.is_empty() || id.contains(['/', '\\']) || id.contains("..") {
        return None;
    }
    Some(skills_dir()?.join(format!("{id}.json")))
}

/// 读取 `~/.chimera/skills/{skill_id}.json`；任意失败静默返回 `None`。
pub fn load_skill(skill_id: &str) -> Option<SkillDefinition> {
    let path = skill_file_path(skill_id)?;
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// 扫描 `~/.chimera/skills/*.json` 并解析为技能列表。
pub fn load_all_skills() -> Vec<SkillDefinition> {
    let Some(dir) = skills_dir() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut skills = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(skill) = serde_json::from_str::<SkillDefinition>(&raw) else {
            continue;
        };
        skills.push(skill);
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

/// 扫描技能目录并与统计文件合并。
pub fn load_all_skills_with_stats() -> Vec<SkillWithStats> {
    let map = skill_stats::load_skill_stats_map();
    load_all_skills()
        .into_iter()
        .map(|s| {
            let stats = map
                .get(&s.id)
                .map(skill_stats::aggregated_from_value)
                .unwrap_or_default();
            SkillWithStats {
                definition: s,
                stats,
            }
        })
        .collect()
}
