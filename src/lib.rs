// Core library modules for the CLI tool
pub mod analysis;
pub mod build;
pub mod commands;
mod error;
pub mod generators;
pub mod interface;
pub mod models;

// Legacy compatibility (deprecated)
#[deprecated(since = "0.2.0", note = "Use interface::config instead")]
pub mod cli {
    pub use crate::interface::config::*;
    pub use crate::interface::generate_from_config;
}

pub use error::{Error, Result};
pub use models::*;

// Convenience re-exports for common use cases
pub use interface::config::GenerateConfig;
pub use interface::generate_from_config;
pub use interface::output::{Logger, ProgressReporter};

// Build system integration
pub use build::BuildSystem;

// Tauri plugin initialization
use tauri::{
    plugin::{Builder, TauriPlugin},
    Runtime,
};

/// Initializes the typegen plugin for Tauri.
///
/// This allows the plugin's commands (ping, analyze_commands, generate_models)
/// to be invoked from the frontend via the Tauri IPC.
///
/// # Example
///
/// ```rust,ignore
/// tauri::Builder::default()
///     .plugin(tauri_typegen::init())
///     .run(tauri::generate_context!())
///     .expect("error while running tauri application");
/// ```
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("typegen")
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::analyze_commands,
            commands::generate_models
        ])
        .build()
}
