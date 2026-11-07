//! Rust desktop dictation: capture, local NVIDIA inference, and text insertion.
pub mod audio;
pub mod engines;
mod inject;
pub mod ipc;
pub mod models;
mod pipeline;
pub mod settings;
mod ui;

pub fn run() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    ui::run()
}
