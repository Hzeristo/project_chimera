//! 本地技能定义：从系统数据目录读取 JSON，供 Astrocyte 组装发往 Oligo 的强类型 Payload。

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 与 `crucible_core` 侧技能契约一致：id / 展示名 / 覆写文案 / 可选工具白名单。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillDefinition {
    pub id: String,
    pub name: String,
    pub system_override: String,
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
}

fn skills_dir() -> Option<PathBuf> {
    let root = dirs::data_local_dir()?;
    Some(root.join("chimera").join("skills"))
}

fn skill_file_path(skill_id: &str) -> Option<PathBuf> {
    let id = skill_id.trim();
    if id.is_empty() || id.contains(['/', '\\']) || id.contains("..") {
        return None;
    }
    Some(skills_dir()?.join(format!("{id}.json")))
}

/// 读取 `{data_local}/chimera/skills/{skill_id}.json`；任意失败静默返回 `None`。
pub fn load_skill(skill_id: &str) -> Option<SkillDefinition> {
    let path = skill_file_path(skill_id)?;
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// 扫描 `{data_local}/chimera/skills/*.json` 并解析为技能列表。
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
