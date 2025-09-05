use std::fs;
use std::path::Path;
use tauri_plugin_typegen::analyzer::CommandAnalyzer;
use tauri_plugin_typegen::generator::TypeScriptGenerator;
use tempfile::TempDir;

#[test]
fn test_validator_attributes_parsing() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create a test file with validator attributes
    let test_content = r#"
use serde::{Deserialize, Serialize};
use validator::Validate;
use tauri::command;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 1, max = 50))]
    pub name: String,
    
    #[validate(email)]
    pub email: String,
    
    #[validate(range(min = 18, max = 120))]
    pub age: i32,
}

#[command]
pub async fn create_user(request: CreateUserRequest) -> Result<String, String> {
    Ok("User created".to_string())
}
"#;

    fs::write(src_dir.join("lib.rs"), test_content).unwrap();

    // Analyze the file
    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer.analyze_project(temp_dir.path().to_str().unwrap()).unwrap();
    let discovered_structs = analyzer.get_discovered_structs();

    // Check that validator attributes were parsed
    let create_user_request = discovered_structs.get("CreateUserRequest").unwrap();
    
    // Test name field with length constraint
    let name_field = create_user_request.fields.iter().find(|f| f.name == "name").unwrap();
    assert!(name_field.validator_attributes.is_some());
    let name_attrs = name_field.validator_attributes.as_ref().unwrap();
    assert!(name_attrs.length.is_some());
    let length_constraint = name_attrs.length.as_ref().unwrap();
    assert_eq!(length_constraint.min, Some(1));
    assert_eq!(length_constraint.max, Some(50));

    // Test email field with email constraint
    let email_field = create_user_request.fields.iter().find(|f| f.name == "email").unwrap();
    assert!(email_field.validator_attributes.is_some());
    let email_attrs = email_field.validator_attributes.as_ref().unwrap();
    assert!(email_attrs.email);

    // Test age field with range constraint
    let age_field = create_user_request.fields.iter().find(|f| f.name == "age").unwrap();
    assert!(age_field.validator_attributes.is_some());
    let age_attrs = age_field.validator_attributes.as_ref().unwrap();
    assert!(age_attrs.range.is_some());
    let range_constraint = age_attrs.range.as_ref().unwrap();
    assert_eq!(range_constraint.min, Some(18.0));
    assert_eq!(range_constraint.max, Some(120.0));
}

#[test]
fn test_zod_validator_constraints_generation() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let output_dir = temp_dir.path().join("generated");
    fs::create_dir_all(&src_dir).unwrap();

    // Create a test file with validator attributes
    let test_content = r#"
use serde::{Deserialize, Serialize};
use validator::Validate;
use tauri::command;

#[derive(Debug, Deserialize, Validate)]
pub struct ProductData {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    
    #[validate(email)]
    pub contact_email: String,
    
    #[validate(range(min = 0.01, max = 999.99))]
    pub price: f64,
}

#[command]
pub async fn create_product(data: ProductData) -> Result<String, String> {
    Ok("Product created".to_string())
}
"#;

    fs::write(src_dir.join("lib.rs"), test_content).unwrap();

    // Analyze and generate
    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer.analyze_project(temp_dir.path().to_str().unwrap()).unwrap();
    let discovered_structs = analyzer.get_discovered_structs();

    let mut generator = TypeScriptGenerator::new(Some("zod".to_string()));
    generator.generate_models(&commands, &discovered_structs, output_dir.to_str().unwrap(), &CommandAnalyzer::new().unwrap()).unwrap();

    // Read the generated types file
    let types_content = fs::read_to_string(output_dir.join("types.ts")).unwrap();
    
    // Check that the zod schema includes validator constraints
    println!("Generated types content:\n{}", types_content);
    
    // Should contain length constraints for name field
    assert!(types_content.contains("z.string().min(1).max(100)"));
    
    // Should contain email validation for contact_email field  
    assert!(types_content.contains("z.string().email()"));
    
    // Should contain range constraints for price field
    assert!(types_content.contains("z.number().min(0.01).max(999.99)"));
}