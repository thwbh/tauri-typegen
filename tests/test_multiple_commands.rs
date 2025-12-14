use std::collections::HashMap;
use std::fs;
use tauri_typegen::analysis::CommandAnalyzer;
use tauri_typegen::generators::create_generator;
use tauri_typegen::models::{CommandInfo, FieldInfo, ParameterInfo, StructInfo, TypeStructure};
use tempfile::TempDir;

// Helper function to create a test Rust file with mixed content
fn create_test_rust_file_with_unused_types(temp_dir: &TempDir) -> std::path::PathBuf {
    let file_path = temp_dir.path().join("mixed_types.rs");
    let content = r#"
use tauri::command;
use serde::{Deserialize, Serialize};

// These structs should be included (used by commands)
#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: Option<String>,
}

// These structs should NOT be included (not used by any command)
#[derive(Serialize, Deserialize)]
pub struct sqlite3_pcache_methods {
    pub iVersion: i32,
    pub pArg: *mut std::ffi::c_void,
}

#[derive(Serialize, Deserialize)]
pub struct UnusedStruct {
    pub unused_field: String,
}

pub struct AnotherUnusedStruct {
    pub data: Vec<i32>,
}

// Commands that use only some of the types
#[command]
pub async fn create_user(request: CreateUserRequest) -> Result<User, String> {
    Ok(User {
        id: 1,
        name: request.name,
        email: request.email,
    })
}

#[command]
pub async fn get_user_count() -> Result<i32, String> {
    Ok(42)
}

// This function is not a command, so even if it uses types, they shouldn't be included
pub fn internal_function(_unused: UnusedStruct) -> AnotherUnusedStruct {
    AnotherUnusedStruct { data: vec![1, 2, 3] }
}
"#;

    fs::write(&file_path, content).unwrap();
    file_path
}

fn create_sample_commands_with_unused_structs() -> Vec<CommandInfo> {
    vec![
        CommandInfo::new_for_test(
            "create_user",
            "test_file.rs",
            10,
            vec![ParameterInfo {
                name: "request".to_string(),
                rust_type: "CreateUserRequest".to_string(),
                is_optional: false,
                type_structure: TypeStructure::Custom("CreateUserRequest".to_string()),
                serde_rename: None,
            }],
            "User",
            true,
            vec![],
        ),
        CommandInfo::new_for_test(
            "get_user_count",
            "test_file.rs",
            15,
            vec![],
            "number",
            true,
            vec![],
        ),
    ]
}

