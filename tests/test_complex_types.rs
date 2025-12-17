use std::fs;
use std::path::Path;
use tauri_typegen::analysis::CommandAnalyzer;
use tauri_typegen::generators::create_generator;
use tauri_typegen::GenerateConfig;
use tempfile::TempDir;

#[test]
fn test_complex_types_analysis() {
    let mut analyzer = CommandAnalyzer::new();
    let fixture_path = Path::new("tests/fixtures/complex_types.rs");

    let commands = analyzer.analyze_file(fixture_path).unwrap();

    // Should find 6 commands
    assert_eq!(commands.len(), 6);

    let command_names: Vec<String> = commands.iter().map(|c| c.name.clone()).collect();
    assert!(command_names.contains(&"create_user_with_metadata".to_string()));
    assert!(command_names.contains(&"get_user_products".to_string()));
    assert!(command_names.contains(&"update_order_status".to_string()));
    assert!(command_names.contains(&"process_complex_data".to_string()));
    assert!(command_names.contains(&"get_tuple_data".to_string()));
    assert!(command_names.contains(&"bulk_update_attributes".to_string()));
}

#[test]
fn test_complex_parameter_types() {
    let mut analyzer = CommandAnalyzer::new();
    let fixture_path = Path::new("tests/fixtures/complex_types.rs");

    let commands = analyzer.analyze_file(fixture_path).unwrap();

    // Test create_user_with_metadata command
    let create_user = commands
        .iter()
        .find(|c| c.name == "create_user_with_metadata")
        .expect("create_user_with_metadata command should be found");

    assert_eq!(create_user.parameters.len(), 3);
    assert_eq!(create_user.parameters[0].name, "name");
    assert_eq!(create_user.parameters[1].name, "metadata");
    assert_eq!(create_user.parameters[2].name, "tags");

    // Test get_user_products command with optional BTreeMap
    let get_products = commands
        .iter()
        .find(|c| c.name == "get_user_products")
        .expect("get_user_products command should be found");

    assert_eq!(get_products.parameters.len(), 2);
    assert_eq!(get_products.parameters[1].name, "filters");
    assert!(get_products.parameters[1].is_optional);
}

#[test]
fn test_tuple_return_type() {
    let mut analyzer = CommandAnalyzer::new();
    let fixture_path = Path::new("tests/fixtures/complex_types.rs");

    let commands = analyzer.analyze_file(fixture_path).unwrap();

    let get_tuple_data = commands
        .iter()
        .find(|c| c.name == "get_tuple_data")
        .expect("get_tuple_data command should be found");

    assert_eq!(get_tuple_data.return_type, "(String, i32, Option<f64>)");
}

#[test]
fn test_enum_variant_parsing() {
    let mut analyzer = CommandAnalyzer::new();
    let fixture_path = Path::new("tests/fixtures/complex_types.rs");

    // Analyze the file to discover structs
    let _ = analyzer.analyze_file(fixture_path).unwrap();
    // Create a temp directory with only our complex_types.rs file to avoid syntax errors
    let temp_dir = TempDir::new().unwrap();
    fs::copy(
        "tests/fixtures/complex_types.rs",
        temp_dir.path().join("complex_types.rs"),
    )
    .unwrap();
    let _ = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    let discovered_structs = analyzer.get_discovered_structs();

    // Check if OrderStatus enum was discovered
    let order_status = discovered_structs.get("OrderStatus");
    assert!(order_status.is_some());

    let order_status = order_status.unwrap();
    assert!(order_status.is_enum);

    // Should have different variant types
    let variant_names: Vec<&String> = order_status.fields.iter().map(|f| &f.name).collect();
    assert!(variant_names.contains(&&"Pending".to_string()));
    assert!(variant_names.contains(&&"Processing".to_string()));
    assert!(variant_names.contains(&&"Shipped".to_string()));
    assert!(variant_names.contains(&&"Delivered".to_string()));
    assert!(variant_names.contains(&&"Cancelled".to_string()));
}

#[test]
fn test_full_generation_with_complex_types() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("generated");
    fs::create_dir_all(&output_path).unwrap();

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_file(Path::new("tests/fixtures/complex_types.rs"))
        .unwrap();
    // Create a temp directory with only our complex_types.rs file to avoid syntax errors
    let temp_dir_for_analysis = TempDir::new().unwrap();
    fs::copy(
        "tests/fixtures/complex_types.rs",
        temp_dir_for_analysis.path().join("complex_types.rs"),
    )
    .unwrap();
    let _ = analyzer
        .analyze_project(temp_dir_for_analysis.path().to_str().unwrap())
        .unwrap();
    let discovered_structs = analyzer.get_discovered_structs();

    let mut generator = create_generator(Some("zod".to_string()));
    let generated_files = generator
        .generate_models(
            &commands,
            discovered_structs,
            output_path.to_str().unwrap(),
            &CommandAnalyzer::new(),
            &GenerateConfig::new(),
        )
        .unwrap();

    // Check that all expected files were generated
    assert!(generated_files.contains(&"types.ts".to_string()));
    // schemas.ts is not generated for zod - schemas are embedded in types.ts
    assert!(generated_files.contains(&"commands.ts".to_string()));
    assert!(generated_files.contains(&"index.ts".to_string()));

    // Should have 3 files for zod validation
    assert_eq!(generated_files.len(), 3);

    // Check that the files actually exist and contain expected content
    let types_content = fs::read_to_string(output_path.join("types.ts")).unwrap();
    assert!(types_content.contains("Record<string, string>") || types_content.contains("z.record"));
    assert!(types_content.contains("string[]") || types_content.contains("z.array"));
    assert!(types_content.contains("[string, number") || types_content.contains("z.tuple"));

    // For zod, schemas are embedded in types.ts
    assert!(types_content.contains("z.record("));
    assert!(types_content.contains("z.array("));

    let commands_content = fs::read_to_string(output_path.join("commands.ts")).unwrap();
    assert!(commands_content.contains("createUserWithMetadata"));
    assert!(commands_content.contains("getUserProducts"));
    assert!(commands_content.contains("updateOrderStatus"));
}
