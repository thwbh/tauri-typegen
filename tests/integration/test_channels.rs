//! Integration tests for Tauri IPC channels
//! Tests Channel<T> parameter discovery and TypeScript generation

use crate::common;
use crate::fixtures;

use common::{TestGenerator, TestProject};

#[test]
fn test_simple_channel_discovery() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::channels::SIMPLE_CHANNEL);

    let (analyzer, commands) = project.analyze();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].channels.len(), 1);

    let channel = &commands[0].channels[0];
    assert_eq!(channel.parameter_name, "channel");
    assert_eq!(channel.message_type, "String");
}

#[test]
fn test_channel_with_parameters() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::channels::CHANNEL_WITH_PARAMETERS);

    let (analyzer, commands) = project.analyze();
    assert_eq!(commands[0].parameters.len(), 1); // url parameter
    assert_eq!(commands[0].channels.len(), 1); // progress channel

    let generator = TestGenerator::new();
    generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("none"),
        None,
    );

    let types = generator.read_file("types.ts");
    // Should have both regular parameter and channel
    assert!(types.contains("url: string"));
    assert!(types.contains("progress: Channel<number>"));
}

#[test]
fn test_multiple_channels() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::channels::MULTIPLE_CHANNELS);

    let (analyzer, commands) = project.analyze();
    assert_eq!(commands[0].channels.len(), 2);

    let generator = TestGenerator::new();
    generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("none"),
        None,
    );

    let types = generator.read_file("types.ts");
    assert!(types.contains("progress: Channel<number>"));
    assert!(types.contains("logs: Channel<string>"));
}

#[test]
fn test_channel_with_custom_type() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::channels::CHANNEL_WITH_CUSTOM_TYPE);

    let (analyzer, commands) = project.analyze();
    let generator = TestGenerator::new();
    generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("none"),
        None,
    );

    let types = generator.read_file("types.ts");
    // Should define the custom type
    assert!(types.contains("export interface ProgressUpdate"));
    assert!(types.contains("current: number"));
    assert!(types.contains("total: number"));
    assert!(types.contains("message: string"));

    // Should use it in the channel type
    assert!(types.contains("progress: Channel<ProgressUpdate>"));
}

#[test]
fn test_channel_with_serde_rename() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::channels::CHANNEL_WITH_SERDE_RENAME);

    let (analyzer, commands) = project.analyze();
    let generator = TestGenerator::new();
    generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("none"),
        None,
    );

    let types = generator.read_file("types.ts");
    // Channel parameter should respect serde rename
    assert!(types.contains("progressChannel: Channel<number>"));
    // Regular parameter should respect rename_all
    assert!(types.contains("taskId: string"));
}

#[test]
fn test_channel_imports_typescript() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::channels::SIMPLE_CHANNEL);

    let (analyzer, commands) = project.analyze();
    let generator = TestGenerator::new();
    generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("none"),
        None,
    );

    let types = generator.read_file("types.ts");
    // Should import Channel type from @tauri-apps/api
    assert!(types.contains("import type { Channel } from '@tauri-apps/api/core'"));
}

#[test]
fn test_channel_imports_zod() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::channels::SIMPLE_CHANNEL);

    let (analyzer, commands) = project.analyze();
    let generator = TestGenerator::new();
    generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("zod"),
        None,
    );

    let types = generator.read_file("types.ts");
    // Zod should also import Channel type
    assert!(types.contains("import type { Channel } from '@tauri-apps/api/core'"));
}
