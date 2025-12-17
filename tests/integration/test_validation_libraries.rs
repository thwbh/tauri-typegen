//! Integration tests for validation library support (Zod)
//! Tests Zod schema generation and validation

use crate::common;
use crate::fixtures;

use common::{TestGenerator, TestProject};

#[test]
fn test_zod_generates_schemas() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::basic_commands::SIMPLE_COMMAND);

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
    // Should import zod
    assert!(types.contains("import { z } from 'zod'"));

    // Should generate schema
    assert!(types.contains("export const GreetParamsSchema"));
    assert!(types.contains("z.object"));

    // Should generate type from schema
    assert!(types.contains("export type GreetParams = z.infer<typeof GreetParamsSchema>"));
}

#[test]
fn test_zod_optional_fields() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::basic_commands::OPTIONAL_PARAMETERS);

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
    // Required field
    assert!(types.contains("query: z.string()"));
    // Optional field
    assert!(types.contains("limit: z.number().optional()"));
}

#[test]
fn test_zod_commands_use_validation() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::basic_commands::SIMPLE_COMMAND);

    let (analyzer, commands) = project.analyze();
    let generator = TestGenerator::new();
    generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("zod"),
        None,
    );

    let commands_file = generator.read_file("commands.ts");
    // Should validate params using schema
    assert!(commands_file.contains("GreetParamsSchema.parse"));
}

#[test]
fn test_zod_struct_schemas() {
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
        Some("zod"),
        None,
    );

    let types = generator.read_file("types.ts");
    // Should generate schema for struct
    assert!(types.contains("export const UserProfileSchema"));
    assert!(types.contains("userId: z.string()"));
    assert!(types.contains("firstName: z.string()"));
}

#[test]
fn test_zod_enum_schemas() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::serde_attributes::ENUM_WITH_RENAME);

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
    // Should generate enum schema
    assert!(types.contains("export const StatusSchema = z.enum"));
    assert!(types.contains("\"ACTIVE\""));
    assert!(types.contains("\"PENDING\""));
}

#[test]
fn test_vanilla_typescript_no_zod() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::basic_commands::SIMPLE_COMMAND);

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
    // Should NOT import zod
    assert!(!types.contains("import { z } from 'zod'"));

    // Should use interface, not schema
    assert!(types.contains("export interface GreetParams"));
    assert!(!types.contains("GreetParamsSchema"));

    let commands_file = generator.read_file("commands.ts");
    // Should NOT validate
    assert!(!commands_file.contains(".parse"));
}
