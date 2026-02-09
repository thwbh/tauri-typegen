//! End-to-end integration tests
//! Tests complete Rust → TypeScript translation pipeline
//!
//! These tests verify the FULL pipeline works correctly, not individual components.
//! Component-level testing is done in unit tests (src/**/*.rs).

mod common;
mod fixtures;

use common::{TestGenerator, TestProject};

/// Test complete vanilla TypeScript generation from Rust to TS
#[test]
fn test_vanilla_typescript_full_pipeline() {
    let project = TestProject::new();

    project.write_file(
        "main.rs",
        r#"
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct User {
            pub id: String,
            pub name: String,
            pub email: String,
        }

        #[tauri::command]
        pub fn get_user(user_id: String) -> Result<User, String> {
            Ok(User {
                id: user_id,
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
            })
        }

        #[tauri::command]
        pub fn create_user(name: String, email: String) -> Result<User, String> {
            Ok(User {
                id: "123".to_string(),
                name,
                email,
            })
        }
    "#,
    );

    let (analyzer, commands) = project.analyze();
    assert_eq!(commands.len(), 2);

    let generator = TestGenerator::new();
    let files = generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("none"),
        None,
    );

    // Verify all expected files are generated
    assert!(files.contains(&"types.ts".to_string()));
    assert!(files.contains(&"commands.ts".to_string()));
    assert!(files.contains(&"index.ts".to_string()));

    // Verify types.ts content
    let types = generator.read_file("types.ts");
    assert!(types.contains("export interface User"));
    assert!(types.contains("id: string"));
    assert!(types.contains("name: string"));
    assert!(types.contains("email: string"));

    // Verify NO Zod schemas in vanilla TS
    assert!(!types.contains("z.object"));
    assert!(!types.contains("UserSchema"));

    // Verify commands.ts content
    let commands_file = generator.read_file("commands.ts");
    assert!(commands_file.contains("export async function getUser"));
    assert!(commands_file.contains("export async function createUser"));
    assert!(commands_file.contains("invoke"));

    // Verify index.ts exports
    let index = generator.read_file("index.ts");
    assert!(index.contains("export * from './types'"));
    assert!(index.contains("export * from './commands'"));
}

/// Test complete Zod TypeScript generation from Rust to TS with validation
#[test]
fn test_zod_typescript_full_pipeline() {
    let project = TestProject::new();

    project.write_file(
        "main.rs",
        r#"
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Product {
            pub id: String,
            pub name: String,
            pub price: f64,
            pub in_stock: bool,
        }

        #[tauri::command]
        pub fn get_product(product_id: String) -> Result<Product, String> {
            Ok(Product {
                id: product_id,
                name: "Widget".to_string(),
                price: 19.99,
                in_stock: true,
            })
        }

        #[tauri::command]
        pub fn create_product(name: String, price: f64) -> Result<Product, String> {
            Ok(Product {
                id: "123".to_string(),
                name,
                price,
                in_stock: true,
            })
        }
    "#,
    );

    let (analyzer, commands) = project.analyze();
    assert_eq!(commands.len(), 2);

    let generator = TestGenerator::new();
    let files = generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("zod"),
        None,
    );

    // Verify all expected files are generated
    assert!(files.contains(&"types.ts".to_string()));
    assert!(files.contains(&"commands.ts".to_string()));
    assert!(files.contains(&"index.ts".to_string()));

    // Verify types.ts with Zod schemas
    let types = generator.read_file("types.ts");

    // Struct schema - should exist with zod validation
    assert!(
        types.contains("ProductSchema"),
        "Should generate ProductSchema"
    );
    assert!(
        types.contains("z.object") || types.contains("z.string"),
        "Should use zod validators"
    );

    // Type aliases should exist
    assert!(
        types.contains("Product") && types.contains("export"),
        "Should export Product type"
    );

    // Verify commands.ts uses schemas for validation
    let commands_file = generator.read_file("commands.ts");
    assert!(commands_file.contains("getProduct") || commands_file.contains("get_product"));
    assert!(commands_file.contains("createProduct") || commands_file.contains("create_product"));
}

