//! # Tauri TypeGen
//!
//! Automatically generate TypeScript bindings from Tauri commands.
//!
//! This library scans Rust source code for `#[tauri::command]` functions and generates
//! strongly-typed TypeScript interfaces, command functions, and optional Zod schemas
//! with runtime validation.
//!
//! ## Features
//!
//! - 🔍 **Automatic Discovery**: Scans Rust source for `#[tauri::command]` functions
//! - 📝 **TypeScript Generation**: Creates TypeScript interfaces for command parameters and return types
//! - ✅ **Validation Support**: Optional Zod schema generation with runtime validation
//! - 🚀 **Command Bindings**: Strongly-typed frontend functions
//! - 📡 **Event Support**: Discovers and types `app.emit()` events
//! - 📞 **Channel Support**: Types for streaming `Channel<T>` parameters
//! - 🏷️ **Serde Support**: Respects `#[serde(rename)]` and `#[serde(rename_all)]` attributes
//!
//! ## Quick Start
//!
//! ### As a CLI Tool
//!
//! ```bash
//! # Install globally
//! cargo install tauri-typegen
//!
//! # Generate TypeScript bindings
//! cargo tauri-typegen generate
//! ```
//!
//! ### As a Build Dependency
//!
//! Add to your `src-tauri/build.rs`:
//!
//! ```rust,ignore
//! fn main() {
//!     // Generate TypeScript bindings before build
//!     tauri_typegen::BuildSystem::generate_at_build_time()
//!         .expect("Failed to generate TypeScript bindings");
//!
//!     tauri_build::build()
//! }
//! ```
//!
//! ### Programmatic Usage
//!
//! ```rust,no_run
//! use tauri_typegen::{GenerateConfig, generate_from_config};
//!
//! let config = GenerateConfig {
//!     project_path: "./src-tauri".to_string(),
//!     output_path: "./src/generated".to_string(),
//!     validation_library: "zod".to_string(),
//!     verbose: Some(true),
//!     ..Default::default()
//! };
//!
//! let files = generate_from_config(&config)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Example
//!
//! Given this Rust code:
//!
//! ```rust,ignore
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize)]
//! pub struct User {
//!     pub id: i32,
//!     pub name: String,
//! }
//!
//! #[tauri::command]
//! pub async fn get_user(id: i32) -> Result<User, String> {
//!     // Implementation
//! }
//! ```
//!
//! Generates this TypeScript:
//!
//! ```typescript
//! export interface User {
//!   id: number;
//!   name: string;
//! }
//!
//! export async function getUser(params: { id: number }): Promise<User> {
//!   return invoke('get_user', params);
//! }
//! ```
//!
//! ## Configuration
//!
//! Configure via `tauri.conf.json`:
//!
//! ```json
//! {
//!   "plugins": {
//!     "tauri-typegen": {
//!       "project_path": ".",
//!       "output_path": "../src/generated",
//!       "validation_library": "zod",
//!       "type_mappings": {
//!         "DateTime<Utc>": "string",
//!         "PathBuf": "string"
//!       }
//!     }
//!   }
//! }
//! ```

// Core library modules for the CLI tool
pub mod analysis;
pub mod build;
// pub mod commands; // Removed: plugin commands are not used
mod error;
pub mod generators;
pub mod interface;
pub mod models;

pub use error::{Error, Result};
pub use models::*;

// Convenience re-exports for common use cases
pub use interface::config::GenerateConfig;
pub use interface::generate_from_config;
pub use interface::output::{Logger, ProgressReporter};

// Build system integration
pub use build::BuildSystem;
