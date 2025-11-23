use std::collections::HashMap;
use std::fs;
use tauri_typegen::analysis::CommandAnalyzer;
use tauri_typegen::generators::generator::BindingsGenerator;
use tauri_typegen::models::{ChannelInfo, CommandInfo, ParameterInfo};
use tempfile::TempDir;

/// Test for issue: Commands with both regular parameters and channels should generate
/// valid TypeScript code that explicitly references channel parameters by name,
/// not use a non-existent `extractChannels` function.
#[test]
fn test_zod_command_with_params_and_channels_generates_valid_code() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    // Create a command with both regular parameters and a channel
    let commands = vec![CommandInfo {
        name: "download_file".to_string(),
        file_path: "test_file.rs".to_string(),
        line_number: 10,
        parameters: vec![
            ParameterInfo {
                name: "url".to_string(),
                rust_type: "String".to_string(),
                typescript_type: "string".to_string(),
                is_optional: false,
            },
            ParameterInfo {
                name: "timeout".to_string(),
                rust_type: "i32".to_string(),
                typescript_type: "number".to_string(),
                is_optional: false,
            },
        ],
        return_type: "string".to_string(),
        return_type_ts: "string".to_string(),
        is_async: true,
        channels: vec![ChannelInfo {
            parameter_name: "on_progress".to_string(),
            message_type: "DownloadProgress".to_string(),
            typescript_message_type: "DownloadProgress".to_string(),
            command_name: "download_file".to_string(),
            file_path: "test_file.rs".to_string(),
            line_number: 10,
        }],
    }];

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

    // Should NOT contain the non-existent extractChannels function
    assert!(
        !commands_content.contains("extractChannels"),
        "Generated code should not reference non-existent extractChannels function"
    );

    // Should contain explicit channel parameter reference
    assert!(
        commands_content.contains("onProgress: params.onProgress"),
        "Generated code should explicitly reference channel parameter by name"
    );

    // Should spread result.data for validated parameters
    assert!(
        commands_content.contains("...result.data"),
        "Generated code should spread validated result.data"
    );
}

#[test]
fn test_zod_command_with_multiple_channels_generates_all_references() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    // Create a command with regular parameters and multiple channels
    let commands = vec![CommandInfo {
        name: "complex_operation".to_string(),
        file_path: "test_file.rs".to_string(),
        line_number: 20,
        parameters: vec![ParameterInfo {
            name: "config".to_string(),
            rust_type: "String".to_string(),
            typescript_type: "string".to_string(),
            is_optional: false,
        }],
        return_type: "void".to_string(),
        return_type_ts: "void".to_string(),
        is_async: true,
        channels: vec![
            ChannelInfo {
                parameter_name: "on_progress".to_string(),
                message_type: "Progress".to_string(),
                typescript_message_type: "Progress".to_string(),
                command_name: "complex_operation".to_string(),
                file_path: "test_file.rs".to_string(),
                line_number: 20,
            },
            ChannelInfo {
                parameter_name: "on_log".to_string(),
                message_type: "LogMessage".to_string(),
                typescript_message_type: "LogMessage".to_string(),
                command_name: "complex_operation".to_string(),
                file_path: "test_file.rs".to_string(),
                line_number: 20,
            },
        ],
    }];

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

    // Should contain explicit references for both channels
    assert!(
        commands_content.contains("onProgress: params.onProgress"),
        "Generated code should reference onProgress channel"
    );
    assert!(
        commands_content.contains("onLog: params.onLog"),
        "Generated code should reference onLog channel"
    );

    // Should NOT contain extractChannels
    assert!(
        !commands_content.contains("extractChannels"),
        "Generated code should not reference non-existent extractChannels function"
    );
}

#[test]
fn test_zod_command_with_only_params_no_channels() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    // Create a command with only regular parameters, no channels
    let commands = vec![CommandInfo {
        name: "simple_command".to_string(),
        file_path: "test_file.rs".to_string(),
        line_number: 30,
        parameters: vec![ParameterInfo {
            name: "data".to_string(),
            rust_type: "String".to_string(),
            typescript_type: "string".to_string(),
            is_optional: false,
        }],
        return_type: "string".to_string(),
        return_type_ts: "string".to_string(),
        is_async: true,
        channels: vec![],
    }];

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

    // Should just use result.data directly without spreading channels
    assert!(
        commands_content.contains("result.data"),
        "Generated code should pass result.data to invoke"
    );

    // Should NOT contain extractChannels or channel spreading
    assert!(
        !commands_content.contains("extractChannels"),
        "Generated code should not reference extractChannels"
    );
}

#[test]
fn test_zod_command_with_only_channels_no_params() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    // Create a command with only channels, no regular parameters
    let commands = vec![CommandInfo {
        name: "stream_only".to_string(),
        file_path: "test_file.rs".to_string(),
        line_number: 40,
        parameters: vec![],
        return_type: "void".to_string(),
        return_type_ts: "void".to_string(),
        is_async: true,
        channels: vec![ChannelInfo {
            parameter_name: "on_data".to_string(),
            message_type: "StreamData".to_string(),
            typescript_message_type: "StreamData".to_string(),
            command_name: "stream_only".to_string(),
            file_path: "test_file.rs".to_string(),
            line_number: 40,
        }],
    }];

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

    // Should pass params directly since there's no validation needed
    assert!(
        commands_content.contains("'stream_only', params"),
        "Generated code should pass params directly for channel-only commands"
    );

    // Should NOT contain extractChannels
    assert!(
        !commands_content.contains("extractChannels"),
        "Generated code should not reference extractChannels"
    );
}

#[test]
fn test_vanilla_ts_command_with_params_and_channels() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    // Create a command with both regular parameters and a channel
    let commands = vec![CommandInfo {
        name: "download_file".to_string(),
        file_path: "test_file.rs".to_string(),
        line_number: 10,
        parameters: vec![ParameterInfo {
            name: "url".to_string(),
            rust_type: "String".to_string(),
            typescript_type: "string".to_string(),
            is_optional: false,
        }],
        return_type: "string".to_string(),
        return_type_ts: "string".to_string(),
        is_async: true,
        channels: vec![ChannelInfo {
            parameter_name: "on_progress".to_string(),
            message_type: "DownloadProgress".to_string(),
            typescript_message_type: "DownloadProgress".to_string(),
            command_name: "download_file".to_string(),
            file_path: "test_file.rs".to_string(),
            line_number: 10,
        }],
    }];

    let discovered_structs = HashMap::new();

    // Use vanilla TypeScript (no validation)
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

    // Should NOT contain extractChannels
    assert!(
        !commands_content.contains("extractChannels"),
        "Generated code should not reference extractChannels"
    );

    // Should pass params directly
    assert!(
        commands_content.contains("'download_file', params"),
        "Generated code should pass params to invoke"
    );
}
