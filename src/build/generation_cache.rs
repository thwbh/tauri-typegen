use crate::interface::config::GenerateConfig;
use crate::models::{CommandInfo, StructInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Hash generation error: {0}")]
    HashError(String),
}

/// Cache file name stored in the output directory
const CACHE_FILE_NAME: &str = ".typecache";

/// Represents the cached state of a generation run
#[derive(Debug, Serialize, Deserialize)]
pub struct GenerationCache {
    /// Version of the cache format for future compatibility
    version: u32,
    /// Hash of all discovered commands
    commands_hash: String,
    /// Hash of all discovered structs
    structs_hash: String,
    /// Hash of configuration settings that affect output
    config_hash: String,
    /// Combined hash for quick comparison
    combined_hash: String,
}

impl GenerationCache {
    const CURRENT_VERSION: u32 = 1;

    /// Create a new cache from current generation state
    pub fn new(
        commands: &[CommandInfo],
        structs: &HashMap<String, StructInfo>,
        config: &GenerateConfig,
    ) -> Result<Self, CacheError> {
        let commands_hash = Self::hash_commands(commands)?;
        let structs_hash = Self::hash_structs(structs)?;
        let config_hash = Self::hash_config(config)?;
        let combined_hash = Self::combine_hashes(&commands_hash, &structs_hash, &config_hash)?;

        Ok(Self {
            version: Self::CURRENT_VERSION,
            commands_hash,
            structs_hash,
            config_hash,
            combined_hash,
        })
    }

    /// Load cache from file
    pub fn load<P: AsRef<Path>>(output_dir: P) -> Result<Self, CacheError> {
        let cache_path = Self::cache_path(output_dir);
        let content = fs::read_to_string(cache_path)?;
        let cache: Self = serde_json::from_str(&content)?;
        Ok(cache)
    }

    /// Save cache to file
    pub fn save<P: AsRef<Path>>(&self, output_dir: P) -> Result<(), CacheError> {
        let cache_path = Self::cache_path(output_dir);

        // Ensure output directory exists
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(cache_path, content)?;
        Ok(())
    }

    /// Check if generation is needed by comparing with previous cache
    pub fn needs_regeneration<P: AsRef<Path>>(
        output_dir: P,
        commands: &[CommandInfo],
        structs: &HashMap<String, StructInfo>,
        config: &GenerateConfig,
    ) -> Result<bool, CacheError> {
        // Try to load previous cache
        let previous_cache = match Self::load(&output_dir) {
            Ok(cache) => cache,
            Err(_) => {
                // No cache file or error reading it - needs regeneration
                return Ok(true);
            }
        };

        // Check version compatibility
        if previous_cache.version != Self::CURRENT_VERSION {
            return Ok(true);
        }

        // Generate current cache
        let current_cache = Self::new(commands, structs, config)?;

        // Compare combined hashes
        Ok(previous_cache.combined_hash != current_cache.combined_hash)
    }

    /// Get the cache file path
    fn cache_path<P: AsRef<Path>>(output_dir: P) -> PathBuf {
        output_dir.as_ref().join(CACHE_FILE_NAME)
    }

    /// Generate a deterministic hash of commands
    fn hash_commands(commands: &[CommandInfo]) -> Result<String, CacheError> {
        // Create a serializable representation
        #[derive(Serialize)]
        struct CommandHashData<'a> {
            name: &'a str,
            file_path: &'a str,
            parameters: Vec<ParameterHashData<'a>>,
            return_type: &'a str,
            is_async: bool,
            channels: Vec<ChannelHashData<'a>>,
        }

