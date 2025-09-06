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
