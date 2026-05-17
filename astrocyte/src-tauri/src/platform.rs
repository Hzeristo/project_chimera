//! 平台相关路径与环境的统一抽象层。
//! 所有业务代码必须通过此模块获取路径。

use std::path::{Path, PathBuf};

use log::{info, warn};

/// 仅在目标不存在时复制文件或目录树（从旧版 `data_local_dir()/chimera` 合并）。
fn copy_if_dest_missing(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.exists() {
        return Ok(());
    }
    if src.is_file() {
        if !dst.exists() {
            if let Some(p) = dst.parent() {
                std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            std::fs::copy(src, dst).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    if !dst.exists() {
        std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    }
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        copy_if_dest_missing(&entry.path(), &dst.join(entry.file_name()))?;
    }
    Ok(())
}

/// 一次性迁移：旧版 OS 本地 `chimera` 数据目录 → `~/.chimera`（无点目录迁移在 `get_chimera_root` 内完成）。
pub fn migrate_legacy_app_data() -> Result<(), String> {
    let Some(old_root) = dirs::data_local_dir() else {
        return Ok(());
    };

    let old_chimera = old_root.join("chimera");
    let new_root = get_chimera_root()?;

    if old_chimera.is_dir() {
        copy_if_dest_missing(
            &old_chimera.join("provider_config.json"),
            &new_root.join("provider_config.json"),
        )?;
        copy_if_dest_missing(
            &old_chimera.join("scratchpad.json"),
            &new_root.join("scratchpad.json"),
        )?;
        copy_if_dest_missing(
            &old_chimera.join("scratchpad.md"),
            &new_root.join("scratchpad.md"),
        )?;

        let old_skills = old_chimera.join("skills");
        let new_skills = new_root.join("skills");
        if old_skills.is_dir() {
            std::fs::create_dir_all(&new_skills).map_err(|e| e.to_string())?;
            copy_if_dest_missing(&old_skills, &new_skills)?;
        }

        let old_hist = old_chimera.join("history");
        let new_hist = new_root.join("history");
        if old_hist.is_dir() {
            copy_if_dest_missing(&old_hist, &new_hist)?;
        }
    }

    let old_personas = old_root.join("chimera_personas.json");
    let new_personas = new_root.join("chimera_personas.json");
    copy_if_dest_missing(&old_personas, &new_personas)?;

    Ok(())
}

/// 获取 Chimera 根目录的绝对路径（与 `crucible_core` 的 `platform.get_chimera_root()` 顺序一致：先迁移 `~/chimera` 再 `mkdir`）。
pub fn get_chimera_root() -> Result<PathBuf, String> {
    let home = dirs::home_dir()
        .ok_or("Cannot determine home directory")?;

    let new_dir = home.join(".chimera");
    let old_dir = home.join("chimera");

    // 先迁移，再创建：若先 `create_dir_all`，`new_dir.exists()` 会为 true，迁移条件永不满足。
    if old_dir.exists() && !new_dir.exists() {
        info!("Migrating ~/chimera -> ~/.chimera");
        std::fs::rename(&old_dir, &new_dir).map_err(|e| {
            warn!("Failed to migrate ~/chimera -> ~/.chimera: {}", e);
            format!("Migration failed: {}", e)
        })?;
    }

    std::fs::create_dir_all(&new_dir)
        .map_err(|e| format!("Failed to create chimera dir: {}", e))?;

    new_dir
        .canonicalize()
        .map_err(|e| format!("Failed to resolve chimera dir: {}", e))
}

/// 获取配置文件路径
pub fn get_config_path() -> Result<PathBuf, String> {
    Ok(get_chimera_root()?.join("config.toml"))
}

/// 获取 Skills 目录路径
pub fn get_skills_dir() -> Result<PathBuf, String> {
    let skills_dir = get_chimera_root()?.join("skills");
    std::fs::create_dir_all(&skills_dir)
        .map_err(|e| format!("Failed to create skills dir: {}", e))?;
    Ok(skills_dir)
}

/// Skill 使用统计 JSON（与 `crucible_core` `get_skill_stats_path()` 一致）
pub fn get_skill_stats_path() -> Result<PathBuf, String> {
    Ok(get_chimera_root()?.join("skill_stats.json"))
}

/// 获取日志目录路径
pub fn get_logs_dir() -> Result<PathBuf, String> {
    let logs_dir = get_chimera_root()?.join("logs");
    std::fs::create_dir_all(&logs_dir)
        .map_err(|e| format!("Failed to create logs dir: {}", e))?;
    Ok(logs_dir)
}

/// Linux systemd service 路径（预留）
#[cfg(target_os = "linux")]
pub fn get_systemd_service_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir()
        .ok_or("Cannot determine home directory")?;

    let systemd_dir = home.join(".config").join("systemd").join("user");
    std::fs::create_dir_all(&systemd_dir)
        .map_err(|e| format!("Failed to create systemd dir: {}", e))?;

    Ok(systemd_dir.join("chimera-oligo.service"))
}

#[cfg(not(target_os = "linux"))]
pub fn get_systemd_service_path() -> Result<PathBuf, String> {
    Err("systemd is only available on Linux".to_string())
}
