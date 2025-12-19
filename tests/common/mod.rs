#![allow(dead_code)]
/// Common test utilities and helpers
use std::collections::HashMap;
use std::fs;
use tauri_typegen::analysis::CommandAnalyzer;
use tauri_typegen::generators::create_generator;
use tauri_typegen::models::{CommandInfo, StructInfo};
use tauri_typegen::GenerateConfig;
use tempfile::TempDir;

/// Create a test project with Rust source files
pub struct TestProject {
    pub temp_dir: TempDir,
}

impl TestProject {
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().unwrap(),
        }
    }

    /// Write a Rust file to the test project
    pub fn write_file(&self, name: &str, content: &str) -> &Self {
        let file_path = self.temp_dir.path().join(name);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file_path, content).unwrap();
        self
    }

    /// Get the project path as a string
    pub fn path(&self) -> &str {
        self.temp_dir.path().to_str().unwrap()
    }

    /// Analyze the project and return discovered commands
    pub fn analyze(&self) -> (CommandAnalyzer, Vec<CommandInfo>) {
        let mut analyzer = CommandAnalyzer::new();
        let commands = analyzer.analyze_project(self.path()).unwrap();
        (analyzer, commands)
    }
}

/// Generate TypeScript bindings for a test project
pub struct TestGenerator {
    pub output_dir: TempDir,
}

impl TestGenerator {
    pub fn new() -> Self {
        Self {
            output_dir: TempDir::new().unwrap(),
        }
    }

    /// Generate bindings with the specified validation library
    pub fn generate(
        &self,
        commands: &[CommandInfo],
        discovered_structs: &HashMap<String, StructInfo>,
        analyzer: &CommandAnalyzer,
        validation: Option<&str>,
        config: Option<&GenerateConfig>,
    ) -> Vec<String> {
        let mut generator = create_generator(validation.map(|s| s.to_string()));
        let final_config = config.cloned().unwrap_or_default();

        generator
            .generate_models(
                commands,
                discovered_structs,
                self.output_path(),
                analyzer,
                &final_config,
            )
            .unwrap()
    }

    /// Get the output path as a string
    pub fn output_path(&self) -> &str {
        self.output_dir.path().to_str().unwrap()
    }

    /// Read a generated file
    pub fn read_file(&self, filename: &str) -> String {
        let path = self.output_dir.path().join(filename);
        fs::read_to_string(path).unwrap()
    }

    /// Check if a file exists
    pub fn file_exists(&self, filename: &str) -> bool {
        self.output_dir.path().join(filename).exists()
    }
}

/// Assert that generated content contains expected string
#[macro_export]
macro_rules! assert_generated_contains {
    ($content:expr, $expected:expr) => {
        assert!(
            $content.contains($expected),
            "Expected generated content to contain:\n{}\n\nBut got:\n{}",
            $expected,
            $content
        );
    };
    ($content:expr, $expected:expr, $($arg:tt)*) => {
        assert!(
            $content.contains($expected),
            $($arg)*
        );
    };
}

/// Assert that generated content does NOT contain string
#[macro_export]
macro_rules! assert_generated_not_contains {
    ($content:expr, $unexpected:expr) => {
        assert!(
            !$content.contains($unexpected),
            "Expected generated content NOT to contain:\n{}\n\nBut got:\n{}",
            $unexpected,
            $content
        );
    };
}
