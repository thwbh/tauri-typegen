pub mod file_writer;
pub mod template_context;
pub mod templates;
pub mod type_visitor;

use crate::analysis::CommandAnalyzer;
use crate::models::{CommandInfo, StructInfo};
use std::collections::HashMap;

/// Common trait for all generators
pub trait BaseBindingsGenerator {
    /// Generate models from Rust commands and structs
    fn generate_models(
        &mut self,
        commands: &[CommandInfo],
        discovered_structs: &HashMap<String, StructInfo>,
        output_path: &str,
        analyzer: &CommandAnalyzer,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>>;
}
