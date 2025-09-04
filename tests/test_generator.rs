use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;
use tauri_plugin_typegen::generator::TypeScriptGenerator;
use tauri_plugin_typegen::models::{CommandInfo, ParameterInfo, StructInfo};

fn create_sample_commands() -> Vec<CommandInfo> {
    vec![
        CommandInfo {
            name: "greet".to_string(),
            file_path: "test_file.rs".to_string(),
            line_number: 10,
            parameters: vec![
                ParameterInfo {
                    name: "name".to_string(),
                    rust_type: "String".to_string(),
                    typescript_type: "string".to_string(),
                    is_optional: false,
                },
            ],
            return_type: "string".to_string(),
            is_async: true,
        },
        CommandInfo {
            name: "get_user_count".to_string(),
            file_path: "test_file.rs".to_string(),
            line_number: 15,
            parameters: vec![],
            return_type: "number".to_string(),
            is_async: false,
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

    let mut generator = TypeScriptGenerator::new(Some("zod".to_string()));
    let generated_files = generator.generate_models(&commands, &discovered_structs, output_path).unwrap();

    assert_eq!(generated_files.len(), 4);
    assert!(generated_files.contains(&"types.ts".to_string()));
    assert!(generated_files.contains(&"schemas.ts".to_string()));
    assert!(generated_files.contains(&"commands.ts".to_string()));
    assert!(generated_files.contains(&"index.ts".to_string()));

    // Verify files exist
    assert!(temp_dir.path().join("types.ts").exists());
    assert!(temp_dir.path().join("schemas.ts").exists());
    assert!(temp_dir.path().join("commands.ts").exists());
    assert!(temp_dir.path().join("index.ts").exists());
}

#[test]
fn test_generator_without_validation_library() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = create_sample_commands();
    let discovered_structs = create_empty_structs();

    let mut generator = TypeScriptGenerator::new(Some("none".to_string()));
    let generated_files = generator.generate_models(&commands, &discovered_structs, output_path).unwrap();

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

    let mut generator = TypeScriptGenerator::new(None);
    generator.generate_models(&commands, &discovered_structs, output_path).unwrap();

    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();

    // Should contain parameter interfaces for commands with parameters
    assert!(types_content.contains("export interface GreetParams"));
    assert!(types_content.contains("name: string;"));

    // Should NOT contain params interface for commands without parameters
    assert!(!types_content.contains("GetUserCountParams"));
}

#[test]
fn test_zod_schemas_generation() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = create_sample_commands();
    let discovered_structs = create_empty_structs();

    let mut generator = TypeScriptGenerator::new(Some("zod".to_string()));
    generator.generate_models(&commands, &discovered_structs, output_path).unwrap();

    let schemas_content = fs::read_to_string(temp_dir.path().join("schemas.ts")).unwrap();

    assert!(schemas_content.contains("import { z } from 'zod';"));
    assert!(schemas_content.contains("GreetParamsSchema"));
    assert!(schemas_content.contains("z.object({"));
    assert!(schemas_content.contains("name: z.string()"));

    // Should not generate schema for commands without parameters
    assert!(!schemas_content.contains("GetUserCountParamsSchema"));
}

#[test]
fn test_yup_schemas_generation() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = create_sample_commands();
    let discovered_structs = create_empty_structs();

    let mut generator = TypeScriptGenerator::new(Some("yup".to_string()));
    generator.generate_models(&commands, &discovered_structs, output_path).unwrap();

    let schemas_content = fs::read_to_string(temp_dir.path().join("schemas.ts")).unwrap();

    assert!(schemas_content.contains("import * as yup from 'yup';"));
    assert!(schemas_content.contains("GreetParamsSchema"));
    assert!(schemas_content.contains("yup.object({"));
    assert!(schemas_content.contains("name: yup.string()"));
}

