use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OutputError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid output path: {0}")]
    InvalidPath(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

pub struct OutputManager {
    output_dir: PathBuf,
    managed_files: HashSet<String>,
    backup_dir: Option<PathBuf>,
}

impl OutputManager {
    pub fn new<P: AsRef<Path>>(output_dir: P) -> Self {
        Self {
            output_dir: output_dir.as_ref().to_path_buf(),
            managed_files: HashSet::new(),
            backup_dir: None,
        }
    }

    pub fn with_backup<P: AsRef<Path>>(output_dir: P, backup_dir: Option<P>) -> Self {
        Self {
            output_dir: output_dir.as_ref().to_path_buf(),
            managed_files: HashSet::new(),
            backup_dir: backup_dir.map(|p| p.as_ref().to_path_buf()),
        }
    }

    /// Ensure the output directory exists and is writable
    pub fn prepare_output_directory(&self) -> Result<(), OutputError> {
        if !self.output_dir.exists() {
            fs::create_dir_all(&self.output_dir).map_err(|e| {
                OutputError::PermissionDenied(format!(
                    "Cannot create output directory {}: {}",
                    self.output_dir.display(),
                    e
                ))
            })?;
        }

        // Test write permissions by creating a temporary file
        let test_file = self.output_dir.join(".write_test");
        fs::write(&test_file, "test").map_err(|e| {
            OutputError::PermissionDenied(format!(
                "Cannot write to output directory {}: {}",
                self.output_dir.display(),
                e
            ))
        })?;
        fs::remove_file(&test_file).ok(); // Ignore errors on cleanup

        Ok(())
    }

    /// Register a file as managed by this generator
    pub fn register_managed_file(&mut self, filename: &str) {
        self.managed_files.insert(filename.to_string());
    }

    /// Clean up old generated files that are no longer needed
    pub fn cleanup_old_files(&self, current_files: &[String]) -> Result<Vec<String>, OutputError> {
        let mut cleaned_files = Vec::new();

        if !self.output_dir.exists() {
            return Ok(cleaned_files);
        }

        let current_set: HashSet<String> = current_files.iter().cloned().collect();

        // Read the directory and find files that look like they were generated
        let entries = fs::read_dir(&self.output_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    // Only clean up files that look like generated TypeScript files
                    if self.is_generated_file(filename) && !current_set.contains(filename) {
                        self.backup_and_remove_file(&path)?;
                        cleaned_files.push(filename.to_string());
                    }
                }
            }
        }

        Ok(cleaned_files)
    }

    /// Check if a file appears to be a generated file based on naming patterns
    fn is_generated_file(&self, filename: &str) -> bool {
        // Check for common generated file patterns
        let generated_patterns = [
            "types.ts",
            "types.d.ts",
            "commands.ts",
            "commands.d.ts",
            "schemas.ts",
            "schemas.d.ts",
            "index.ts",
            "index.d.ts",
            "models.ts",
            "models.d.ts",
            "bindings.ts",
            "bindings.d.ts",
        ];

        generated_patterns.contains(&filename)
            || filename.starts_with("generated_")
            || filename.contains("_generated")
            || self.managed_files.contains(filename)
    }

    /// Backup a file before removing it
    fn backup_and_remove_file(&self, file_path: &Path) -> Result<(), OutputError> {
        if let Some(backup_dir) = &self.backup_dir {
            if !backup_dir.exists() {
                fs::create_dir_all(backup_dir)?;
            }

            if let Some(filename) = file_path.file_name() {
                let backup_path = backup_dir.join(format!(
                    "{}.backup.{}",
                    filename.to_string_lossy(),
                    chrono::Utc::now().timestamp()
                ));
                fs::copy(file_path, backup_path)?;
            }
        }

        fs::remove_file(file_path)?;
        Ok(())
    }

    /// Write a file to the output directory with proper error handling
    pub fn write_file(&self, filename: &str, content: &str) -> Result<PathBuf, OutputError> {
        let file_path = self.output_dir.join(filename);

        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        // Write with atomic operation (write to temp file first, then rename)
        let temp_path = file_path.with_extension("tmp");
        fs::write(&temp_path, content)?;
        fs::rename(&temp_path, &file_path)?;

        Ok(file_path)
    }

    /// Verify that all expected files were generated
    pub fn verify_output(&self, expected_files: &[String]) -> Result<Vec<String>, OutputError> {
        let mut missing_files = Vec::new();

        for expected in expected_files {
            let file_path = self.output_dir.join(expected);
            if !file_path.exists() {
                missing_files.push(expected.clone());
            }
        }

        Ok(missing_files)
    }

    /// Get metadata about generated files
    pub fn get_generation_metadata(&self) -> Result<GenerationMetadata, OutputError> {
        let mut metadata = GenerationMetadata {
            output_directory: self.output_dir.clone(),
            generated_at: chrono::Utc::now(),
            files: Vec::new(),
            total_size: 0,
        };

        if !self.output_dir.exists() {
            return Ok(metadata);
        }

        let entries = fs::read_dir(&self.output_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Ok(metadata_entry) = entry.metadata() {
                    let size = metadata_entry.len();
                    metadata.total_size += size;

                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        metadata.files.push(FileMetadata {
                            name: filename.to_string(),
                            path: path.clone(),
                            size,
                            modified: metadata_entry.modified().ok(),
                        });
                    }
                }
            }
        }

        Ok(metadata)
    }

    /// Finalize the generation process
    pub fn finalize_generation(&mut self, generated_files: &[String]) -> Result<(), OutputError> {
        self.prepare_output_directory()?;

        // Register all generated files as managed
        for file in generated_files {
            self.register_managed_file(file);
        }

        // Clean up old files
        let cleaned = self.cleanup_old_files(generated_files)?;
        if !cleaned.is_empty() {
            eprintln!("Cleaned up {} old generated files", cleaned.len());
        }

        // Verify all expected files exist
        let missing = self.verify_output(generated_files)?;
        if !missing.is_empty() {
            return Err(OutputError::InvalidPath(format!(
                "Missing generated files: {}",
                missing.join(", ")
            )));
        }

        Ok(())
    }

    /// Create a summary report of the generation process
    pub fn create_summary_report(&self) -> Result<String, OutputError> {
        let metadata = self.get_generation_metadata()?;

        let mut report = String::new();
        report.push_str("# TypeScript Generation Summary\n\n");
        report.push_str(&format!(
            "Generated at: {}\n",
            metadata.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        report.push_str(&format!(
            "Output directory: {}\n",
            metadata.output_directory.display()
        ));
        report.push_str(&format!("Total files: {}\n", metadata.files.len()));
        report.push_str(&format!("Total size: {} bytes\n\n", metadata.total_size));

        report.push_str("## Generated Files\n\n");
        for file in &metadata.files {
            report.push_str(&format!("- **{}** ({} bytes)\n", file.name, file.size));
        }

        Ok(report)
    }
}

#[derive(Debug)]
pub struct GenerationMetadata {
    pub output_directory: PathBuf,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub files: Vec<FileMetadata>,
    pub total_size: u64,
}

#[derive(Debug)]
pub struct FileMetadata {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: Option<std::time::SystemTime>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_prepare_output_directory() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output");

        let manager = OutputManager::new(&output_path);
        manager.prepare_output_directory().unwrap();

        assert!(output_path.exists());
        assert!(output_path.is_dir());
    }

    #[test]
    fn test_write_file() {
        let temp_dir = TempDir::new().unwrap();
        let manager = OutputManager::new(temp_dir.path());

        manager.prepare_output_directory().unwrap();

        let content = "export interface Test { name: string; }";
        let file_path = manager.write_file("test.ts", content).unwrap();

        assert!(file_path.exists());
        let written_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(written_content, content);
    }

    #[test]
    fn test_is_generated_file() {
        let temp_dir = TempDir::new().unwrap();
        let manager = OutputManager::new(temp_dir.path());

        // Test standard patterns
        assert!(manager.is_generated_file("types.ts"));
        assert!(manager.is_generated_file("commands.ts"));
        assert!(manager.is_generated_file("schemas.ts"));
        assert!(manager.is_generated_file("index.ts"));

        // Test prefix/suffix patterns
        assert!(manager.is_generated_file("generated_models.ts"));
        assert!(manager.is_generated_file("api_generated.ts"));

        // Test non-generated files
        assert!(!manager.is_generated_file("user_code.ts"));
        assert!(!manager.is_generated_file("main.ts"));
        assert!(!manager.is_generated_file("app.vue"));
    }

    #[test]
    fn test_cleanup_old_files() {
        let temp_dir = TempDir::new().unwrap();
        let manager = OutputManager::new(temp_dir.path());

        manager.prepare_output_directory().unwrap();

        // Create some files
        fs::write(temp_dir.path().join("types.ts"), "old content").unwrap();
        fs::write(temp_dir.path().join("commands.ts"), "old content").unwrap();
        fs::write(temp_dir.path().join("user_file.ts"), "user content").unwrap();

        // Current generation only includes types.ts
        let current_files = vec!["types.ts".to_string()];

        let cleaned = manager.cleanup_old_files(&current_files).unwrap();

        // Should clean up commands.ts but not user_file.ts or types.ts
        assert_eq!(cleaned.len(), 1);
        assert!(cleaned.contains(&"commands.ts".to_string()));

        assert!(temp_dir.path().join("types.ts").exists());
        assert!(!temp_dir.path().join("commands.ts").exists());
        assert!(temp_dir.path().join("user_file.ts").exists());
    }

    #[test]
    fn test_verify_output() {
        let temp_dir = TempDir::new().unwrap();
        let manager = OutputManager::new(temp_dir.path());

        manager.prepare_output_directory().unwrap();

        // Create one expected file
        fs::write(temp_dir.path().join("types.ts"), "content").unwrap();

        let expected = vec!["types.ts".to_string(), "commands.ts".to_string()];
        let missing = manager.verify_output(&expected).unwrap();

        assert_eq!(missing.len(), 1);
        assert!(missing.contains(&"commands.ts".to_string()));
    }

    #[test]
    fn test_generation_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let manager = OutputManager::new(temp_dir.path());

        manager.prepare_output_directory().unwrap();

        // Create some test files
        fs::write(temp_dir.path().join("types.ts"), "interface Test {}").unwrap();
        fs::write(
            temp_dir.path().join("commands.ts"),
            "export function test() {}",
        )
        .unwrap();

        let metadata = manager.get_generation_metadata().unwrap();

        assert_eq!(metadata.files.len(), 2);
        assert!(metadata.total_size > 0);
        assert_eq!(metadata.output_directory, temp_dir.path());

        // Check individual files
        let types_file = metadata
            .files
            .iter()
            .find(|f| f.name == "types.ts")
            .unwrap();
        assert_eq!(types_file.size, 17); // Length of "interface Test {}"
    }
}
