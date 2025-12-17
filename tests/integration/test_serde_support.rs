//! Integration tests for serde attribute support
//! Tests #[serde(rename = "...")] and #[serde(rename_all = "...")]

use crate::common;
use crate::fixtures;

use common::{TestGenerator, TestProject};
use tauri_typegen::GenerateConfig;

#[test]
fn test_command_rename_all_snake_case() {
    let project = TestProject::new();
    project.write_file(
        "main.rs",
        fixtures::serde_attributes::COMMAND_WITH_RENAME_ALL,
    );

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
    // Command-level rename_all should affect parameters
    assert!(types.contains("user_id: string"));
    assert!(types.contains("full_name: string"));

    let commands_file = generator.read_file("commands.ts");
    // But NOT the function name - should still be camelCase
    assert!(commands_file.contains("export async function updateUserProfile"));
}

#[test]
fn test_parameter_level_rename() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::serde_attributes::PARAMETER_WITH_RENAME);

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
    // Parameter with explicit rename
    assert!(types.contains("id: string"));
    // Parameter without rename uses default (camelCase)
    assert!(types.contains("customerName: string"));
}

#[test]
fn test_struct_rename_all_camel_case() {
    let project = TestProject::new();
    project.write_file(
        "main.rs",
        fixtures::serde_attributes::STRUCT_WITH_RENAME_ALL,
    );

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
    assert!(types.contains("export interface UserProfile"));
    assert!(types.contains("userId: string"));
    assert!(types.contains("firstName: string"));
    assert!(types.contains("lastName: string"));
    assert!(types.contains("emailAddress: string"));
}

#[test]
fn test_field_level_rename() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::serde_attributes::FIELD_WITH_RENAME);

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
    assert!(types.contains("export interface Product"));
    // Field with explicit rename
    assert!(types.contains("productName: string"));
    // Other fields use default camelCase
    assert!(types.contains("id: string"));
    assert!(types.contains("price: number"));
}

#[test]
fn test_enum_rename_all() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::serde_attributes::ENUM_WITH_RENAME);

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
    assert!(types.contains("\"ACTIVE\""));
    assert!(types.contains("\"PENDING\""));
    assert!(types.contains("\"COMPLETED\""));
    assert!(types.contains("\"CANCELLED\""));
}

#[test]
fn test_enum_variant_rename() {
    let project = TestProject::new();
    project.write_file(
        "main.rs",
        fixtures::serde_attributes::ENUM_VARIANT_WITH_RENAME,
    );

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
    // Variants with explicit renames
    assert!(types.contains("\"new\""));
    assert!(types.contains("\"processing\""));
    assert!(types.contains("\"shipped\""));
}

#[test]
fn test_config_default_parameter_case() {
    let project = TestProject::new();
    project.write_file(
        "main.rs",
        r#"
        #[tauri::command]
        pub fn update_order(order_id: String, new_status: String) -> Result<String, String> {
            Ok("Updated".to_string())
        }
    "#,
    );

    let (analyzer, commands) = project.analyze();
    let generator = TestGenerator::new();

    // Use config with snake_case default
    let mut config = GenerateConfig::default();
    config.default_parameter_case = "snake_case".to_string();

    generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("none"),
        Some(&config),
    );

    let types = generator.read_file("types.ts");
    // Should use snake_case when no serde attributes present
    assert!(types.contains("order_id: string"));
    assert!(types.contains("new_status: string"));
}

#[test]
fn test_serde_attribute_overrides_config_default() {
    let project = TestProject::new();
    project.write_file(
        "main.rs",
        r#"
        #[tauri::command]
        #[serde(rename_all = "camelCase")]
        pub fn update_order(order_id: String, new_status: String) -> Result<String, String> {
            Ok("Updated".to_string())
        }
    "#,
    );

    let (analyzer, commands) = project.analyze();
    let generator = TestGenerator::new();

    // Even with snake_case default, serde attribute should win
    let mut config = GenerateConfig::default();
    config.default_parameter_case = "snake_case".to_string();

    generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("none"),
        Some(&config),
    );

    let types = generator.read_file("types.ts");
    // Should use camelCase from serde attribute, not config default
    assert!(types.contains("orderId: string"));
    assert!(types.contains("newStatus: string"));
}
