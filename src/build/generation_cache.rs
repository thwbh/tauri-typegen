use crate::interface::config::GenerateConfig;
use crate::models::{CommandInfo, EventInfo, StructInfo};
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
    /// Hash of all discovered events
    events_hash: String,
    /// Hash of configuration settings that affect output
    config_hash: String,
    /// Combined hash for quick comparison
    combined_hash: String,
}

impl GenerationCache {
    const CURRENT_VERSION: u32 = 2;

    /// Create a new cache from current generation state
    pub fn new(
        commands: &[CommandInfo],
        structs: &HashMap<String, StructInfo>,
        events: &[EventInfo],
        config: &GenerateConfig,
    ) -> Result<Self, CacheError> {
        let commands_hash = Self::hash_commands(commands)?;
        let structs_hash = Self::hash_structs(structs)?;
        let events_hash = Self::hash_events(events)?;
        let config_hash = Self::hash_config(config)?;
        let combined_hash =
            Self::combine_hashes(&commands_hash, &structs_hash, &events_hash, &config_hash)?;

        Ok(Self {
            version: Self::CURRENT_VERSION,
            commands_hash,
            structs_hash,
            events_hash,
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
        events: &[EventInfo],
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
        let current_cache = Self::new(commands, structs, events, config)?;

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
            serde_rename_all: Option<&'a str>,
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
            serde_rename: Option<&'a str>,
        }

        #[derive(Serialize)]
        struct ChannelHashData<'a> {
            parameter_name: &'a str,
            message_type: &'a str,
            serde_rename: Option<&'a str>,
        }

        let mut serialized_commands: Vec<String> = commands
            .iter()
            .map(|cmd| {
                serde_json::to_string(&CommandHashData {
                    name: &cmd.name,
                    serde_rename_all: cmd
                        .serde_rename_all
                        .as_ref()
                        .map(|rule| rule.to_rename_all_str()),
                    parameters: cmd
                        .parameters
                        .iter()
                        .map(|p| ParameterHashData {
                            name: &p.name,
                            rust_type: &p.rust_type,
                            is_optional: p.is_optional,
                            serde_rename: p.serde_rename.as_deref(),
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
                            serde_rename: c.serde_rename.as_deref(),
                        })
                        .collect(),
                })
            })
            .collect::<Result<_, _>>()?;
        serialized_commands.sort_unstable();

        let json = serde_json::to_string(&serialized_commands)?;
        Ok(Self::compute_hash(&json))
    }

    /// Generate a deterministic hash of events
    fn hash_events(events: &[EventInfo]) -> Result<String, CacheError> {
        #[derive(Serialize)]
        struct EventHashData<'a> {
            event_name: &'a str,
            payload_type: &'a str,
        }

        let mut serialized_events: Vec<String> = events
            .iter()
            .map(|event| {
                serde_json::to_string(&EventHashData {
                    event_name: &event.event_name,
                    payload_type: &event.payload_type,
                })
            })
            .collect::<Result<_, _>>()?;
        serialized_events.sort_unstable();