#[test]
fn test_commands_file_generation() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = create_sample_commands();
    let discovered_structs = create_empty_structs();

    let mut generator = TypeScriptGenerator::new(Some("zod".to_string()));
    generator.generate_models(&commands, &discovered_structs, output_path).unwrap();

    let commands_content = fs::read_to_string(temp_dir.path().join("commands.ts")).unwrap();

    assert!(commands_content.contains("import { invoke } from '@tauri-apps/api/core';"));
    assert!(commands_content.contains("import * as schemas from './schemas';"));
    assert!(commands_content.contains("import type * as types from './types';"));

    // Check specific command functions
    assert!(commands_content.contains("export async function greet"));
    assert!(commands_content.contains("params: types.GreetParams"));
    assert!(commands_content.contains("Promise<string>"));
    assert!(commands_content.contains("schemas.GreetParamsSchema.parse(params)"));
    assert!(commands_content.contains("invoke('greet'"));

    // Check command without parameters
    assert!(commands_content.contains("export async function getUserCount(): Promise<number>"));
    assert!(commands_content.contains("return invoke('get_user_count');"));
}

#[test]
fn test_commands_without_validation() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = create_sample_commands();
    let discovered_structs = create_empty_structs();

    let mut generator = TypeScriptGenerator::new(Some("none".to_string()));
    generator.generate_models(&commands, &discovered_structs, output_path).unwrap();

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

    let mut generator = TypeScriptGenerator::new(None);
    generator.generate_models(&commands, &discovered_structs, output_path).unwrap();

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();

    assert!(index_content.contains("export * from './types';"));
    assert!(index_content.contains("export * from './schemas';"));
    assert!(index_content.contains("export * from './commands';"));
}

#[test]
fn test_pascal_case_conversion() {
    let generator = TypeScriptGenerator::new(None);

    assert_eq!(generator.to_pascal_case("hello_world"), "HelloWorld");
    assert_eq!(generator.to_pascal_case("get_user_count"), "GetUserCount");
    assert_eq!(generator.to_pascal_case("simple"), "Simple");
    assert_eq!(generator.to_pascal_case(""), "");
}

#[test]
fn test_typescript_to_zod_type_conversion() {
    let generator = TypeScriptGenerator::new(None);

    assert_eq!(generator.typescript_to_zod_type("string"), "z.string()");
    assert_eq!(generator.typescript_to_zod_type("number"), "z.number()");
    assert_eq!(generator.typescript_to_zod_type("boolean"), "z.boolean()");
    assert_eq!(generator.typescript_to_zod_type("string[]"), "z.array(z.string())");
    assert_eq!(generator.typescript_to_zod_type("string | null"), "z.string().nullable()");
    assert_eq!(generator.typescript_to_zod_type("CustomType"), "z.any()");
}

#[test]
fn test_typescript_to_yup_type_conversion() {
    let generator = TypeScriptGenerator::new(None);

    assert_eq!(generator.typescript_to_yup_type("string"), "yup.string()");
    assert_eq!(generator.typescript_to_yup_type("number"), "yup.number()");
    assert_eq!(generator.typescript_to_yup_type("boolean"), "yup.boolean()");
    assert_eq!(generator.typescript_to_yup_type("string[]"), "yup.array().of(yup.string())");
    assert_eq!(generator.typescript_to_yup_type("string | null"), "yup.string().nullable()");
    assert_eq!(generator.typescript_to_yup_type("CustomType"), "yup.mixed()");
}

#[test]
fn test_custom_type_detection() {
    let generator = TypeScriptGenerator::new(None);

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

    let commands = vec![
        CommandInfo {
            name: "delete_item".to_string(),
            file_path: "test_file.rs".to_string(),
            line_number: 10,
            parameters: vec![
                ParameterInfo {
                    name: "id".to_string(),
                    rust_type: "i32".to_string(),
                    typescript_type: "number".to_string(),
                    is_optional: false,
                },
            ],
            return_type: "void".to_string(),
            is_async: true,
        },
    ];
    let discovered_structs = create_empty_structs();

    let mut generator = TypeScriptGenerator::new(None);
    generator.generate_models(&commands, &discovered_structs, output_path).unwrap();

    let commands_content = fs::read_to_string(temp_dir.path().join("commands.ts")).unwrap();
    assert!(commands_content.contains("Promise<void>"));
}

#[test]
fn test_generator_empty_commands_list() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = vec![];
    let discovered_structs = create_empty_structs();

    let mut generator = TypeScriptGenerator::new(None);
    let generated_files = generator.generate_models(&commands, &discovered_structs, output_path).unwrap();

    // Should still generate files, just with empty content
    assert_eq!(generated_files.len(), 4);

    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();
    // Should contain header but no interfaces
    assert!(types_content.contains("Auto-generated TypeScript types"));
    assert!(!types_content.contains("export interface"));
}