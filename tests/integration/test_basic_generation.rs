//! Integration tests for basic TypeScript generation
//! Tests the complete pipeline from Rust code to TypeScript output

use crate::common;
use crate::fixtures;

use common::{TestGenerator, TestProject};

#[test]
fn test_simple_command_generates_typescript() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::basic_commands::SIMPLE_COMMAND);

    let (analyzer, commands) = project.analyze();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].name, "greet");

    let generator = TestGenerator::new();
    let files = generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("none"),
        None,
    );

    assert!(files.contains(&"types.ts".to_string()));
    assert!(files.contains(&"commands.ts".to_string()));

    let types = generator.read_file("types.ts");
    assert!(types.contains("export interface GreetParams"));
    assert!(types.contains("name: string"));

    let commands_file = generator.read_file("commands.ts");
    assert!(commands_file.contains("export async function greet"));
}

#[test]
fn test_multiple_parameters_command() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::basic_commands::MULTIPLE_PARAMETERS);

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
    assert!(types.contains("x: number"));
    assert!(types.contains("y: number"));
    assert!(types.contains("operation: string"));
}

#[test]
fn test_optional_parameters() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::basic_commands::OPTIONAL_PARAMETERS);

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
    assert!(types.contains("query: string"));
    assert!(types.contains("limit?: number"));
}

#[test]
fn test_async_command() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::basic_commands::ASYNC_COMMAND);

    let (analyzer, commands) = project.analyze();
    assert_eq!(commands[0].is_async, true);

    let generator = TestGenerator::new();
    generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("none"),
        None,
    );

    let commands_file = generator.read_file("commands.ts");
    assert!(commands_file.contains("export async function fetchData"));
}

#[test]
fn test_no_parameters_command() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::basic_commands::NO_PARAMETERS);

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
    // Should not generate a params interface for commands with no parameters
    assert!(!types.contains("GetVersionParams"));
}

#[test]
fn test_index_file_generation() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::basic_commands::SIMPLE_COMMAND);

    let (analyzer, commands) = project.analyze();
    let generator = TestGenerator::new();
    let files = generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("none"),
        None,
    );

    assert!(files.contains(&"index.ts".to_string()));

    let index = generator.read_file("index.ts");
    assert!(index.contains("export * from './types'"));
    assert!(index.contains("export * from './commands'"));
}
