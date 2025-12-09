use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tauri_typegen::analysis::CommandAnalyzer;
use tauri_typegen::generators::generator::BindingsGenerator;
use tauri_typegen::models::StructInfo;
use tempfile::TempDir;

#[test]
fn test_map_type_conversion() {
    let mut analyzer = CommandAnalyzer::new();

    // Test HashMap<String, String> -> Map<string, string>
    let result = analyzer.map_rust_type_to_typescript("HashMap<String, String>");
    assert_eq!(result, "Map<string, string>");

    // Test BTreeMap<i32, User> -> Map<number, User>
    let result = analyzer.map_rust_type_to_typescript("BTreeMap<i32, User>");
    assert_eq!(result, "Map<number, User>");

    // Test nested HashMap<String, Vec<Option<i32>>> -> Map<string, (number | null)[]>
    let result = analyzer.map_rust_type_to_typescript("HashMap<String, Vec<Option<i32>>>");
    assert_eq!(result, "Map<string, number | null[]>");
}

#[test]
fn test_set_type_conversion() {
    let mut analyzer = CommandAnalyzer::new();

    // Test HashSet<String> -> string[] (arrays for JSON compatibility)
    let result = analyzer.map_rust_type_to_typescript("HashSet<String>");
    assert_eq!(result, "string[]");

    // Test BTreeSet<i32> -> number[]
    let result = analyzer.map_rust_type_to_typescript("BTreeSet<i32>");
    assert_eq!(result, "number[]");
}

#[test]
fn test_tuple_type_conversion() {
    let mut analyzer = CommandAnalyzer::new();

    // Test (String, i32) -> [string, number]
    let result = analyzer.map_rust_type_to_typescript("(String, i32)");
    assert_eq!(result, "[string, number]");

    // Test (String, i32, Option<f64>) -> [string, number, number | null]
    let result = analyzer.map_rust_type_to_typescript("(String, i32, Option<f64>)");
    assert_eq!(result, "[string, number, number | null]");

    // Test () -> void
    let result = analyzer.map_rust_type_to_typescript("()");
    assert_eq!(result, "void");
}

#[test]
fn test_deeply_nested_types() {
    let mut analyzer = CommandAnalyzer::new();

    // Test Option<Vec<Result<MyStruct, String>>> -> (MyStruct)[] | null
    let result = analyzer.map_rust_type_to_typescript("Option<Vec<Result<MyStruct, String>>>");
    assert_eq!(result, "MyStruct[] | null");

    // Test HashMap<String, Vec<Option<User>>> -> Map<string, (User | null)[]>
    let result = analyzer.map_rust_type_to_typescript("HashMap<String, Vec<Option<User>>>");
    assert_eq!(result, "Map<string, User | null[]>");
}

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
    assert_eq!(create_user.parameters[0].typescript_type, "string");

    assert_eq!(create_user.parameters[1].name, "metadata");
    assert_eq!(
        create_user.parameters[1].typescript_type,
        "Map<string, string>"
    );

    assert_eq!(create_user.parameters[2].name, "tags");
    assert_eq!(create_user.parameters[2].typescript_type, "string[]");

    // Test get_user_products command with optional BTreeMap
    let get_products = commands
        .iter()
        .find(|c| c.name == "get_user_products")
        .expect("get_user_products command should be found");

    assert_eq!(get_products.parameters.len(), 2);
    assert_eq!(get_products.parameters[1].name, "filters");
    assert_eq!(
        get_products.parameters[1].typescript_type,
        "Map<string, string[]> | null"
    );
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
    assert_eq!(
        get_tuple_data.return_type_ts,
        "[string, number, number | null]"
    );
}

#[test]
fn test_yup_schema_generation_for_maps() {
    let generator = BindingsGenerator::new(Some("yup".to_string()));

    // Test Record<string, string> - yup support removed
    let schema = generator.typescript_to_yup_type("Record<string, string>");
    assert!(schema.contains("yup support removed"));

    // Test Record arrays - yup support removed
    let schema = generator.typescript_to_yup_type("Record<string, number>[]");
    assert!(schema.contains("yup support removed"));
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

    let mut generator = BindingsGenerator::new(Some("zod".to_string()));
    let generated_files = generator
        .generate_models(
            &commands,
            discovered_structs,
            output_path.to_str().unwrap(),
            &CommandAnalyzer::new(),
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

#[allow(dead_code)]
fn create_sample_structs() -> HashMap<String, StructInfo> {
    HashMap::new() // Use discovered structs from analyzer in real tests
}