/// Test complete pipeline with commands, channels, and events together
#[test]
fn test_complete_app_with_commands_channels_and_events() {
    let project = TestProject::new();

    project.write_file(
        "main.rs",
        r#"
        use tauri::Manager;
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Message {
            pub id: String,
            pub content: String,
            pub timestamp: u64,
        }

        #[tauri::command]
        pub fn send_message(
            content: String,
            app: tauri::AppHandle,
            on_progress: tauri::ipc::Channel<f32>,
        ) -> Result<Message, String> {
            let msg = Message {
                id: "123".to_string(),
                content,
                timestamp: 1234567890,
            };

            // Emit event
            app.emit("message-sent", msg.clone()).ok();

            // Send channel progress
            on_progress.send(100.0).ok();

            Ok(msg)
        }
    "#,
    );

    let (analyzer, commands) = project.analyze();
    let events = analyzer.get_discovered_events();
    let all_channels = analyzer.get_all_discovered_channels(&commands);

    // Verify analysis found everything
    assert_eq!(commands.len(), 1);
    assert_eq!(events.len(), 1);
    assert_eq!(all_channels.len(), 1);

    // Generate with Zod
    let generator = TestGenerator::new();
    let files = generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("zod"),
        None,
    );

    // Should generate all files including events
    assert!(files.contains(&"types.ts".to_string()));
    assert!(files.contains(&"commands.ts".to_string()));
    assert!(files.contains(&"events.ts".to_string()));
    assert!(files.contains(&"index.ts".to_string()));

    // Verify types.ts
    let types = generator.read_file("types.ts");
    assert!(types.contains("Message"), "Should contain Message type");
    assert!(
        types.contains("MessageSchema"),
        "Should contain MessageSchema"
    );
    assert!(
        types.contains("Channel") || types.contains("channel"),
        "Should reference Channel"
    );

    // Verify commands.ts has channel parameter
    let commands_file = generator.read_file("commands.ts");
    assert!(
        commands_file.contains("sendMessage"),
        "Should contain sendMessage function"
    );
    assert!(
        commands_file.contains("Channel") || commands_file.contains("channel"),
        "Should use Channel in signature"
    );

    // Verify events.ts
    let events_file = generator.read_file("events.ts");
    assert!(
        events_file.contains("MessageSent") || events_file.contains("message"),
        "Should contain message-sent event listener"
    );

    // Verify index.ts exports everything
    let index = generator.read_file("index.ts");
    assert!(index.contains("export * from './types'"));
    assert!(index.contains("export * from './commands'"));
    assert!(index.contains("export * from './events'"));
}

/// Test serde attributes are properly translated through the full pipeline
#[test]
fn test_serde_rename_full_pipeline() {
    let project = TestProject::new();

    project.write_file(
        "main.rs",
        r#"
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct UserProfile {
            pub user_id: String,
            pub first_name: String,
            pub last_name: String,
            #[serde(rename = "emailAddress")]
            pub email: String,
        }

        #[tauri::command]
        #[serde(rename_all = "camelCase")]
        pub fn get_profile(user_id: String) -> Result<UserProfile, String> {
            Ok(UserProfile {
                user_id,
                first_name: "John".to_string(),
                last_name: "Doe".to_string(),
                email: "john@example.com".to_string(),
            })
        }
    "#,
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

    // Verify struct fields use camelCase from serde (field names, not necessarily exact zod syntax)
    assert!(
        types.contains("userId"),
        "Should use userId from camelCase rename"
    );
    assert!(
        types.contains("firstName"),
        "Should use firstName from camelCase rename"
    );
    assert!(
        types.contains("lastName"),
        "Should use lastName from camelCase rename"
    );

    // Verify explicit rename overrides rename_all
    assert!(
        types.contains("emailAddress"),
        "Should use emailAddress from explicit rename"
    );
}

/// Test empty project generates no TypeScript files
#[test]
fn test_empty_project_no_generation() {
    let project = TestProject::new();

    // Empty Rust file with no commands
    project.write_file(
        "main.rs",
        r#"
        fn main() {
            println!("Hello, world!");
        }
    "#,
    );

    let (analyzer, commands) = project.analyze();
    assert_eq!(commands.len(), 0);

    let generator = TestGenerator::new();
    let files = generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("none"),
        None,
    );

    // Should still generate index.ts even with no commands
    // but types.ts and commands.ts should be minimal/empty
    assert!(files.contains(&"index.ts".to_string()));
}

