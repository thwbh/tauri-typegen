use std::collections::HashMap;
use std::fs;
use tauri_typegen::analysis::CommandAnalyzer;
use tauri_typegen::generators::generator::BindingsGenerator;
use tauri_typegen::models::{CommandInfo, ParameterInfo, StructInfo};
use tempfile::TempDir;

fn create_sample_commands() -> Vec<CommandInfo> {
    vec![
        CommandInfo {
            name: "greet".to_string(),
            file_path: "test_file.rs".to_string(),
            line_number: 10,
            parameters: vec![ParameterInfo {
                name: "name".to_string(),
                rust_type: "String".to_string(),
                typescript_type: "string".to_string(),
                is_optional: false,
            }],
            return_type: "string".to_string(),
            return_type_ts: "string".to_string(),
            is_async: true,
            channels: vec![],
        },
        CommandInfo {
            name: "get_user_count".to_string(),
            file_path: "test_file.rs".to_string(),
            line_number: 15,
            parameters: vec![],
            return_type: "number".to_string(),
            return_type_ts: "number".to_string(),
            is_async: false,
            channels: vec![],
        },
    ]
}

fn create_empty_structs() -> HashMap<String, StructInfo> {
    HashMap::new() // Empty struct map for basic tests
}

#[test]
fn test_generator_creates_all_files() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = create_sample_commands();
    let discovered_structs = create_empty_structs();

    let mut generator = BindingsGenerator::new(Some("zod".to_string()));
    let generated_files = generator
        .generate_models(
            &commands,
            &discovered_structs,
            output_path,
            &CommandAnalyzer::new(),
        )
        .unwrap();

    assert_eq!(generated_files.len(), 3);
    assert!(generated_files.contains(&"types.ts".to_string()));
    assert!(generated_files.contains(&"commands.ts".to_string()));
    assert!(generated_files.contains(&"index.ts".to_string()));

    // Verify files exist
    assert!(temp_dir.path().join("types.ts").exists());
    assert!(temp_dir.path().join("commands.ts").exists());
    assert!(temp_dir.path().join("index.ts").exists());
}

#[test]
fn test_generator_without_validation_library() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = create_sample_commands();
    let discovered_structs = create_empty_structs();

    let mut generator = BindingsGenerator::new(Some("none".to_string()));
    let generated_files = generator
        .generate_models(
            &commands,
            &discovered_structs,
            output_path,
            &CommandAnalyzer::new(),
        )
        .unwrap();

    // Should generate 3 files (no schemas.ts)
    assert_eq!(generated_files.len(), 3);
    assert!(!generated_files.contains(&"schemas.ts".to_string()));
    assert!(!temp_dir.path().join("schemas.ts").exists());
}

#[test]
fn test_types_file_generation() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = create_sample_commands();
    let discovered_structs = create_empty_structs();

    let mut generator = BindingsGenerator::new(None);
    generator
        .generate_models(
            &commands,
            &discovered_structs,
            output_path,
            &CommandAnalyzer::new(),
        )
        .unwrap();

    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();

    // Should contain parameter interfaces for commands with parameters (vanilla TypeScript)
    assert!(types_content.contains("export interface GreetParams"));
    assert!(types_content.contains("name: string;"));

    // Should NOT contain params interface for commands without parameters
    assert!(!types_content.contains("GetUserCountParams"));
}

#[test]
fn test_zod_schemas_in_types_file() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = create_sample_commands();
    let discovered_structs = create_empty_structs();

    let mut generator = BindingsGenerator::new(Some("zod".to_string()));
    generator
        .generate_models(
            &commands,
            &discovered_structs,
            output_path,
            &CommandAnalyzer::new(),
        )
        .unwrap();

    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();

    // Schemas are now embedded in types.ts file
    assert!(types_content.contains("import { z } from 'zod';"));
    assert!(types_content.contains("GreetParamsSchema"));
    assert!(types_content.contains("z.object({"));
    assert!(types_content.contains("name: z.string()"));

    // Should not generate schema for commands without parameters
    assert!(!types_content.contains("GetUserCountParamsSchema"));
}

#[test]
fn test_yup_schemas_generation() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = create_sample_commands();
    let discovered_structs = create_empty_structs();

    // Yup support removed - should fall back to vanilla generator
    let mut generator = BindingsGenerator::new(Some("yup".to_string()));
    let generated_files = generator
        .generate_models(
            &commands,
            &discovered_structs,
            output_path,
            &CommandAnalyzer::new(),
        )
        .unwrap();

    // Should generate vanilla files (no schemas.ts for yup)
    assert_eq!(generated_files.len(), 3);
    assert!(generated_files.contains(&"types.ts".to_string()));
    assert!(generated_files.contains(&"commands.ts".to_string()));
    assert!(generated_files.contains(&"index.ts".to_string()));
    assert!(!generated_files.contains(&"schemas.ts".to_string()));
}

