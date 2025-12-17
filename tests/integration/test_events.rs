//! Integration tests for Tauri event discovery and generation
//! Tests app.emit() detection and TypeScript event listener generation

use crate::common;
use crate::fixtures;

use common::{TestGenerator, TestProject};

#[test]
fn test_simple_event_discovery() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::events::SIMPLE_EVENT);

    let (analyzer, _commands) = project.analyze();
    let events = analyzer.get_discovered_events();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_name, "update");
    assert_eq!(events[0].payload_type, "String");
}

#[test]
fn test_event_with_custom_type() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::events::EVENT_WITH_CUSTOM_TYPE);

    let (analyzer, commands) = project.analyze();
    let events = analyzer.get_discovered_events();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_name, "status-update");
    assert_eq!(events[0].payload_type, "StatusUpdate");

    let generator = TestGenerator::new();
    generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("none"),
        None,
    );

    // Should generate events.ts file
    assert!(generator.file_exists("events.ts"));

    let events_file = generator.read_file("events.ts");

    // Should define event listener function
    assert!(events_file.contains("export function onStatusUpdate"));

    // Should use custom type
    let types = generator.read_file("types.ts");
    assert!(types.contains("export interface StatusUpdate"));
}

#[test]
fn test_multiple_events() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::events::MULTIPLE_EVENTS);

    let (analyzer, _commands) = project.analyze();
    let events = analyzer.get_discovered_events();

    assert_eq!(events.len(), 3);

    // Check all events discovered
    let event_names: Vec<&str> = events.iter().map(|e| e.event_name.as_str()).collect();
    assert!(event_names.contains(&"started"));
    assert!(event_names.contains(&"progress"));
    assert!(event_names.contains(&"completed"));
}

#[test]
fn test_event_function_naming() {
    let project = TestProject::new();
    project.write_file(
        "main.rs",
        r#"
        use tauri::Manager;
        
        pub fn emit_events(app: &tauri::AppHandle) {
            app.emit("user-login", "data").ok();
            app.emit("data_update", "data").ok();
            app.emit("status.changed", "data").ok();
        }
    "#,
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

    let events_file = generator.read_file("events.ts");

    // Event names should be converted to camelCase function names
    assert!(events_file.contains("export function onUserLogin"));
    assert!(events_file.contains("export function onDataUpdate"));
    assert!(events_file.contains("export function onStatusChanged"));
}

#[test]
fn test_event_with_serde_rename() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::events::EVENT_WITH_SERDE_RENAME);

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

    // Event payload type should respect serde attributes
    assert!(types.contains("export interface UserEvent"));
    assert!(types.contains("userId: string"));
    assert!(types.contains("eventType: string"));
}

#[test]
fn test_no_events_no_file() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::basic_commands::SIMPLE_COMMAND);

    let (analyzer, commands) = project.analyze();
    let events = analyzer.get_discovered_events();

    assert_eq!(events.len(), 0);

    let generator = TestGenerator::new();
    generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("none"),
        None,
    );

    // Should NOT generate events.ts if no events
    assert!(!generator.file_exists("events.ts"));
}

#[test]
fn test_events_index_export() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::events::SIMPLE_EVENT);

    let (analyzer, commands) = project.analyze();
    let generator = TestGenerator::new();
    generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("none"),
        None,
    );

    let index = generator.read_file("index.ts");

    // index.ts should export from events.ts
    assert!(index.contains("export * from './events'"));
}

#[test]
fn test_events_with_zod() {
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::events::EVENT_WITH_CUSTOM_TYPE);

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

    // Event payload type should have Zod schema
    assert!(types.contains("export const StatusUpdateSchema"));
    assert!(types.contains("status: z.string()"));
    assert!(types.contains("progress: z.number()"));
}