fn create_sample_structs_with_unused() -> HashMap<String, StructInfo> {
    let mut structs = HashMap::new();

    // Used structs (should be included)
    structs.insert(
        "User".to_string(),
        StructInfo {
            name: "User".to_string(),
            fields: vec![
                FieldInfo {
                    name: "id".to_string(),
                    rust_type: "i32".to_string(),
                    is_optional: false,
                    is_public: true,
                    validator_attributes: None,
                    type_structure: TypeStructure::Primitive("number".to_string()),
                    serde_rename: None,
                },
                FieldInfo {
                    name: "name".to_string(),
                    rust_type: "String".to_string(),
                    is_optional: false,
                    is_public: true,
                    validator_attributes: None,
                    type_structure: TypeStructure::Primitive("string".to_string()),
                    serde_rename: None,
                },
                FieldInfo {
                    name: "email".to_string(),
                    rust_type: "Option<String>".to_string(),
                    is_optional: true,
                    is_public: true,
                    validator_attributes: None,
                    type_structure: TypeStructure::Optional(Box::new(TypeStructure::Primitive(
                        "string".to_string(),
                    ))),
                    serde_rename: None,
                },
            ],
            file_path: "test_file.rs".to_string(),
            is_enum: false,
            serde_rename_all: None,
        },
    );

    structs.insert(
        "CreateUserRequest".to_string(),
        StructInfo {
            name: "CreateUserRequest".to_string(),
            fields: vec![
                FieldInfo {
                    name: "name".to_string(),
                    rust_type: "String".to_string(),
                    is_optional: false,
                    is_public: true,
                    validator_attributes: None,
                    type_structure: TypeStructure::Primitive("string".to_string()),
                    serde_rename: None,
                },
                FieldInfo {
                    name: "email".to_string(),
                    rust_type: "Option<String>".to_string(),
                    is_optional: true,
                    is_public: true,
                    validator_attributes: None,
                    type_structure: TypeStructure::Optional(Box::new(TypeStructure::Primitive(
                        "string".to_string(),
                    ))),
                    serde_rename: None,
                },
            ],
            file_path: "test_file.rs".to_string(),
            is_enum: false,
            serde_rename_all: None,
        },
    );

    // Unused structs (should NOT be included)
    structs.insert(
        "sqlite3_pcache_methods".to_string(),
        StructInfo {
            name: "sqlite3_pcache_methods".to_string(),
            fields: vec![FieldInfo {
                name: "iVersion".to_string(),
                rust_type: "i32".to_string(),
                is_optional: false,
                is_public: true,
                validator_attributes: None,
                type_structure: TypeStructure::Primitive("number".to_string()),
                serde_rename: Some("iVersion".to_string()),
            }],
            file_path: "test_file.rs".to_string(),
            is_enum: false,
            serde_rename_all: None,
        },
    );

    structs.insert(
        "UnusedStruct".to_string(),
        StructInfo {
            name: "UnusedStruct".to_string(),
            fields: vec![FieldInfo {
                name: "unused_field".to_string(),
                rust_type: "String".to_string(),
                is_optional: false,
                is_public: true,
                validator_attributes: None,
                type_structure: TypeStructure::Primitive("string".to_string()),
                serde_rename: None,
            }],
            file_path: "test_file.rs".to_string(),
            is_enum: false,
            serde_rename_all: Some("camelCase".to_string()),
        },
    );

    structs.insert(
        "AnotherUnusedStruct".to_string(),
        StructInfo {
            name: "AnotherUnusedStruct".to_string(),
            fields: vec![FieldInfo {
                name: "data".to_string(),
                rust_type: "Vec<i32>".to_string(),
                is_optional: false,
                is_public: true,
                validator_attributes: None,
                type_structure: TypeStructure::Array(Box::new(TypeStructure::Primitive(
                    "number".to_string(),
                ))),
                serde_rename: None,
            }],
            file_path: "test_file.rs".to_string(),
            is_enum: false,
            serde_rename_all: None,
        },
    );

    structs
}

#[test]
fn test_only_generates_types_used_by_commands() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = create_sample_commands_with_unused_structs();
    let all_discovered_structs = create_sample_structs_with_unused();

    let mut generator = create_generator(None);
    generator
        .generate_models(
            &commands,
            &all_discovered_structs,
            output_path,
            &CommandAnalyzer::new(),
        )
        .unwrap();

    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();

    println!("Generated types.ts content:\n{}", types_content);

    // Should contain interfaces for types used by commands
    assert!(types_content.contains("export interface User"));
    assert!(types_content.contains("export interface CreateUserRequest"));
    assert!(types_content.contains("export interface CreateUserParams"));

    // Should contain the fields of used types
    assert!(types_content.contains("id: number;"));
    assert!(types_content.contains("name: string;"));
    assert!(types_content.contains("email?: string | null;"));

    // Should NOT contain interfaces for unused types
    assert!(!types_content.contains("export interface sqlite3_pcache_methods"));
    assert!(!types_content.contains("export interface UnusedStruct"));
    assert!(!types_content.contains("export interface AnotherUnusedStruct"));

    // Should not contain fields from unused types
    assert!(!types_content.contains("iVersion"));
    assert!(!types_content.contains("unused_field"));

    // Should NOT generate params interface for commands with no parameters
    assert!(!types_content.contains("GetUserCountParams"));
}

