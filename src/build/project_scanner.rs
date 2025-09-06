use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScanError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid project structure: {0}")]
    InvalidProject(String),
}

#[derive(Debug, Clone)]
pub struct ProjectInfo {
    pub root_path: PathBuf,
    pub src_tauri_path: PathBuf,
    pub tauri_config_path: Option<PathBuf>,
}

pub struct ProjectScanner {
    current_dir: PathBuf,
}

impl ProjectScanner {
    pub fn new() -> Self {
        Self {
            current_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    pub fn with_current_dir<P: AsRef<Path>>(path: P) -> Self {
        Self {
            current_dir: path.as_ref().to_path_buf(),
        }
    }

    /// Detect if we're in a Tauri project and gather project information
    pub fn detect_project(&self) -> Result<Option<ProjectInfo>, ScanError> {
        // Start from current directory and walk up the tree
        let mut current = self.current_dir.clone();

        loop {
            if let Some(project_info) = self.check_directory(&current)? {
                return Ok(Some(project_info));
            }

            // Move up one directory
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                // Reached filesystem root, no Tauri project found
                break;
            }
        }

        Ok(None)
    }

    /// Check if a specific directory contains a Tauri project
    fn check_directory(&self, dir: &Path) -> Result<Option<ProjectInfo>, ScanError> {
        // Check for tauri.conf.json (v2) or tauri.conf.js
        let tauri_config_json = dir.join("tauri.conf.json");
        let tauri_config_js = dir.join("tauri.conf.js");
        let src_tauri = dir.join("src-tauri");

        let tauri_config_path = if tauri_config_json.exists() {
            Some(tauri_config_json)
        } else if tauri_config_js.exists() {
            Some(tauri_config_js)
        } else {
            None
        };

        // A Tauri project should have either a config file or a src-tauri directory
        if tauri_config_path.is_some() || src_tauri.exists() {
            // Determine the actual source path
            let src_tauri_path = if src_tauri.exists() && src_tauri.is_dir() {
                src_tauri
            } else if let Some(ref config_path) = tauri_config_path {
                // Try to read the config to find the source path
                if let Ok(source_dir) = self.read_source_dir_from_config(config_path) {
                    dir.join(source_dir)
                } else {
                    // Default fallback
                    src_tauri
                }
            } else {
                src_tauri
            };

            return Ok(Some(ProjectInfo {
                root_path: dir.to_path_buf(),
                src_tauri_path,
                tauri_config_path,
            }));
        }

        Ok(None)
    }

    /// Try to read the source directory from the Tauri configuration
    fn read_source_dir_from_config(&self, config_path: &Path) -> Result<String, ScanError> {
        let content = fs::read_to_string(config_path)?;

        // Handle JSON config
        if config_path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(build) = config.get("build") {
                    if let Some(dev_path) = build.get("devPath").and_then(|v| v.as_str()) {
                        return Ok(dev_path.to_string());
                    }
                }
            }
        }

        // Default fallback
        Ok("src-tauri".to_string())
    }

    /// Discover all Rust source files in the project
    pub fn discover_rust_files(
        &self,
        project_info: &ProjectInfo,
    ) -> Result<Vec<PathBuf>, ScanError> {
        let mut rust_files = Vec::new();
        self.walk_directory(&project_info.src_tauri_path, &mut rust_files)?;
        Ok(rust_files)
    }

    /// Recursively walk a directory to find Rust files
    fn walk_directory(&self, dir: &Path, rust_files: &mut Vec<PathBuf>) -> Result<(), ScanError> {
        if !dir.exists() || !dir.is_dir() {
            return Ok(());
        }

        let entries = fs::read_dir(dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Skip common directories that shouldn't contain source
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if !["target", "node_modules", ".git", "dist"].contains(&dir_name) {
                    self.walk_directory(&path, rust_files)?;
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                rust_files.push(path);
            }
        }

        Ok(())
    }

    /// Check if the project has package.json (indicates frontend project)
    pub fn has_frontend(&self, project_info: &ProjectInfo) -> bool {
        let package_json = project_info.root_path.join("package.json");
        package_json.exists()
    }

    /// Get the recommended output path based on project structure
    pub fn get_recommended_output_path(&self, project_info: &ProjectInfo) -> String {
        if self.has_frontend(project_info) {
            // Frontend project, use src/generated
            "./src/generated".to_string()
        } else {
            // Backend-only project, use generated in root
            "./generated".to_string()
        }
    }
}

