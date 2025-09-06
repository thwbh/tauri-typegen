use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid validation library: {0}. Use 'zod' or 'none'")]
    InvalidValidationLibrary(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenerateConfig {
    /// Path to the Tauri project source directory
    #[serde(default = "default_project_path")]
    pub project_path: String,
    
    /// Output path for generated TypeScript files
    #[serde(default = "default_output_path")]
    pub output_path: String,
    
    /// Validation library to use ('zod' or 'none')
    #[serde(default = "default_validation_library")]
    pub validation_library: String,
    
    /// Enable verbose output
    #[serde(default)]
    pub verbose: Option<bool>,
    
    /// Generate dependency graph visualization
    #[serde(default)]
    pub visualize_deps: Option<bool>,
    
    /// Include private struct fields in generation
    #[serde(default)]
    pub include_private: Option<bool>,
    
    /// Custom type mappings
    #[serde(default)]
    pub type_mappings: Option<std::collections::HashMap<String, String>>,
    
    /// File patterns to exclude from analysis
    #[serde(default)]
    pub exclude_patterns: Option<Vec<String>>,
    
    /// File patterns to include in analysis (overrides excludes)
    #[serde(default)]
    pub include_patterns: Option<Vec<String>>,
}

fn default_project_path() -> String {
    "./src-tauri".to_string()
}

fn default_output_path() -> String {
    "./src/generated".to_string()
}

fn default_validation_library() -> String {
    "zod".to_string()
}

impl Default for GenerateConfig {
    fn default() -> Self {
        Self {
            project_path: default_project_path(),
            output_path: default_output_path(),
            validation_library: default_validation_library(),
            verbose: Some(false),
            visualize_deps: Some(false),
            include_private: Some(false),
            type_mappings: None,
            exclude_patterns: None,
            include_patterns: None,
        }
    }
}

impl GenerateConfig {
    /// Create a new configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Load configuration from Tauri configuration file
    pub fn from_tauri_config<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let tauri_config: serde_json::Value = serde_json::from_str(&content)?;

        let mut config = Self::default();

        // Look for typegen plugin configuration
        if let Some(plugins) = tauri_config.get("plugins") {
            if let Some(typegen) = plugins.get("typegen") {
                if let Some(project_path) = typegen.get("projectPath").and_then(|v| v.as_str()) {
                    config.project_path = project_path.to_string();
                }
                if let Some(output_path) = typegen.get("outputPath").and_then(|v| v.as_str()) {
                    config.output_path = output_path.to_string();
                }
                if let Some(validation) = typegen.get("validationLibrary").and_then(|v| v.as_str()) {
                    config.validation_library = validation.to_string();
                }
                if let Some(verbose) = typegen.get("verbose").and_then(|v| v.as_bool()) {
                    config.verbose = Some(verbose);
                }
                if let Some(visualize_deps) = typegen.get("visualizeDeps").and_then(|v| v.as_bool()) {
                    config.visualize_deps = Some(visualize_deps);
                }
                if let Some(include_private) = typegen.get("includePrivate").and_then(|v| v.as_bool()) {
                    config.include_private = Some(include_private);
                }
                if let Some(type_mappings) = typegen.get("typeMappings") {
                    if let Ok(mappings) = serde_json::from_value::<std::collections::HashMap<String, String>>(type_mappings.clone()) {
                        config.type_mappings = Some(mappings);
                    }
                }
                if let Some(exclude_patterns) = typegen.get("excludePatterns") {
                    if let Ok(patterns) = serde_json::from_value::<Vec<String>>(exclude_patterns.clone()) {
                        config.exclude_patterns = Some(patterns);
                    }
                }
                if let Some(include_patterns) = typegen.get("includePatterns") {
                    if let Ok(patterns) = serde_json::from_value::<Vec<String>>(include_patterns.clone()) {
                        config.include_patterns = Some(patterns);
                    }
                }
            }
        }

        config.validate()?;
        Ok(config)
    }

    /// Save configuration to a file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Save configuration to Tauri configuration file
    pub fn save_to_tauri_config<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        
        // Read existing tauri.conf.json or create new one
        let mut tauri_config = if path.as_ref().exists() {
            let content = fs::read_to_string(&path)?;
            serde_json::from_str::<serde_json::Value>(&content)?
        } else {
            serde_json::json!({
                "build": {},
                "package": {},
                "plugins": {}
            })
        };

        // Create typegen plugin configuration
        let typegen_config = serde_json::json!({
            "projectPath": self.project_path,
            "outputPath": self.output_path,
            "validationLibrary": self.validation_library,
            "verbose": self.verbose.unwrap_or(false),
            "visualizeDeps": self.visualize_deps.unwrap_or(false),
            "includePrivate": self.include_private.unwrap_or(false),
            "typeMappings": self.type_mappings,
            "excludePatterns": self.exclude_patterns,
            "includePatterns": self.include_patterns,
        });

        // Insert into plugins section
        if let Some(plugins) = tauri_config.get_mut("plugins") {
            if let Some(plugins_obj) = plugins.as_object_mut() {
                plugins_obj.insert("typegen".to_string(), typegen_config);
            }
        }

        let content = serde_json::to_string_pretty(&tauri_config)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate validation library
        match self.validation_library.as_str() {
            "zod" | "none" => {}
            _ => return Err(ConfigError::InvalidValidationLibrary(self.validation_library.clone())),
        }

        // Validate paths exist
        let project_path = Path::new(&self.project_path);
        if !project_path.exists() {
            return Err(ConfigError::InvalidConfig(format!(
                "Project path does not exist: {}",
                self.project_path
            )));
        }

        Ok(())
    }

    /// Merge with another configuration, with other taking precedence
    pub fn merge(&mut self, other: &GenerateConfig) {
        if other.project_path != default_project_path() {
            self.project_path = other.project_path.clone();
        }
        if other.output_path != default_output_path() {
            self.output_path = other.output_path.clone();
        }
        if other.validation_library != default_validation_library() {
            self.validation_library = other.validation_library.clone();
        }
        if other.verbose.is_some() {
            self.verbose = other.verbose;
        }
        if other.visualize_deps.is_some() {
            self.visualize_deps = other.visualize_deps;
        }
        if other.include_private.is_some() {
            self.include_private = other.include_private;
        }
        if other.type_mappings.is_some() {
            self.type_mappings = other.type_mappings.clone();
        }
        if other.exclude_patterns.is_some() {
            self.exclude_patterns = other.exclude_patterns.clone();
        }
        if other.include_patterns.is_some() {
            self.include_patterns = other.include_patterns.clone();
        }
    }

    /// Get effective verbose setting
    pub fn is_verbose(&self) -> bool {
        self.verbose.unwrap_or(false)
    }

    /// Get effective visualize_deps setting
    pub fn should_visualize_deps(&self) -> bool {
        self.visualize_deps.unwrap_or(false)
    }

    /// Get effective include_private setting
    pub fn should_include_private(&self) -> bool {
        self.include_private.unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = GenerateConfig::default();
        assert_eq!(config.project_path, "./src-tauri");
        assert_eq!(config.output_path, "./src/generated");
        assert_eq!(config.validation_library, "zod");
        assert!(!config.is_verbose());
        assert!(!config.should_visualize_deps());
        assert!(!config.should_include_private());
    }

    #[test]
    fn test_config_validation() {
        let mut config = GenerateConfig::default();
        config.validation_library = "invalid".to_string();
        
        let result = config.validate();
        assert!(result.is_err());
        if let Err(ConfigError::InvalidValidationLibrary(lib)) = result {
            assert_eq!(lib, "invalid");
        } else {
            panic!("Expected InvalidValidationLibrary error");
        }
    }

    #[test]
    fn test_config_merge() {
        let mut base = GenerateConfig::default();
        let override_config = GenerateConfig {
            output_path: "./custom".to_string(),
            verbose: Some(true),
            ..Default::default()
        };

        base.merge(&override_config);
        assert_eq!(base.output_path, "./custom");
        assert!(base.is_verbose());
        assert_eq!(base.validation_library, "zod"); // Should remain default
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let project_path = temp_dir.path().join("src-tauri");
        std::fs::create_dir_all(&project_path).unwrap();
        
        let config = GenerateConfig {
            project_path: project_path.to_string_lossy().to_string(),
            output_path: "./test".to_string(),
            verbose: Some(true),
            ..Default::default()
        };

        let temp_file = NamedTempFile::new().unwrap();
        config.save_to_file(temp_file.path()).unwrap();
        
        let loaded_config = GenerateConfig::from_file(temp_file.path()).unwrap();
        assert_eq!(loaded_config.output_path, "./test");
        assert!(loaded_config.is_verbose());
    }
}