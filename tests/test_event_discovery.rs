use std::fs;
use tauri_typegen::analysis::CommandAnalyzer;
use tempfile::TempDir;

fn create_test_project(files: &[(&str, &str)]) -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    for (file_path, content) in files {
        let full_path = temp_dir.path().join(file_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full_path, content).unwrap();
    }

    temp_dir
}

#[test]
fn test_discovers_simple_emit() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::{AppHandle, Emitter};
            use serde::Serialize;

            #[derive(Clone, Serialize)]
            pub struct MyEvent {
                pub message: String,
            }

            #[tauri::command]
            pub fn my_command(app: AppHandle) {
                app.emit("my-event", MyEvent { message: "Hello".to_string() }).unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let _commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    let events = analyzer.get_discovered_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_name, "my-event");
    assert_eq!(events[0].payload_type, "MyEvent");
}

#[test]
fn test_discovers_multiple_events() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::{AppHandle, Emitter};
            use serde::Serialize;

            #[derive(Clone, Serialize)]
            pub struct Event1 {
                pub data: String,
            }

            #[derive(Clone, Serialize)]
            pub struct Event2 {
                pub count: i32,
            }

            #[tauri::command]
            pub fn my_command(app: AppHandle) {
                app.emit("event-1", Event1 { data: "test".to_string() }).unwrap();
                app.emit("event-2", Event2 { count: 42 }).unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let _commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    let events = analyzer.get_discovered_events();
    assert_eq!(events.len(), 2);

    let event_names: Vec<&str> = events.iter().map(|e| e.event_name.as_str()).collect();
    assert!(event_names.contains(&"event-1"));
    assert!(event_names.contains(&"event-2"));
}

#[test]
fn test_discovers_emit_to() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::{AppHandle, Emitter};
            use serde::Serialize;

            #[derive(Clone, Serialize)]
            pub struct TargetedEvent {
                pub value: bool,
            }

            #[tauri::command]
            pub fn my_command(app: AppHandle) {
                app.emit_to("main", "targeted-event", TargetedEvent { value: true }).unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let _commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    let events = analyzer.get_discovered_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_name, "targeted-event");
    assert_eq!(events[0].payload_type, "TargetedEvent");
}

#[test]
fn test_discovers_events_in_conditionals() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::{AppHandle, Emitter};
            use serde::Serialize;

            #[derive(Clone, Serialize)]
            pub struct SuccessEvent {
                pub result: String,
            }

            #[derive(Clone, Serialize)]
            pub struct ErrorEvent {
                pub error: String,
            }

            #[tauri::command]
            pub fn my_command(app: AppHandle, success: bool) {
                if success {
                    app.emit("success", SuccessEvent { result: "ok".to_string() }).unwrap();
                } else {
                    app.emit("error", ErrorEvent { error: "failed".to_string() }).unwrap();
                }
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let _commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    let events = analyzer.get_discovered_events();
    assert_eq!(events.len(), 2);

    let event_names: Vec<&str> = events.iter().map(|e| e.event_name.as_str()).collect();
    assert!(event_names.contains(&"success"));
    assert!(event_names.contains(&"error"));
}

#[test]
fn test_discovers_events_in_match_arms() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::{AppHandle, Emitter};

            #[tauri::command]
            pub fn my_command(app: AppHandle, step: i32) {
                match step {
                    1 => {
                        app.emit("step-one", "First step").unwrap();
                    }
                    2 => {
                        app.emit("step-two", "Second step").unwrap();
                    }
                    _ => {
                        app.emit("step-unknown", "Unknown step").unwrap();
                    }
                }
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let _commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    let events = analyzer.get_discovered_events();
    assert_eq!(events.len(), 3);

    let event_names: Vec<&str> = events.iter().map(|e| e.event_name.as_str()).collect();
    assert!(event_names.contains(&"step-one"));
    assert!(event_names.contains(&"step-two"));
    assert!(event_names.contains(&"step-unknown"));
}

#[test]
fn test_discovers_events_in_loops() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::{AppHandle, Emitter};

            #[tauri::command]
            pub fn my_command(app: AppHandle, count: i32) {
                for i in 0..count {
                    app.emit("iteration", i).unwrap();
                }
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let _commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    let events = analyzer.get_discovered_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_name, "iteration");
}

