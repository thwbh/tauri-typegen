use std::collections::HashMap;
use std::fs;
use tauri_typegen::analysis::CommandAnalyzer;
use tauri_typegen::generators::generator::BindingsGenerator;
use tauri_typegen::models::{CommandInfo, ParameterInfo};
use tempfile::TempDir;

fn create_test_command() -> CommandInfo {
    CommandInfo {
        name: "test_command".to_string(),
        file_path: "test.rs".to_string(),
        line_number: 1,
        parameters: vec![ParameterInfo {
            name: "input".to_string(),
            rust_type: "String".to_string(),
            typescript_type: "string".to_string(),
            is_optional: false,
            type_structure: Default::default(),
        }],
        return_type: "String".to_string(),
        return_type_ts: "string".to_string(),
        is_async: true,
        channels: vec![],
    }
}

fn create_command_without_params() -> CommandInfo {
    CommandInfo {
        name: "simple_command".to_string(),
        file_path: "test.rs".to_string(),
        line_number: 1,
        parameters: vec![],
        return_type: "()".to_string(),
        return_type_ts: "void".to_string(),
        is_async: true,
        channels: vec![],
    }
}

#[test]
fn test_command_hooks_interface_generated() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = vec![create_test_command()];
    let discovered_structs = HashMap::new();

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

    // Verify CommandHooks interface is present
    assert!(commands_content.contains("export interface CommandHooks<T>"));
    assert!(commands_content.contains("onValidationError?: (error: ZodError) => void"));
    assert!(commands_content.contains("onInvokeError?: (error: unknown) => void"));
    assert!(commands_content.contains("onSuccess?: (result: T) => void"));
    assert!(commands_content.contains("onSettled?: () => void"));
}

#[test]
fn test_command_with_hooks_parameter() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = vec![create_test_command()];
    let discovered_structs = HashMap::new();

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

    // Verify hooks parameter is optional
    assert!(commands_content.contains("hooks?: CommandHooks<string>"));

    // Verify function signature includes hooks
    assert!(
        commands_content.contains("params: types.TestCommandParams, hooks?: CommandHooks<string>")
    );
}

#[test]
fn test_command_without_params_has_hooks() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = vec![create_command_without_params()];
    let discovered_structs = HashMap::new();

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

    // Commands without params should still have hooks parameter
    assert!(commands_content.contains("hooks?: CommandHooks<void>"));
    assert!(commands_content.contains("function simpleCommand(hooks?: CommandHooks<void>)"));
}

#[test]
fn test_hooks_implementation_structure() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = vec![create_test_command()];
    let discovered_structs = HashMap::new();

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

    // Verify try-catch-finally structure
    assert!(commands_content.contains("try {"));
    assert!(commands_content.contains("} catch (error) {"));
    assert!(commands_content.contains("} finally {"));

    // Verify hook calls
    assert!(commands_content.contains("hooks?.onValidationError?.(result.error)"));
    assert!(commands_content.contains("hooks?.onInvokeError?.(error)"));
    assert!(commands_content.contains("hooks?.onSuccess?.(data)"));
    assert!(commands_content.contains("hooks?.onSettled?.()"));

    // Verify errors are still thrown
    assert!(commands_content.contains("throw result.error"));
    assert!(commands_content.contains("throw error"));
}

#[test]
fn test_zod_error_import_present() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = vec![create_test_command()];
    let discovered_structs = HashMap::new();

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

    // Verify ZodError is imported
    assert!(commands_content.contains("import { ZodError } from 'zod'"));

    // Verify ZodError check in error handling
    assert!(commands_content.contains("if (!(error instanceof ZodError))"));
}

#[test]
fn test_safe_parse_used_instead_of_parse() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = vec![create_test_command()];
    let discovered_structs = HashMap::new();

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

    // Verify safeParse is used (not parse)
    assert!(commands_content.contains(".safeParse(params)"));
    assert!(commands_content.contains("if (!result.success)"));

    // Should not use .parse() directly anymore
    let parse_count = commands_content.matches(".parse(").count();
    assert_eq!(parse_count, 0, "Should use safeParse instead of parse");
}

#[test]
fn test_backward_compatibility_with_vanilla_typescript() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    let commands = vec![create_test_command()];
    let discovered_structs = HashMap::new();

    // Generate without validation (vanilla TypeScript)
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

    // Vanilla TS should NOT have hooks (not part of this feature)
    assert!(!commands_content.contains("CommandHooks"));
    assert!(!commands_content.contains("hooks?:"));
}