        #[derive(Serialize)]
        struct ParameterHashData<'a> {
            name: &'a str,
            rust_type: &'a str,
            is_optional: bool,
        }

        #[derive(Serialize)]
        struct ChannelHashData<'a> {
            parameter_name: &'a str,
            message_type: &'a str,
        }

        let hash_data: Vec<CommandHashData> = commands
            .iter()
            .map(|cmd| CommandHashData {
                name: &cmd.name,
                file_path: &cmd.file_path,
                parameters: cmd
                    .parameters
                    .iter()
                    .map(|p| ParameterHashData {
                        name: &p.name,
                        rust_type: &p.rust_type,
                        is_optional: p.is_optional,
                    })
                    .collect(),
                return_type: &cmd.return_type,
                is_async: cmd.is_async,
                channels: cmd
                    .channels
                    .iter()
                    .map(|c| ChannelHashData {
                        parameter_name: &c.parameter_name,
                        message_type: &c.message_type,
                    })
                    .collect(),
            })
            .collect();

        let json = serde_json::to_string(&hash_data)?;
        Ok(Self::compute_hash(&json))
    }

    /// Generate a deterministic hash of structs
    fn hash_structs(structs: &HashMap<String, StructInfo>) -> Result<String, CacheError> {
        #[derive(Serialize)]
        struct StructHashData<'a> {
            name: &'a str,
            file_path: &'a str,
            is_enum: bool,
            fields: Vec<FieldHashData<'a>>,
        }

        #[derive(Serialize)]
        struct FieldHashData<'a> {
            name: &'a str,
            rust_type: &'a str,
            is_optional: bool,
            is_public: bool,
        }

        // Sort by name for deterministic ordering
        let mut sorted_structs: Vec<_> = structs.values().collect();
        sorted_structs.sort_by(|a, b| a.name.cmp(&b.name));

        let hash_data: Vec<StructHashData> = sorted_structs
            .iter()
            .map(|s| StructHashData {
                name: &s.name,
                file_path: &s.file_path,
                is_enum: s.is_enum,
                fields: s
                    .fields
                    .iter()
                    .map(|f| FieldHashData {
                        name: &f.name,
                        rust_type: &f.rust_type,
                        is_optional: f.is_optional,
                        is_public: f.is_public,
                    })
                    .collect(),
            })
            .collect();

        let json = serde_json::to_string(&hash_data)?;
        Ok(Self::compute_hash(&json))
    }

    /// Generate a hash of configuration settings that affect output
    fn hash_config(config: &GenerateConfig) -> Result<String, CacheError> {
        #[derive(Serialize)]
        struct ConfigHashData<'a> {
            validation_library: &'a str,
            include_private: bool,
            type_mappings: Option<&'a HashMap<String, String>>,
            default_parameter_case: &'a str,
            default_field_case: &'a str,
        }

        let hash_data = ConfigHashData {
            validation_library: &config.validation_library,
            include_private: config.include_private.unwrap_or(false),
            type_mappings: config.type_mappings.as_ref(),
            default_parameter_case: &config.default_parameter_case,
            default_field_case: &config.default_field_case,
        };

        let json = serde_json::to_string(&hash_data)?;
        Ok(Self::compute_hash(&json))
    }

    /// Combine multiple hashes into a single hash
    fn combine_hashes(commands: &str, structs: &str, config: &str) -> Result<String, CacheError> {
        let combined = format!("{}{}{}", commands, structs, config);
        Ok(Self::compute_hash(&combined))
    }

    /// Compute SHA-256 hash of a string
    fn compute_hash(data: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Test utilities already imported from parent module
    use tempfile::TempDir;

    fn create_test_config() -> GenerateConfig {
        GenerateConfig {
            project_path: "./src-tauri".to_string(),
            output_path: "./src/generated".to_string(),
            validation_library: "none".to_string(),
            verbose: Some(false),
            visualize_deps: Some(false),
            include_private: Some(false),
            type_mappings: None,
            exclude_patterns: None,
            include_patterns: None,
            default_parameter_case: "camelCase".to_string(),
            default_field_case: "snake_case".to_string(),
            force: Some(false),
        }
    }

    fn create_test_command(name: &str) -> CommandInfo {
        CommandInfo::new_for_test(name, "test.rs", 1, vec![], "String", false, vec![])
    }

    #[test]
    fn test_cache_creation() {
        let commands = vec![create_test_command("test_command")];
        let structs = HashMap::new();
        let config = create_test_config();

        let cache = GenerationCache::new(&commands, &structs, &config).unwrap();

        assert_eq!(cache.version, GenerationCache::CURRENT_VERSION);
        assert!(!cache.commands_hash.is_empty());
        assert!(!cache.structs_hash.is_empty());
        assert!(!cache.config_hash.is_empty());
        assert!(!cache.combined_hash.is_empty());
    }

    #[test]
    fn test_cache_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let commands = vec![create_test_command("test_command")];
        let structs = HashMap::new();
        let config = create_test_config();

        let cache = GenerationCache::new(&commands, &structs, &config).unwrap();
        cache.save(temp_dir.path()).unwrap();

        let loaded_cache = GenerationCache::load(temp_dir.path()).unwrap();

        assert_eq!(cache.combined_hash, loaded_cache.combined_hash);
        assert_eq!(cache.commands_hash, loaded_cache.commands_hash);
        assert_eq!(cache.structs_hash, loaded_cache.structs_hash);
    }

    #[test]
    fn test_needs_regeneration_no_cache() {
        let temp_dir = TempDir::new().unwrap();
        let commands = vec![create_test_command("test_command")];
        let structs = HashMap::new();
        let config = create_test_config();

        let needs_regen =
            GenerationCache::needs_regeneration(temp_dir.path(), &commands, &structs, &config)
                .unwrap();

        assert!(needs_regen);
    }

    #[test]
    fn test_needs_regeneration_same_state() {
        let temp_dir = TempDir::new().unwrap();
        let commands = vec![create_test_command("test_command")];
        let structs = HashMap::new();
        let config = create_test_config();

        // Save initial cache
        let cache = GenerationCache::new(&commands, &structs, &config).unwrap();
        cache.save(temp_dir.path()).unwrap();

        // Check if regeneration needed with same data
        let needs_regen =
            GenerationCache::needs_regeneration(temp_dir.path(), &commands, &structs, &config)
                .unwrap();

        assert!(!needs_regen);
    }

    #[test]
    fn test_needs_regeneration_command_changed() {
        let temp_dir = TempDir::new().unwrap();
        let commands = vec![create_test_command("test_command")];
        let structs = HashMap::new();
        let config = create_test_config();

        // Save initial cache
        let cache = GenerationCache::new(&commands, &structs, &config).unwrap();
        cache.save(temp_dir.path()).unwrap();

        // Change commands
        let new_commands = vec![create_test_command("different_command")];

        let needs_regen =
            GenerationCache::needs_regeneration(temp_dir.path(), &new_commands, &structs, &config)
                .unwrap();

        assert!(needs_regen);
    }

    #[test]
    fn test_needs_regeneration_config_changed() {
        let temp_dir = TempDir::new().unwrap();
        let commands = vec![create_test_command("test_command")];
        let structs = HashMap::new();
        let config = create_test_config();

        // Save initial cache
        let cache = GenerationCache::new(&commands, &structs, &config).unwrap();
        cache.save(temp_dir.path()).unwrap();

        // Change config
        let mut new_config = config;
        new_config.validation_library = "zod".to_string();

        let needs_regen =
            GenerationCache::needs_regeneration(temp_dir.path(), &commands, &structs, &new_config)
                .unwrap();

        assert!(needs_regen);
    }

    #[test]
    fn test_hash_determinism() {
        let commands = vec![create_test_command("test_command")];
        let structs = HashMap::new();
        let config = create_test_config();

        let cache1 = GenerationCache::new(&commands, &structs, &config).unwrap();
        let cache2 = GenerationCache::new(&commands, &structs, &config).unwrap();

        assert_eq!(cache1.combined_hash, cache2.combined_hash);
        assert_eq!(cache1.commands_hash, cache2.commands_hash);
        assert_eq!(cache1.structs_hash, cache2.structs_hash);
        assert_eq!(cache1.config_hash, cache2.config_hash);
    }

    #[test]
    fn test_needs_regeneration_version_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let commands = vec![create_test_command("test_command")];
        let structs = HashMap::new();
        let config = create_test_config();

        // Create a cache with a different version
        let old_cache_content = r#"{
            "version": 0,
            "commands_hash": "abc123",
            "structs_hash": "def456",
            "config_hash": "ghi789",
            "combined_hash": "xyz000"
        }"#;
        let cache_path = temp_dir.path().join(".typecache");
        std::fs::write(&cache_path, old_cache_content).unwrap();

        // Should need regeneration due to version mismatch
        let needs_regen =
            GenerationCache::needs_regeneration(temp_dir.path(), &commands, &structs, &config)
                .unwrap();

        assert!(needs_regen);
    }

    #[test]
    fn test_empty_commands_and_structs() {
        let commands: Vec<CommandInfo> = vec![];
        let structs: HashMap<String, crate::models::StructInfo> = HashMap::new();
        let config = create_test_config();

        let cache = GenerationCache::new(&commands, &structs, &config).unwrap();

        // Should still create valid hashes even with empty data
        assert!(!cache.commands_hash.is_empty());
        assert!(!cache.structs_hash.is_empty());
        assert!(!cache.combined_hash.is_empty());
    }

    #[test]
    fn test_struct_hash_order_independence() {
        use crate::models::{FieldInfo, StructInfo, TypeStructure};

        let config = create_test_config();
        let commands = vec![create_test_command("test_command")];

        // Create two structs
        let struct_a = StructInfo {
            name: "StructA".to_string(),
            fields: vec![FieldInfo {
                name: "field_a".to_string(),
                rust_type: "String".to_string(),
                is_optional: false,
                is_public: true,
                validator_attributes: None,
                serde_rename: None,
                type_structure: TypeStructure::Primitive("string".to_string()),
            }],
            file_path: "test.rs".to_string(),
            is_enum: false,
            serde_rename_all: None,
            serde_tag: None,
            enum_variants: None,
        };

        let struct_b = StructInfo {
            name: "StructB".to_string(),
            fields: vec![FieldInfo {
                name: "field_b".to_string(),
                rust_type: "i32".to_string(),
                is_optional: false,
                is_public: true,
                validator_attributes: None,
                serde_rename: None,
                type_structure: TypeStructure::Primitive("number".to_string()),
            }],
            file_path: "test.rs".to_string(),
            is_enum: false,
            serde_rename_all: None,
            serde_tag: None,
            enum_variants: None,
        };

        // Insert in order A, B
        let mut structs1 = HashMap::new();
        structs1.insert("StructA".to_string(), struct_a.clone());
        structs1.insert("StructB".to_string(), struct_b.clone());

        // Insert in order B, A (reverse)
        let mut structs2 = HashMap::new();
        structs2.insert("StructB".to_string(), struct_b);
        structs2.insert("StructA".to_string(), struct_a);

        let cache1 = GenerationCache::new(&commands, &structs1, &config).unwrap();
        let cache2 = GenerationCache::new(&commands, &structs2, &config).unwrap();

        // Hash should be the same regardless of insertion order
        assert_eq!(cache1.structs_hash, cache2.structs_hash);
        assert_eq!(cache1.combined_hash, cache2.combined_hash);
    }

    #[test]
    fn test_needs_regeneration_with_corrupted_cache_file() {
        let temp_dir = TempDir::new().unwrap();
        let commands = vec![create_test_command("test_command")];
        let structs = HashMap::new();
        let config = create_test_config();

        // Create a corrupted cache file
        let cache_path = temp_dir.path().join(".typecache");
        std::fs::write(&cache_path, "not valid json").unwrap();

        // Should need regeneration because cache is unreadable
        let needs_regen =
            GenerationCache::needs_regeneration(temp_dir.path(), &commands, &structs, &config)
                .unwrap();

        assert!(needs_regen);
    }

    #[test]
    fn test_cache_with_type_mappings_config() {
        let commands = vec![create_test_command("test_command")];
        let structs = HashMap::new();

        let mut config1 = create_test_config();
        let mut type_mappings = std::collections::HashMap::new();
        type_mappings.insert("CustomType".to_string(), "string".to_string());
        config1.type_mappings = Some(type_mappings);

        let config2 = create_test_config(); // No type mappings

        let cache1 = GenerationCache::new(&commands, &structs, &config1).unwrap();
        let cache2 = GenerationCache::new(&commands, &structs, &config2).unwrap();

        // Config hash should differ when type_mappings differ
        assert_ne!(cache1.config_hash, cache2.config_hash);
        assert_ne!(cache1.combined_hash, cache2.combined_hash);
    }

    #[test]
    fn test_cache_with_channels() {
        use crate::models::ChannelInfo;

        let structs = HashMap::new();
        let config = create_test_config();

        let channel = ChannelInfo::new_for_test("progress", "u32", "test_command", "test.rs", 1);

        let cmd_with_channel = CommandInfo::new_for_test(
            "test_command",
            "test.rs",
            1,
            vec![],
            "String",
            false,
            vec![channel],
        );

        let cmd_without_channel = create_test_command("test_command");

        let cache_with = GenerationCache::new(&[cmd_with_channel], &structs, &config).unwrap();
        let cache_without =
            GenerationCache::new(&[cmd_without_channel], &structs, &config).unwrap();

        // Commands hash should differ when channels differ
        assert_ne!(cache_with.commands_hash, cache_without.commands_hash);
    }

    #[test]
    fn test_save_creates_output_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_output = temp_dir.path().join("nested").join("output").join("dir");

        let commands = vec![create_test_command("test_command")];
        let structs = HashMap::new();
        let config = create_test_config();

        let cache = GenerationCache::new(&commands, &structs, &config).unwrap();

        // Should create nested directories
        cache.save(&nested_output).unwrap();

        assert!(nested_output.join(".typecache").exists());
    }

    #[test]
    fn test_load_nonexistent_cache() {
        let temp_dir = TempDir::new().unwrap();

        // Should return an error when cache doesn't exist
        let result = GenerationCache::load(temp_dir.path());
        assert!(result.is_err());
    }
}