#[test]
fn test_commands_file_generation() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = create_sample_commands();
    let discovered_structs = create_empty_structs();

    let mut generator = BindingsGenerator::new(Some("zod".to_string()));
    generator
        .generate_models(
            &commands,
            &discovered_structs,
            output_path,
            &CommandAnalyzer::new(),
        )
        .unwrap();

    let commands_content = fs::read_to_string(temp_dir.path().join("commands.ts")).unwrap();

    assert!(commands_content.contains("import { invoke } from '@tauri-apps/api/core';"));
    assert!(commands_content.contains("import * as types from './types';"));

    // Check specific command functions
    assert!(commands_content.contains("export async function greet"));
    assert!(commands_content.contains("params: types.GreetParams"));
    assert!(commands_content.contains("hooks?: CommandHooks<string>"));
    assert!(commands_content.contains("Promise<string>"));
    assert!(commands_content.contains("types.GreetParamsSchema.safeParse(params)"));
    assert!(commands_content.contains("invoke<string>('greet'"));

    // Check command without parameters has hooks
    assert!(commands_content.contains("export async function getUserCount"));
    assert!(commands_content.contains("hooks?: CommandHooks<number>"));
    assert!(commands_content.contains("invoke<number>('get_user_count')"));
}

#[test]
fn test_commands_without_validation() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = create_sample_commands();
    let discovered_structs = create_empty_structs();

    let mut generator = BindingsGenerator::new(Some("none".to_string()));
    generator
        .generate_models(
            &commands,
            &discovered_structs,
            output_path,
            &CommandAnalyzer::new(),
        )
        .unwrap();

    let commands_content = fs::read_to_string(temp_dir.path().join("commands.ts")).unwrap();

    // Should not import schemas
    assert!(!commands_content.contains("import * as schemas"));
    assert!(commands_content.contains("return invoke('greet', params);"));
    assert!(!commands_content.contains("parse(params)"));
}

#[test]
fn test_index_file_generation() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = create_sample_commands();
    let discovered_structs = create_empty_structs();

    let mut generator = BindingsGenerator::new(None);
    generator
        .generate_models(
            &commands,
            &discovered_structs,
            output_path,
            &CommandAnalyzer::new(),
        )
        .unwrap();

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();

    assert!(index_content.contains("export * from './types';"));
    assert!(index_content.contains("export * from './commands';"));
}

#[test]
fn test_pascal_case_conversion() {
    let generator = BindingsGenerator::new(None);

    assert_eq!(generator.to_pascal_case("hello_world"), "HelloWorld");
    assert_eq!(generator.to_pascal_case("get_user_count"), "GetUserCount");
    assert_eq!(generator.to_pascal_case("simple"), "Simple");
    assert_eq!(generator.to_pascal_case(""), "");
}

#[test]
fn test_typescript_to_zod_type_conversion() {
    let generator = BindingsGenerator::new(None);

    assert_eq!(generator.typescript_to_zod_type("string"), "z.string()");
    assert_eq!(
        generator.typescript_to_zod_type("number"),
        "z.coerce.number()"
    );
    assert_eq!(generator.typescript_to_zod_type("boolean"), "z.boolean()");
    assert_eq!(
        generator.typescript_to_zod_type("string[]"),
        "z.array(z.string())"
    );
    assert_eq!(
        generator.typescript_to_zod_type("string | null"),
        "z.string().nullable()"
    );
    assert_eq!(
        generator.typescript_to_zod_type("CustomType"),
        "z.lazy(() => z.any()) /* CustomType - define schema separately if needed */"
    );
}

#[test]
fn test_typescript_to_yup_type_conversion() {
    let generator = BindingsGenerator::new(None);

    // Yup support has been removed - all types return the removed message
    assert!(generator
        .typescript_to_yup_type("string")
        .contains("yup support removed"));
    assert!(generator
        .typescript_to_yup_type("number")
        .contains("yup support removed"));
    assert!(generator
        .typescript_to_yup_type("boolean")
        .contains("yup support removed"));
    assert!(generator
        .typescript_to_yup_type("string[]")
        .contains("yup support removed"));
    assert!(generator
        .typescript_to_yup_type("string | null")
        .contains("yup support removed"));
    assert!(generator
        .typescript_to_yup_type("CustomType")
        .contains("yup support removed"));
}