impl Default for ProjectScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_tauri_project_with_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("tauri.conf.json");
        fs::write(&config_path, r#"{"build": {"devPath": "./src"}}"#).unwrap();

        let scanner = ProjectScanner::with_current_dir(temp_dir.path());
        let project_info = scanner.detect_project().unwrap().unwrap();

        assert_eq!(project_info.root_path, temp_dir.path());
        assert!(project_info.tauri_config_path.is_some());
    }

    #[test]
    fn test_detect_tauri_project_with_src_tauri() {
        let temp_dir = TempDir::new().unwrap();
        let src_tauri = temp_dir.path().join("src-tauri");
        fs::create_dir(&src_tauri).unwrap();

        let scanner = ProjectScanner::with_current_dir(temp_dir.path());
        let project_info = scanner.detect_project().unwrap().unwrap();

        assert_eq!(project_info.root_path, temp_dir.path());
        assert_eq!(project_info.src_tauri_path, src_tauri);
    }

    #[test]
    fn test_no_tauri_project() {
        let temp_dir = TempDir::new().unwrap();

        let scanner = ProjectScanner::with_current_dir(temp_dir.path());
        let project_info = scanner.detect_project().unwrap();

        assert!(project_info.is_none());
    }

    #[test]
    fn test_discover_rust_files() {
        let temp_dir = TempDir::new().unwrap();
        let src_tauri = temp_dir.path().join("src-tauri");
        fs::create_dir(&src_tauri).unwrap();

        let main_rs = src_tauri.join("main.rs");
        let lib_rs = src_tauri.join("lib.rs");
        fs::write(&main_rs, "// main").unwrap();
        fs::write(&lib_rs, "// lib").unwrap();

        let project_info = ProjectInfo {
            root_path: temp_dir.path().to_path_buf(),
            src_tauri_path: src_tauri,
            tauri_config_path: None,
        };

        let scanner = ProjectScanner::new();
        let rust_files = scanner.discover_rust_files(&project_info).unwrap();

        assert_eq!(rust_files.len(), 2);
        assert!(rust_files.contains(&main_rs));
        assert!(rust_files.contains(&lib_rs));
    }

    #[test]
    fn test_has_frontend_detection() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");
        fs::write(&package_json, r#"{"name": "test"}"#).unwrap();

        let project_info = ProjectInfo {
            root_path: temp_dir.path().to_path_buf(),
            src_tauri_path: temp_dir.path().join("src-tauri"),
            tauri_config_path: None,
        };

        let scanner = ProjectScanner::new();
        assert!(scanner.has_frontend(&project_info));
    }

    #[test]
    fn test_recommended_output_path() {
        let temp_dir = TempDir::new().unwrap();

        // Test without frontend
        let project_info = ProjectInfo {
            root_path: temp_dir.path().to_path_buf(),
            src_tauri_path: temp_dir.path().join("src-tauri"),
            tauri_config_path: None,
        };

        let scanner = ProjectScanner::new();
        assert_eq!(
            scanner.get_recommended_output_path(&project_info),
            "./generated"
        );

        // Test with frontend
        let package_json = temp_dir.path().join("package.json");
        fs::write(&package_json, r#"{"name": "test"}"#).unwrap();

        assert_eq!(
            scanner.get_recommended_output_path(&project_info),
            "./src/generated"
        );
    }
}
