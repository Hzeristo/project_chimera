// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::Write;

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "debug");
    }
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
        .format(|buf, record| {
            writeln!(
                buf,
                "{} | {:<8} | {} | {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                format!("{}", record.level()),
                record.target(),
                record.args()
            )
        })
        .try_init();

    match astrocyte_lib::config::load_config() {
        Ok(cfg) => log::info!(
            "[Astrocyte] Chimera Oligo invoke URL (from ~/.chimera/config.toml): {}",
            cfg.oligo_agent_invoke_url()
        ),
        Err(e) => log::warn!("[Astrocyte] Chimera config.toml not loaded: {}", e),
    }

    astrocyte_lib::run()
}
