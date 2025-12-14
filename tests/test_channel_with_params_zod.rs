use std::collections::HashMap;
use std::fs;
use tauri_typegen::analysis::CommandAnalyzer;
use tauri_typegen::generators::create_generator;
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
    let commands = vec![CommandInfo::new_for_test(
        "download_file",
        "test_file.rs",
        10,
        vec![
            ParameterInfo {
                name: "url".to_string(),
                rust_type: "String".to_string(),
                is_optional: false,
                type_structure: Default::default(),
                serde_rename: None,
            },
            ParameterInfo {
                name: "timeout".to_string(),
                rust_type: "i32".to_string(),
                is_optional: false,
                type_structure: Default::default(),
                serde_rename: None,
            },
        ],
        "string",
        true,
        vec![ChannelInfo::new_for_test(
            "on_progress",
            "DownloadProgress",
            "download_file",
            "test_file.rs",
            10,
        )],
    )];

    let discovered_structs = HashMap::new();

    let mut generator = create_generator(Some("zod".to_string()));
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
    let commands = vec![CommandInfo::new_for_test(
        "complex_operation",
        "test_file.rs",
        20,
        vec![ParameterInfo {
            name: "config".to_string(),
            rust_type: "String".to_string(),
            is_optional: false,
            type_structure: Default::default(),
            serde_rename: None,
        }],
        "void",
        true,
        vec![
            ChannelInfo::new_for_test(
                "on_progress",
                "Progress",
                "complex_operation",
                "test_file.rs",
                20,
            ),
            ChannelInfo::new_for_test(
                "on_log",
                "LogMessage",
                "complex_operation",
                "test_file.rs",
                20,
            ),
        ],
    )];

    let discovered_structs = HashMap::new();

    let mut generator = create_generator(Some("zod".to_string()));
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
    let commands = vec![CommandInfo::new_for_test(
        "simple_command",
        "test_file.rs",
        30,
        vec![ParameterInfo {
            name: "data".to_string(),
            rust_type: "String".to_string(),
            is_optional: false,
            type_structure: Default::default(),
            serde_rename: None,
        }],
        "string",
        true,
        vec![],
    )];

    let discovered_structs = HashMap::new();

    let mut generator = create_generator(Some("zod".to_string()));
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
    let commands = vec![CommandInfo::new_for_test(
        "stream_only",
        "test_file.rs",
        40,
        vec![],
        "void",
        true,
        vec![ChannelInfo::new_for_test(
            "on_data",
            "StreamData",
            "stream_only",
            "test_file.rs",
            40,
        )],
    )];

    let discovered_structs = HashMap::new();

    let mut generator = create_generator(Some("zod".to_string()));
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
    let commands = vec![CommandInfo::new_for_test(
        "download_file",
        "test_file.rs",
        10,
        vec![ParameterInfo {
            name: "url".to_string(),
            rust_type: "String".to_string(),
            is_optional: false,
            type_structure: Default::default(),
            serde_rename: None,
        }],
        "string",
        true,
        vec![ChannelInfo::new_for_test(
            "on_progress",
            "DownloadProgress",
            "download_file",
            "test_file.rs",
            10,
        )],
    )];

    let discovered_structs = HashMap::new();

    // Use vanilla TypeScript (no validation)
    let mut generator = create_generator(Some("none".to_string()));
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