        let json = serde_json::to_string(&serialized_events)?;
        Ok(Self::compute_hash(&json))
    }

    /// Generate a deterministic hash of structs
    fn hash_structs(structs: &HashMap<String, StructInfo>) -> Result<String, CacheError> {
        #[derive(Serialize)]
        struct StructHashData<'a> {
            name: &'a str,
            is_enum: bool,
            serde_rename_all: Option<&'a str>,
            serde_tag: Option<&'a str>,
            fields: Vec<FieldHashData<'a>>,
            enum_variants: Vec<EnumVariantHashData<'a>>,
        }

        #[derive(Serialize)]
        struct FieldHashData<'a> {
            name: &'a str,
            rust_type: &'a str,
            is_optional: bool,
            is_public: bool,
            validator_attributes: Option<&'a crate::models::ValidatorAttributes>,
            serde_rename: Option<&'a str>,
            type_structure: &'a crate::models::TypeStructure,
        }

        #[derive(Serialize)]
        struct EnumVariantHashData<'a> {
            name: &'a str,
            serde_rename: Option<&'a str>,
            kind: &'a crate::models::EnumVariantKind,
        }

        let mut serialized_structs: Vec<String> = structs
            .values()
            .map(|s| {
                serde_json::to_string(&StructHashData {
                    name: &s.name,
                    is_enum: s.is_enum,
                    serde_rename_all: s
                        .serde_rename_all
                        .as_ref()
                        .map(|rule| rule.to_rename_all_str()),
                    serde_tag: s.serde_tag.as_deref(),
                    fields: s
                        .fields
                        .iter()
                        .map(|f| FieldHashData {
                            name: &f.name,
                            rust_type: &f.rust_type,
                            is_optional: f.is_optional,
                            is_public: f.is_public,
                            validator_attributes: f.validator_attributes.as_ref(),
                            serde_rename: f.serde_rename.as_deref(),
                            type_structure: &f.type_structure,
                        })
                        .collect(),
                    enum_variants: s
                        .enum_variants
                        .as_ref()
                        .map(|variants| {
                            variants
                                .iter()
                                .map(|variant| EnumVariantHashData {
                                    name: &variant.name,
                                    serde_rename: variant.serde_rename.as_deref(),
                                    kind: &variant.kind,
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
            })
            .collect::<Result<_, _>>()?;
        serialized_structs.sort_unstable();

        let json = serde_json::to_string(&serialized_structs)?;
        Ok(Self::compute_hash(&json))
    }

    /// Generate a hash of configuration settings that affect output
    fn hash_config(config: &GenerateConfig) -> Result<String, CacheError> {
        #[derive(Serialize)]
        struct ConfigHashData<'a> {
            validation_library: &'a str,
            include_private: bool,
            type_mappings: Option<Vec<(&'a str, &'a str)>>,
            default_parameter_case: &'a str,
            default_field_case: &'a str,
        }

        let type_mappings = config.type_mappings.as_ref().map(|mappings| {
            let mut canonical: Vec<_> = mappings
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str()))
                .collect();
            canonical.sort_unstable();
            canonical
        });

        let hash_data = ConfigHashData {
            validation_library: &config.validation_library,
            include_private: config.include_private.unwrap_or(false),
            type_mappings,
            default_parameter_case: &config.default_parameter_case,
            default_field_case: &config.default_field_case,
        };

        let json = serde_json::to_string(&hash_data)?;
        Ok(Self::compute_hash(&json))
    }

    /// Combine multiple hashes into a single hash
    fn combine_hashes(
        commands: &str,
        structs: &str,
        events: &str,
        config: &str,
    ) -> Result<String, CacheError> {
        let combined = format!("{}{}{}{}", commands, structs, events, config);
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
    use crate::models::{
        EnumVariantInfo, EnumVariantKind, FieldInfo, LengthConstraint, ParameterInfo,
        TypeStructure, ValidatorAttributes,
    };
    use serde_rename_rule::RenameRule;
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

    fn create_test_event(name: &str) -> EventInfo {
        EventInfo {
            event_name: name.to_string(),
            payload_type: "String".to_string(),
            payload_type_structure: crate::models::TypeStructure::Primitive("string".to_string()),
            file_path: "events.rs".to_string(),
            line_number: 1,
        }
    }

    #[test]
    fn test_cache_creation() {
        let commands = vec![create_test_command("test_command")];
        let structs = HashMap::new();
        let config = create_test_config();

        let cache = GenerationCache::new(&commands, &structs, &[], &config).unwrap();

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

        let cache = GenerationCache::new(&commands, &structs, &[], &config).unwrap();
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
            GenerationCache::needs_regeneration(temp_dir.path(), &commands, &structs, &[], &config)
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
        let cache = GenerationCache::new(&commands, &structs, &[], &config).unwrap();
        cache.save(temp_dir.path()).unwrap();

        // Check if regeneration needed with same data
        let needs_regen =
            GenerationCache::needs_regeneration(temp_dir.path(), &commands, &structs, &[], &config)
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
        let cache = GenerationCache::new(&commands, &structs, &[], &config).unwrap();
        cache.save(temp_dir.path()).unwrap();

        // Change commands
        let new_commands = vec![create_test_command("different_command")];

        let needs_regen = GenerationCache::needs_regeneration(
            temp_dir.path(),
            &new_commands,
            &structs,
            &[],
            &config,
        )
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
        let cache = GenerationCache::new(&commands, &structs, &[], &config).unwrap();
        cache.save(temp_dir.path()).unwrap();

        // Change config
        let mut new_config = config;
        new_config.validation_library = "zod".to_string();

        let needs_regen = GenerationCache::needs_regeneration(
            temp_dir.path(),
            &commands,
            &structs,
            &[],
            &new_config,
        )
        .unwrap();

        assert!(needs_regen);
    }

    #[test]
    fn test_hash_determinism() {
        let commands = vec![create_test_command("test_command")];
        let structs = HashMap::new();
        let config = create_test_config();

        let cache1 = GenerationCache::new(&commands, &structs, &[], &config).unwrap();
        let cache2 = GenerationCache::new(&commands, &structs, &[], &config).unwrap();

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
            GenerationCache::needs_regeneration(temp_dir.path(), &commands, &structs, &[], &config)
                .unwrap();

        assert!(needs_regen);
    }

    #[test]
    fn test_empty_commands_and_structs() {
        let commands: Vec<CommandInfo> = vec![];
        let structs: HashMap<String, crate::models::StructInfo> = HashMap::new();
        let config = create_test_config();

        let cache = GenerationCache::new(&commands, &structs, &[], &config).unwrap();

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

        let cache1 = GenerationCache::new(&commands, &structs1, &[], &config).unwrap();
        let cache2 = GenerationCache::new(&commands, &structs2, &[], &config).unwrap();

        // Hash should be the same regardless of insertion order
        assert_eq!(cache1.structs_hash, cache2.structs_hash);
        assert_eq!(cache1.combined_hash, cache2.combined_hash);
    }

    #[test]
    fn command_hash_order_independence() {
        let config = create_test_config();
        let structs = HashMap::new();

        let commands1 = vec![
            create_test_command("alpha_command"),
            create_test_command("beta_command"),
        ];
        let commands2 = vec![
            create_test_command("beta_command"),
            create_test_command("alpha_command"),
        ];

        let cache1 = GenerationCache::new(&commands1, &structs, &[], &config).unwrap();
        let cache2 = GenerationCache::new(&commands2, &structs, &[], &config).unwrap();

        assert_eq!(cache1.commands_hash, cache2.commands_hash);
        assert_eq!(cache1.combined_hash, cache2.combined_hash);
    }

    #[test]
    fn command_hash_ignores_source_location() {
        let config = create_test_config();
        let structs = HashMap::new();

        let command1 = CommandInfo::new_for_test(
            "test_command",
            "src/alpha.rs",
            10,
            vec![],
            "String",
            false,
            vec![],
        );
        let command2 = CommandInfo::new_for_test(
            "test_command",
            "src/beta.rs",
            200,
            vec![],
            "String",
            false,
            vec![],
        );

        let cache1 = GenerationCache::new(&[command1], &structs, &[], &config).unwrap();
        let cache2 = GenerationCache::new(&[command2], &structs, &[], &config).unwrap();

        assert_eq!(cache1.commands_hash, cache2.commands_hash);
        assert_eq!(cache1.combined_hash, cache2.combined_hash);
    }

    #[test]
    fn event_hash_ignores_source_location() {
        let config = create_test_config();
        let commands = vec![create_test_command("test_command")];
        let structs = HashMap::new();

        let event1 = EventInfo {
            event_name: "alpha-ready".to_string(),
            payload_type: "String".to_string(),
            payload_type_structure: crate::models::TypeStructure::Primitive("string".to_string()),
            file_path: "src/alpha.rs".to_string(),
            line_number: 10,
        };
        let event2 = EventInfo {
            event_name: "alpha-ready".to_string(),
            payload_type: "String".to_string(),
            payload_type_structure: crate::models::TypeStructure::Primitive("string".to_string()),
            file_path: "src/beta.rs".to_string(),
            line_number: 200,
        };

        let cache1 = GenerationCache::new(&commands, &structs, &[event1], &config).unwrap();
        let cache2 = GenerationCache::new(&commands, &structs, &[event2], &config).unwrap();

        assert_eq!(cache1.events_hash, cache2.events_hash);
        assert_eq!(cache1.combined_hash, cache2.combined_hash);
    }

    #[test]
    fn struct_hash_ignores_source_location() {
        let config = create_test_config();
        let commands = vec![create_test_command("test_command")];

        let struct1 = StructInfo {
            name: "Payload".to_string(),
            fields: vec![FieldInfo {
                name: "value".to_string(),
                rust_type: "String".to_string(),
                is_optional: false,
                is_public: true,
                validator_attributes: None,
                serde_rename: None,
                type_structure: TypeStructure::Primitive("string".to_string()),
            }],
            file_path: "src/alpha.rs".to_string(),
            is_enum: false,
            serde_rename_all: None,
            serde_tag: None,
            enum_variants: None,
        };
        let struct2 = StructInfo {
            file_path: "src/beta.rs".to_string(),
            ..struct1.clone()
        };

        let mut structs1 = HashMap::new();
        structs1.insert("Payload".to_string(), struct1);

        let mut structs2 = HashMap::new();
        structs2.insert("Payload".to_string(), struct2);

        let cache1 = GenerationCache::new(&commands, &structs1, &[], &config).unwrap();
        let cache2 = GenerationCache::new(&commands, &structs2, &[], &config).unwrap();

        assert_eq!(cache1.structs_hash, cache2.structs_hash);
        assert_eq!(cache1.combined_hash, cache2.combined_hash);
    }

    #[test]
    fn command_hash_changes_with_serde_metadata() {
        let config = create_test_config();
        let structs = HashMap::new();

        let mut command1 = CommandInfo::new_for_test(
            "test_command",
            "src/test.rs",
            10,
            vec![ParameterInfo {
                name: "user_id".to_string(),
                rust_type: "String".to_string(),
                is_optional: false,
                type_structure: TypeStructure::Primitive("string".to_string()),
                serde_rename: None,
            }],
            "String",
            false,
            vec![crate::models::ChannelInfo::new_for_test(
                "progress_updates",
                "String",
                "test_command",
                "src/test.rs",
                10,
            )],
        );
        let mut command2 = CommandInfo::new_for_test(
            "test_command",
            "src/test.rs",
            10,
            vec![ParameterInfo {
                name: "user_id".to_string(),
                rust_type: "String".to_string(),
                is_optional: false,
                type_structure: TypeStructure::Primitive("string".to_string()),
                serde_rename: Some("userIdExplicit".to_string()),
            }],
            "String",
            false,
            vec![crate::models::ChannelInfo::new_for_test(
                "progress_updates",
                "String",
                "test_command",
                "src/test.rs",
                10,
            )],
        );
        command1.serde_rename_all = Some(RenameRule::SnakeCase);
        command2.channels[0].serde_rename = Some("progressUpdates".to_string());

        let cache1 = GenerationCache::new(&[command1], &structs, &[], &config).unwrap();
        let cache2 = GenerationCache::new(&[command2], &structs, &[], &config).unwrap();

        assert_ne!(cache1.commands_hash, cache2.commands_hash);
        assert_ne!(cache1.combined_hash, cache2.combined_hash);
    }

    #[test]
    fn struct_hash_changes_with_field_metadata() {
        let config = create_test_config();
        let commands = vec![create_test_command("test_command")];

        let struct1 = StructInfo {
            name: "Payload".to_string(),
            fields: vec![FieldInfo {
                name: "created_at".to_string(),
                rust_type: "String".to_string(),
                is_optional: false,
                is_public: true,
                validator_attributes: None,
                serde_rename: None,
                type_structure: TypeStructure::Primitive("string".to_string()),
            }],
            file_path: "src/payload.rs".to_string(),
            is_enum: false,
            serde_rename_all: None,
            serde_tag: None,
            enum_variants: None,
        };
        let struct2 = StructInfo {
            fields: vec![FieldInfo {
                name: "created_at".to_string(),
                rust_type: "String".to_string(),
                is_optional: false,
                is_public: true,
                validator_attributes: Some(ValidatorAttributes {
                    length: Some(LengthConstraint {
                        min: Some(1),
                        max: None,
                        message: Some("required".to_string()),
                    }),
                    range: None,
                    email: false,
                    url: false,
                    custom_message: Some("required".to_string()),
                }),
                serde_rename: Some("createdAt".to_string()),
                type_structure: TypeStructure::Primitive("string".to_string()),
            }],
            serde_rename_all: Some(RenameRule::CamelCase),
            ..struct1.clone()
        };

        let mut structs1 = HashMap::new();
        structs1.insert("Payload".to_string(), struct1);

        let mut structs2 = HashMap::new();
        structs2.insert("Payload".to_string(), struct2);

        let cache1 = GenerationCache::new(&commands, &structs1, &[], &config).unwrap();
        let cache2 = GenerationCache::new(&commands, &structs2, &[], &config).unwrap();

        assert_ne!(cache1.structs_hash, cache2.structs_hash);
        assert_ne!(cache1.combined_hash, cache2.combined_hash);
    }

    #[test]
    fn struct_hash_changes_with_enum_metadata() {
        let config = create_test_config();
        let commands = vec![create_test_command("test_command")];

        let base_variant = EnumVariantInfo {
            name: "ReadyState".to_string(),
            kind: EnumVariantKind::Struct(vec![FieldInfo {
                name: "event_id".to_string(),
                rust_type: "String".to_string(),
                is_optional: false,
                is_public: true,
                validator_attributes: None,
                serde_rename: None,
                type_structure: TypeStructure::Primitive("string".to_string()),
            }]),
            serde_rename: None,
        };
        let renamed_variant = EnumVariantInfo {
            serde_rename: Some("ready_state".to_string()),
            ..base_variant.clone()
        };

        let enum1 = StructInfo {
            name: "StatusEvent".to_string(),
            fields: vec![],
            file_path: "src/status.rs".to_string(),
            is_enum: true,
            serde_rename_all: None,
            serde_tag: None,
            enum_variants: Some(vec![base_variant]),
        };
        let enum2 = StructInfo {
            serde_rename_all: Some(RenameRule::SnakeCase),
            serde_tag: Some("kind".to_string()),
            enum_variants: Some(vec![renamed_variant]),
            ..enum1.clone()
        };

        let mut structs1 = HashMap::new();
        structs1.insert("StatusEvent".to_string(), enum1);

        let mut structs2 = HashMap::new();
        structs2.insert("StatusEvent".to_string(), enum2);

        let cache1 = GenerationCache::new(&commands, &structs1, &[], &config).unwrap();
        let cache2 = GenerationCache::new(&commands, &structs2, &[], &config).unwrap();

        assert_ne!(cache1.structs_hash, cache2.structs_hash);
        assert_ne!(cache1.combined_hash, cache2.combined_hash);
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
            GenerationCache::needs_regeneration(temp_dir.path(), &commands, &structs, &[], &config)
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

        let cache1 = GenerationCache::new(&commands, &structs, &[], &config1).unwrap();
        let cache2 = GenerationCache::new(&commands, &structs, &[], &config2).unwrap();

        // Config hash should differ when type_mappings differ
        assert_ne!(cache1.config_hash, cache2.config_hash);
        assert_ne!(cache1.combined_hash, cache2.combined_hash);
    }

    #[test]
    fn config_hash_type_mappings_order_independence() {
        let commands = vec![create_test_command("test_command")];
        let structs = HashMap::new();

        let mut config1 = create_test_config();
        let mut mappings1 = HashMap::new();
        mappings1.insert("First".to_string(), "string".to_string());
        mappings1.insert("Second".to_string(), "number".to_string());
        config1.type_mappings = Some(mappings1);

        let mut config2 = create_test_config();
        let mut mappings2 = HashMap::new();
        mappings2.insert("Second".to_string(), "number".to_string());
        mappings2.insert("First".to_string(), "string".to_string());
        config2.type_mappings = Some(mappings2);

        let cache1 = GenerationCache::new(&commands, &structs, &[], &config1).unwrap();
        let cache2 = GenerationCache::new(&commands, &structs, &[], &config2).unwrap();

        assert_eq!(cache1.config_hash, cache2.config_hash);
        assert_eq!(cache1.combined_hash, cache2.combined_hash);
    }

    #[test]
    fn events_change_requires_regeneration() {
        let temp_dir = TempDir::new().unwrap();
        let commands = vec![create_test_command("test_command")];
        let structs = HashMap::new();
        let config = create_test_config();
        let initial_events = vec![create_test_event("alpha-ready")];
        let changed_events = vec![create_test_event("beta-ready")];

        let cache = GenerationCache::new(&commands, &structs, &initial_events, &config).unwrap();
        cache.save(temp_dir.path()).unwrap();

        let needs_regen = GenerationCache::needs_regeneration(
            temp_dir.path(),
            &commands,
            &structs,
            &changed_events,
            &config,
        )
        .unwrap();

        assert!(needs_regen);
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

        let cache_with = GenerationCache::new(&[cmd_with_channel], &structs, &[], &config).unwrap();
        let cache_without =
            GenerationCache::new(&[cmd_without_channel], &structs, &[], &config).unwrap();

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

        let cache = GenerationCache::new(&commands, &structs, &[], &config).unwrap();

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
