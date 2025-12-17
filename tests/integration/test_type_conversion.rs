//! Integration tests for complex type conversions
//! Tests Rust types → TypeScript type mappings

use crate::common;
use crate::fixtures;

use common::{TestGenerator, TestProject};

#[test]
fn test_nested_struct_types() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::complex_types::NESTED_STRUCT);

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

    // Should define both structs
    assert!(types.contains("export interface Address"));
    assert!(types.contains("export interface User"));

    // Address fields
    assert!(types.contains("street: string"));
    assert!(types.contains("city: string"));
    assert!(types.contains("zip: string"));

    // User with nested Address
    assert!(types.contains("id: string"));
    assert!(types.contains("name: string"));
    assert!(types.contains("address: Address"));
}

#[test]
fn test_collection_types() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::complex_types::COLLECTIONS);

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

    // Vec<T> → Array<T>
    assert!(types.contains("Array<string>") || types.contains("string[]"));

    // HashMap<K, V> → Record<K, V>
    assert!(types.contains("Record<string, string>"));

    // HashSet<T> → Set<T>
    assert!(types.contains("Set<string>"));
}

#[test]
fn test_tuple_types() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::complex_types::TUPLES);

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

    // (f64, f64) → [number, number]
    assert!(types.contains("[number, number]"));

    // (String, u32, bool) → [string, number, boolean]
    assert!(types.contains("[string, number, boolean]"));
}

#[test]
fn test_result_type_conversion() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::complex_types::GENERIC_RESULT);

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

    // Result<T, E> → T (error type ignored in TypeScript)
    assert!(types.contains("string")); // Return type is string, not Result

    // Error struct should still be defined if referenced elsewhere
    assert!(types.contains("export interface ApiError"));
}

#[test]
fn test_nested_collections() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::complex_types::NESTED_COLLECTIONS);

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

    // HashMap<String, Vec<String>> → Record<string, string[]>
    assert!(
        types.contains("Record<string, Array<string>>")
            || types.contains("Record<string, string[]>")
    );

    // Vec<Vec<i32>> → number[][]
    assert!(types.contains("Array<Array<number>>") || types.contains("number[][]"));
}

#[test]
fn test_option_types() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::complex_types::OPTION_TYPES);

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

    // Struct with optional fields
    assert!(types.contains("export interface Profile"));
    assert!(types.contains("name: string"));
    assert!(types.contains("email?: string"));
    assert!(types.contains("age?: number"));

    // Option<T> as return type → T | null
    let commands_file = generator.read_file("commands.ts");
    assert!(
        commands_file.contains("Promise<Profile | null>")
            || commands_file.contains("Promise<Profile | undefined>")
    );
}

#[test]
fn test_zod_complex_types() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::complex_types::NESTED_STRUCT);

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

    // Should generate schemas for nested types
    assert!(types.contains("export const AddressSchema"));
    assert!(types.contains("export const UserSchema"));

    // Address schema
    assert!(types.contains("street: z.string()"));
    assert!(types.contains("city: z.string()"));

    // User schema with nested reference
    assert!(types.contains("address: AddressSchema"));
}

#[test]
fn test_zod_collection_schemas() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::complex_types::COLLECTIONS);

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

    // Vec<T> → z.array()
    assert!(types.contains("z.array(z.string())"));

    // HashMap<K, V> → z.record()
    assert!(types.contains("z.record(z.string(), z.string())"));
}

#[test]
fn test_zod_optional_schemas() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::complex_types::OPTION_TYPES);

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

    // Optional fields → .optional()
    assert!(types.contains("email: z.string().optional()"));
    assert!(types.contains("age: z.number().optional()"));
}