#[test]
fn test_discovers_events_from_method_call_chain() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::{AppHandle, Emitter};

            #[tauri::command]
            pub fn my_command(app: AppHandle) {
                // Method call chains like app.clone().emit() are detected
                app.clone().emit("chained-event", "test").unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let _commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    let events = analyzer.get_discovered_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_name, "chained-event");
}

#[test]
fn test_discovers_events_in_non_command_functions() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::{AppHandle, Emitter};

            pub fn helper_function(app: &AppHandle) {
                app.emit("helper-event", "from helper").unwrap();
            }

            #[tauri::command]
            pub fn my_command(app: AppHandle) {
                helper_function(&app);
                app.emit("command-event", "from command").unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let _commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    let events = analyzer.get_discovered_events();
    // Should discover both events
    assert_eq!(events.len(), 2);

    let event_names: Vec<&str> = events.iter().map(|e| e.event_name.as_str()).collect();
    assert!(event_names.contains(&"helper-event"));
    assert!(event_names.contains(&"command-event"));
}

#[test]
fn test_infers_primitive_payload_types() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::{AppHandle, Emitter};

            #[tauri::command]
            pub fn my_command(app: AppHandle) {
                app.emit("string-event", "text").unwrap();
                app.emit("number-event", 42).unwrap();
                app.emit("bool-event", true).unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let _commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    let events = analyzer.get_discovered_events();
    assert_eq!(events.len(), 3);

    // Find each event and check its inferred type
    let string_event = events
        .iter()
        .find(|e| e.event_name == "string-event")
        .unwrap();
    assert_eq!(string_event.payload_type, "String");

    let number_event = events
        .iter()
        .find(|e| e.event_name == "number-event")
        .unwrap();
    assert_eq!(number_event.payload_type, "i32");

    let bool_event = events
        .iter()
        .find(|e| e.event_name == "bool-event")
        .unwrap();
    assert_eq!(bool_event.payload_type, "bool");
}

#[test]
fn test_typescript_type_conversion() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::{AppHandle, Emitter};
            use serde::Serialize;

            #[derive(Clone, Serialize)]
            pub struct MyEvent {
                pub count: i32,
                pub name: String,
                pub active: bool,
            }

            #[tauri::command]
            pub fn my_command(app: AppHandle) {
                app.emit("typed-event", MyEvent {
                    count: 42,
                    name: "test".to_string(),
                    active: true,
                }).unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let _commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    let events = analyzer.get_discovered_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_name, "typed-event");
    assert_eq!(events[0].payload_type, "MyEvent");
    assert_eq!(events[0].typescript_payload_type, "MyEvent");
}

#[test]
fn test_discovers_events_from_fixture_file() {
    let mut analyzer = CommandAnalyzer::new();
    let fixture_path = std::path::Path::new("tests/fixtures/sample_events.rs");

    // Just analyze the single fixture file
    let _commands = analyzer.analyze_file(fixture_path).unwrap();

    let events = analyzer.get_discovered_events();

    // Should discover multiple events from the fixture
    assert!(!events.is_empty(), "Should discover at least one event");

    // Check for specific events we know are in the fixture
    let event_names: Vec<&str> = events.iter().map(|e| e.event_name.as_str()).collect();

    // These events should be discovered from sample_events.rs
    assert!(event_names.contains(&"download-started"));
    assert!(event_names.contains(&"download-progress"));
    assert!(event_names.contains(&"download-complete"));
}

#[test]
fn test_no_false_positives() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::AppHandle;

            // This should NOT be detected (no emit call)
            #[tauri::command]
            pub fn my_command(app: AppHandle) {
                println!("No events here");
            }

            // This should NOT be detected (not a tauri emitter)
            pub fn other_function() {
                let x = "emit";
                println!("{}", x);
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let _commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    let events = analyzer.get_discovered_events();
    assert_eq!(events.len(), 0, "Should not detect false positives");
}

#[test]
fn test_event_file_path_tracking() {
    let temp_dir = create_test_project(&[(
        "src/events.rs",
        r#"
            use tauri::{AppHandle, Emitter};

            #[tauri::command]
            pub fn test_command(app: AppHandle) {
                app.emit("tracked-event", "test").unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let _commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    let events = analyzer.get_discovered_events();
    assert_eq!(events.len(), 1);

    // Should track the file path
    assert!(events[0].file_path.ends_with("src/events.rs"));
    // Should track line number (non-zero)
    assert!(events[0].line_number > 0);
}
