//! Integration tests for complete end-to-end scenarios
//! Tests real-world usage patterns combining multiple features

use crate::common;
use crate::fixtures;

use common::{TestGenerator, TestProject};

#[test]
fn test_complete_app_with_commands_and_events() {
    let project = TestProject::new();

    // Write a complete mini app
    project.write_file("main.rs", r#"
        use tauri::Manager;
        use serde::{Deserialize, Serialize};
        
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct User {
            pub user_id: String,
            pub user_name: String,
        }
        
        #[tauri::command]
        pub fn get_user(id: String) -> Result<User, String> {
            Ok(User {
                user_id: id,
                user_name: "John".to_string(),
            })
        }
        
        #[tauri::command]
        #[serde(rename_all = "camelCase")]
        pub fn update_user(user_id: String, new_name: String, app: tauri::AppHandle) -> Result<(), String> {
            app.emit("user-updated", User {
                user_id: user_id.clone(),
                user_name: new_name,
            }).ok();
            Ok(())
        }
    "#);

    let (analyzer, commands) = project.analyze();
    let events = analyzer.get_discovered_events();

    // Verify analysis
    assert_eq!(commands.len(), 2);
    assert_eq!(events.len(), 1);

    // Generate with Zod
    let generator = TestGenerator::new();
    let files = generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("zod"),
        None,
    );

    // Should generate all files
    assert!(files.contains(&"types.ts".to_string()));
    assert!(files.contains(&"commands.ts".to_string()));
    assert!(files.contains(&"events.ts".to_string()));
    assert!(files.contains(&"index.ts".to_string()));

    // Verify types.ts
    let types = generator.read_file("types.ts");
    assert!(types.contains("export interface User"));
    assert!(types.contains("userId: string"));
    assert!(types.contains("userName: string"));
    assert!(types.contains("export const UserSchema"));

    // Verify commands.ts
    let commands_file = generator.read_file("commands.ts");
    assert!(commands_file.contains("export async function getUser"));
    assert!(commands_file.contains("export async function updateUser"));

    // Verify events.ts
    let events_file = generator.read_file("events.ts");
    assert!(events_file.contains("export function onUserUpdated"));

    // Verify index.ts exports everything
    let index = generator.read_file("index.ts");
    assert!(index.contains("export * from './types'"));
    assert!(index.contains("export * from './commands'"));
    assert!(index.contains("export * from './events'"));
}

#[test]
fn test_multiple_commands_with_shared_types() {
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
        }
        
        #[tauri::command]
        pub fn get_product(id: String) -> Result<Product, String> {
            Ok(Product {
                id,
                name: "Widget".to_string(),
                price: 19.99,
            })
        }
        
        #[tauri::command]
        pub fn list_products() -> Vec<Product> {
            vec![]
        }
        
        #[tauri::command]
        pub fn update_product(product: Product) -> Result<Product, String> {
            Ok(product)
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

    // Product type should be defined once and reused
    let product_count = types.matches("export interface Product").count();
    assert_eq!(product_count, 1, "Product type should only be defined once");

    // All commands should reference the same Product type
    assert!(types.contains("export interface GetProductParams"));
    assert!(types.contains("export interface UpdateProductParams"));
    assert!(types.contains("product: Product")); // In UpdateProductParams
}

#[test]
fn test_commands_channels_and_events_together() {
    let project = TestProject::new();

    project.write_file(
        "main.rs",
        r#"
        use tauri::{Manager, ipc::Channel};
        use serde::{Deserialize, Serialize};
        
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Progress {
            pub current: u32,
            pub total: u32,
        }
        
        #[tauri::command]
        pub fn process_data(
            data: Vec<String>,
            progress: Channel<Progress>,
            app: tauri::AppHandle,
        ) -> Result<(), String> {
            progress.send(Progress { current: 50, total: 100 }).ok();
            app.emit("processing-complete", ()).ok();
            Ok(())
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

    // Types file should have Progress schema and Channel type import
    let types = generator.read_file("types.ts");
    assert!(types.contains("export const ProgressSchema"));
    assert!(types.contains("import type { Channel } from '@tauri-apps/api/core'"));

    // Commands should handle channel parameter
    let commands_file = generator.read_file("commands.ts");
    assert!(commands_file.contains("progress: Channel<Progress>"));

    // Events should have the event listener
    let events_file = generator.read_file("events.ts");
    assert!(events_file.contains("export function onProcessingComplete"));
}

#[test]
fn test_empty_project() {
    let project = TestProject::new();
    project.write_file("main.rs", "// Empty file");

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

    // Should still generate basic files even with no commands
    assert!(files.contains(&"types.ts".to_string()));
    assert!(files.contains(&"commands.ts".to_string()));
    assert!(files.contains(&"index.ts".to_string()));
}

#[test]
fn test_deeply_nested_types() {
    let project = TestProject::new();

    project.write_file(
        "main.rs",
        r#"
        use serde::{Deserialize, Serialize};
        use std::collections::HashMap;
        
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Item {
            pub id: String,
        }
        
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Category {
            pub name: String,
            pub items: Vec<Item>,
        }
        
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Store {
            pub categories: HashMap<String, Category>,
        }
        
        #[tauri::command]
        pub fn get_store() -> Store {
            Store {
                categories: HashMap::new(),
            }
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

    // All nested types should be defined
    assert!(types.contains("export interface Item"));
    assert!(types.contains("export interface Category"));
    assert!(types.contains("export interface Store"));

    // Relationships should be correct
    assert!(types.contains("items: Array<Item>") || types.contains("items: Item[]"));
    assert!(types.contains("categories: Record<string, Category>"));
}

#[test]
fn test_config_affects_all_generation() {
    use tauri_typegen::GenerateConfig;

    let project = TestProject::new();
    project.write_file(
        "main.rs",
        r#"
        use serde::{Deserialize, Serialize};
        
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct MyStruct {
            pub first_field: String,
            pub second_field: u32,
        }
        
        #[tauri::command]
        pub fn my_command(first_param: String, second_param: u32) -> MyStruct {
            MyStruct {
                first_field: first_param,
                second_field: second_param,
            }
        }
    "#,
    );

    let (analyzer, commands) = project.analyze();
    let generator = TestGenerator::new();

    // Use snake_case for everything
    let mut config = GenerateConfig::default();
    config.default_parameter_case = "snake_case".to_string();
    config.default_field_case = "snake_case".to_string();

    generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("none"),
        Some(&config),
    );

    let types = generator.read_file("types.ts");

    // Parameters should use snake_case
    assert!(types.contains("first_param: string"));
    assert!(types.contains("second_param: number"));

    // Fields should use snake_case
    assert!(types.contains("first_field: string"));
    assert!(types.contains("second_field: number"));
}
