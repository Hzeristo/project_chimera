// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "debug");
    }
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
        .try_init();

    astrocyte_lib::run()
}