/// Test complex nested types are properly resolved through dependencies
#[test]
fn test_deeply_nested_types_full_pipeline() {
    let project = TestProject::new();

    project.write_file(
        "main.rs",
        r#"
        use serde::{Deserialize, Serialize};
        use std::collections::HashMap;

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Address {
            pub street: String,
            pub city: String,
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Contact {
            pub email: String,
            pub address: Address,
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct User {
            pub id: String,
            pub contacts: Vec<Contact>,
        }

        #[tauri::command]
        pub fn get_user_map() -> HashMap<String, User> {
            HashMap::new()
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

    let types = generator.read_file("types.ts");

    // All types should be generated
    assert!(types.contains("export interface Address"));
    assert!(types.contains("export interface Contact"));
    assert!(types.contains("export interface User"));

    // Verify nested references
    assert!(types.contains("address: Address"));
    assert!(types.contains("contacts: Contact[]") || types.contains("contacts: Array<Contact>"));

    // Verify complex return type
    let commands_file = generator.read_file("commands.ts");
    assert!(
        commands_file.contains("Record<string, User>")
            || commands_file.contains("{ [key: string]: User }")
    );
}

/// Test complex enum (discriminated union) TypeScript generation
#[test]
fn test_complex_enum_typescript_generation() {
    let project = TestProject::new();

    project.write_file(
        "main.rs",
        r#"
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(tag = "type")]
        pub enum Message {
            Quit,
            Move(i32, i32),
            Write(String),
            ChangeColor { r: u8, g: u8, b: u8 },
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum Status {
            Active,
            Inactive,
            Pending,
        }

        #[tauri::command]
        pub fn send_message(msg: Message) -> Result<Status, String> {
            Ok(Status::Active)
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

    let types = generator.read_file("types.ts");

    // Simple enum should be string literal union
    assert!(
        types.contains(r#"export type Status = "Active" | "Inactive" | "Pending";"#),
        "Simple enum should use string literal union. Got:\n{}",
        types
    );

    // Complex enum should be discriminated union
    assert!(
        types.contains("export type Message ="),
        "Complex enum should have type declaration. Got:\n{}",
        types
    );

    // Check for discriminated union structure
    assert!(
        types.contains(r#"type: "Quit""#),
        "Should have Quit variant. Got:\n{}",
        types
    );
    assert!(
        types.contains(r#"type: "Move""#),
        "Should have Move variant. Got:\n{}",
        types
    );
    assert!(
        types.contains(r#"type: "Write""#),
        "Should have Write variant. Got:\n{}",
        types
    );
    assert!(
        types.contains(r#"type: "ChangeColor""#),
        "Should have ChangeColor variant. Got:\n{}",
        types
    );

    // Check tuple variant has data field
    assert!(
        types.contains("data:"),
        "Tuple variants should have data field. Got:\n{}",
        types
    );

    // Check struct variant has named fields
    assert!(
        types.contains("r: number"),
        "Struct variant should have r field. Got:\n{}",
        types
    );
}

/// Test complex enum Zod schema generation (discriminated union)
#[test]
fn test_complex_enum_zod_generation() {
    let project = TestProject::new();

    project.write_file(
        "main.rs",
        r#"
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(tag = "kind")]
        pub enum Action {
            Start,
            Move { x: i32, y: i32 },
            Send(String),
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum Status {
            Active,
            Inactive,
        }

        #[tauri::command]
        pub fn perform_action(action: Action) -> Result<Status, String> {
            Ok(Status::Active)
        }
    "#,
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

    // Simple enum should use z.enum
    assert!(
        types.contains("StatusSchema = z.enum"),
        "Simple enum should use z.enum. Got:\n{}",
        types
    );
    assert!(
        types.contains(r#"["Active", "Inactive"]"#),
        "Simple enum should list variants. Got:\n{}",
        types
    );

    // Complex enum should use z.discriminatedUnion
    assert!(
        types.contains("ActionSchema = z.discriminatedUnion"),
        "Complex enum should use z.discriminatedUnion. Got:\n{}",
        types
    );

    // Check discriminator is "kind" (from serde tag)
    assert!(
        types.contains(r#"z.discriminatedUnion("kind""#),
        "Should use 'kind' as discriminator. Got:\n{}",
        types
    );

    // Check unit variant
    assert!(
        types.contains(r#"z.literal("Start")"#),
        "Should have Start variant. Got:\n{}",
        types
    );

    // Check struct variant with named fields
    assert!(
        types.contains(r#"z.literal("Move")"#),
        "Should have Move variant. Got:\n{}",
        types
    );
    assert!(
        types.contains("x:") && types.contains("y:"),
        "Move variant should have x and y fields. Got:\n{}",
        types
    );

    // Check tuple variant with data field
    assert!(
        types.contains(r#"z.literal("Send")"#),
        "Should have Send variant. Got:\n{}",
        types
    );
    assert!(
        types.contains("data:"),
        "Tuple variant should have data field. Got:\n{}",
        types
    );

    // Verify type inference
    assert!(
        types.contains("export type Action = z.infer<typeof ActionSchema>"),
        "Should export inferred Action type. Got:\n{}",
        types
    );
    assert!(
        types.contains("export type Status = z.infer<typeof StatusSchema>"),
        "Should export inferred Status type. Got:\n{}",
        types
    );
}