#[test]
fn test_collect_referenced_types_handles_complex_types() {
    use tauri_typegen::generators::TypeCollector;
    use tauri_typegen::TypeStructure;

    let type_collector = TypeCollector::new();
    let mut used_types = std::collections::HashSet::new();

    // Test Result type extraction
    let result_type = TypeStructure::Result(Box::new(TypeStructure::Custom("User".to_string())));
    type_collector.collect_referenced_types_from_structure(&result_type, &mut used_types);
    assert!(used_types.contains("User"));

    used_types.clear();

    // Test Option type extraction
    let option_type = TypeStructure::Optional(Box::new(TypeStructure::Custom(
        "CreateUserRequest".to_string(),
    )));
    type_collector.collect_referenced_types_from_structure(&option_type, &mut used_types);
    assert!(used_types.contains("CreateUserRequest"));

    used_types.clear();

    // Test Vec type extraction
    let vec_type = TypeStructure::Array(Box::new(TypeStructure::Custom("Product".to_string())));
    type_collector.collect_referenced_types_from_structure(&vec_type, &mut used_types);
    assert!(used_types.contains("Product"));

    used_types.clear();

    // Test nested types
    let nested_type = TypeStructure::Result(Box::new(TypeStructure::Array(Box::new(
        TypeStructure::Custom("User".to_string()),
    ))));
    type_collector.collect_referenced_types_from_structure(&nested_type, &mut used_types);
    assert!(used_types.contains("User"));

    used_types.clear();

    // Test primitive types are not collected
    let string_type = TypeStructure::Primitive("String".to_string());
    type_collector.collect_referenced_types_from_structure(&string_type, &mut used_types);
    assert!(used_types.is_empty());

    let int_type = TypeStructure::Primitive("i32".to_string());
    type_collector.collect_referenced_types_from_structure(&int_type, &mut used_types);
    assert!(used_types.is_empty());

    let bool_type = TypeStructure::Primitive("bool".to_string());
    type_collector.collect_referenced_types_from_structure(&bool_type, &mut used_types);
    assert!(used_types.is_empty());
}

#[test]
fn test_integration_with_real_analyzer() {
    let temp_dir = TempDir::new().unwrap();
    let _file_path = create_test_rust_file_with_unused_types(&temp_dir);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(&temp_dir.path().to_string_lossy())
        .unwrap();

    // Should find the two commands
    assert_eq!(commands.len(), 2);

    let command_names: Vec<&String> = commands.iter().map(|c| &c.name).collect();
    assert!(command_names.contains(&&"create_user".to_string()));
    assert!(command_names.contains(&&"get_user_count".to_string()));

    // Generate TypeScript from the analyzed commands
    let output_path = temp_dir.path().join("output");
    fs::create_dir_all(&output_path).unwrap();

    let mut generator = create_generator(None);
    generator
        .generate_models(
            &commands,
            analyzer.get_discovered_structs(),
            &output_path.to_string_lossy(),
            &analyzer,
        )
        .unwrap();

    let types_content = fs::read_to_string(output_path.join("types.ts")).unwrap();

    // Should only contain types used by the commands
    assert!(types_content.contains("export interface User"));
    assert!(types_content.contains("export interface CreateUserRequest"));

    // Should NOT contain the unused types
    assert!(!types_content.contains("sqlite3_pcache_methods"));
    assert!(!types_content.contains("UnusedStruct"));
    assert!(!types_content.contains("AnotherUnusedStruct"));
}

#[test]
fn test_type_filtering_with_validation_library() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = create_sample_commands_with_unused_structs();
    let all_discovered_structs = create_sample_structs_with_unused();

    // Test with Zod validation
    let mut generator = create_generator(Some("zod".to_string()));
    generator
        .generate_models(
            &commands,
            &all_discovered_structs,
            output_path,
            &CommandAnalyzer::new(),
        )
        .unwrap();

    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();
    let commands_content = fs::read_to_string(temp_dir.path().join("commands.ts")).unwrap();

    // Types file should only contain used types (zod format)
    assert!(types_content.contains("export type User") || types_content.contains("UserSchema"));
    assert!(!types_content.contains("sqlite3_pcache_methods"));

    // Schemas should only be generated for commands with parameters (embedded in types.ts for zod)
    assert!(types_content.contains("CreateUserParamsSchema"));
    assert!(!types_content.contains("GetUserCountParamsSchema")); // No params

    // Commands should reference the correct types
    assert!(commands_content.contains("types.CreateUserParams"));
    assert!(commands_content.contains("Promise<types.User>"));
}

#[test]
fn test_empty_commands_generates_no_unnecessary_types() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    // Empty commands list
    let commands = vec![];
    let all_discovered_structs = create_sample_structs_with_unused();

    let mut generator = create_generator(None);
    generator
        .generate_models(
            &commands,
            &all_discovered_structs,
            output_path,
            &CommandAnalyzer::new(),
        )
        .unwrap();

    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();

    // Should contain only the header comment, no interfaces
    assert!(types_content.contains("Auto-generated TypeScript bindings"));
    assert!(!types_content.contains("export interface"));

    // Verify no struct types were generated
    assert!(!types_content.contains("User"));
    assert!(!types_content.contains("sqlite3_pcache_methods"));
    assert!(!types_content.contains("UnusedStruct"));
}