#[test]
fn test_custom_type_detection() {
    let generator = BindingsGenerator::new(None);

    assert!(!generator.is_custom_type("string"));
    assert!(!generator.is_custom_type("number"));
    assert!(!generator.is_custom_type("boolean"));
    assert!(!generator.is_custom_type("void"));
    assert!(!generator.is_custom_type("string[]"));
    assert!(!generator.is_custom_type("string | null"));

    assert!(generator.is_custom_type("User"));
    assert!(generator.is_custom_type("CreateUserRequest"));
}

#[test]
fn test_generator_with_void_return() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = vec![CommandInfo {
        name: "delete_item".to_string(),
        file_path: "test_file.rs".to_string(),
        line_number: 10,
        parameters: vec![ParameterInfo {
            name: "id".to_string(),
            rust_type: "i32".to_string(),
            typescript_type: "number".to_string(),
            is_optional: false,
        }],
        return_type: "void".to_string(),
        return_type_ts: "void".to_string(),
        is_async: true,
        channels: vec![],
    }];
    let discovered_structs = create_empty_structs();

    let mut generator = BindingsGenerator::new(None);
    generator
        .generate_models(
            &commands,
            &discovered_structs,
            output_path,
            &CommandAnalyzer::new(),
        )
        .unwrap();

    let commands_content = fs::read_to_string(temp_dir.path().join("commands.ts")).unwrap();
    assert!(commands_content.contains("Promise<void>"));
}

#[test]
fn test_generator_empty_commands_list() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = vec![];
    let discovered_structs = create_empty_structs();

    let mut generator = BindingsGenerator::new(None);
    let generated_files = generator
        .generate_models(
            &commands,
            &discovered_structs,
            output_path,
            &CommandAnalyzer::new(),
        )
        .unwrap();

    // Should still generate files, just with empty content
    assert_eq!(generated_files.len(), 3);

    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();
    // Should contain header but no interfaces
    assert!(types_content.contains("Auto-generated TypeScript types"));
    assert!(!types_content.contains("export interface"));
}

#[test]
fn test_primitive_arrays_and_optional_custom_types() {
    // Regression test for issue where Vec<String> became types.string[]
    // and Option<CustomType> didn't get types. prefix
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = vec![
        CommandInfo {
            name: "get_dates".to_string(),
            file_path: "test_file.rs".to_string(),
            line_number: 10,
            parameters: vec![],
            return_type: "Vec<String>".to_string(),
            return_type_ts: "string[]".to_string(), // Already converted from Vec<String>
            is_async: true,
            channels: vec![],
        },
        CommandInfo {
            name: "get_user".to_string(),
            file_path: "test_file.rs".to_string(),
            line_number: 20,
            parameters: vec![],
            return_type: "Option<User>".to_string(),
            return_type_ts: "User | null".to_string(), // Already converted from Option<User>
            is_async: true,
            channels: vec![],
        },
        CommandInfo {
            name: "get_items".to_string(),
            file_path: "test_file.rs".to_string(),
            line_number: 30,
            parameters: vec![],
            return_type: "Vec<Item>".to_string(),
            return_type_ts: "Item[]".to_string(), // Already converted from Vec<Item>
            is_async: true,
            channels: vec![],
        },
    ];

    let mut discovered_structs = HashMap::new();
    discovered_structs.insert(
        "User".to_string(),
        StructInfo {
            name: "User".to_string(),
            fields: vec![],
            is_enum: false,
            file_path: "test_file.rs".to_string(),
        },
    );
    discovered_structs.insert(
        "Item".to_string(),
        StructInfo {
            name: "Item".to_string(),
            fields: vec![],
            is_enum: false,
            file_path: "test_file.rs".to_string(),
        },
    );

    let mut generator = BindingsGenerator::new(Some("zod".to_string()));
    generator
        .generate_models(
            &commands,
            &discovered_structs,
            output_path,
            &CommandAnalyzer::new(),
        )
        .unwrap();

    let commands_content = fs::read_to_string(temp_dir.path().join("commands.ts")).unwrap();

    // Primitive arrays should NOT have types. prefix
    assert!(commands_content.contains("Promise<string[]>"));
    assert!(!commands_content.contains("types.string[]"));

    // Custom types should have types. prefix
    assert!(commands_content.contains("Promise<types.User | null>"));
    assert!(!commands_content.contains("Promise<User | null>")); // Raw User should not appear

    // Arrays of custom types should have types. prefix on the base type
    assert!(commands_content.contains("Promise<types.Item[]>"));
    assert!(!commands_content.contains("Promise<Item[]>")); // Raw Item should not appear
}
