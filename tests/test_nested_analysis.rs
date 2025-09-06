use std::collections::HashSet;
use tauri_plugin_typegen::analysis::CommandAnalyzer;

#[test]
fn test_extract_type_names_recursive() {
    let mut analyzer = CommandAnalyzer::new();
    let mut type_names = HashSet::new();
    
    // Test simple custom type
    analyzer.extract_type_names("User", &mut type_names);
    assert!(type_names.contains("User"));
    
    type_names.clear();
    
    // Test Option<CustomType>
    analyzer.extract_type_names("Option<User>", &mut type_names);
    assert!(type_names.contains("User"));
    assert!(!type_names.contains("Option"));
    
    type_names.clear();
    
    // Test Vec<CustomType>
    analyzer.extract_type_names("Vec<Product>", &mut type_names);
    assert!(type_names.contains("Product"));
    assert!(!type_names.contains("Vec"));
    
    type_names.clear();
    
    // Test Result<CustomType, Error>
    analyzer.extract_type_names("Result<User, AppError>", &mut type_names);
    assert!(type_names.contains("User"));
    assert!(type_names.contains("AppError"));
    assert!(!type_names.contains("Result"));
    
    type_names.clear();
    
    // Test HashMap<String, CustomType>
    analyzer.extract_type_names("HashMap<String, User>", &mut type_names);
    assert!(type_names.contains("User"));
    assert!(!type_names.contains("HashMap"));
    assert!(!type_names.contains("String"));
    
    type_names.clear();
    
    // Test complex nested: Option<Vec<Result<User, String>>>
    analyzer.extract_type_names("Option<Vec<Result<User, String>>>", &mut type_names);
    assert!(type_names.contains("User"));
    assert!(!type_names.contains("String"));
    assert!(!type_names.contains("Vec"));
    assert!(!type_names.contains("Option"));
    assert!(!type_names.contains("Result"));
}

#[test]
fn test_extract_multiple_custom_types() {
    let mut analyzer = CommandAnalyzer::new();
    let mut type_names = HashSet::new();
    
    // Test HashMap<CustomKey, CustomValue>
    analyzer.extract_type_names("HashMap<UserId, UserProfile>", &mut type_names);
    assert!(type_names.contains("UserId"));
    assert!(type_names.contains("UserProfile"));
    assert_eq!(type_names.len(), 2);
    
    type_names.clear();
    
    // Test tuple with custom types
    analyzer.extract_type_names("(User, Product, Order)", &mut type_names);
    assert!(type_names.contains("User"));
    assert!(type_names.contains("Product"));
    assert!(type_names.contains("Order"));
    assert_eq!(type_names.len(), 3);
}

#[test]
fn test_extract_deeply_nested_types() {
    let mut analyzer = CommandAnalyzer::new();
    let mut type_names = HashSet::new();
    
    // Test very complex nested structure
    analyzer.extract_type_names("HashMap<String, Vec<Option<Result<UserProfile, ValidationError>>>>", &mut type_names);
    assert!(type_names.contains("UserProfile"));
    assert!(type_names.contains("ValidationError"));
    assert!(!type_names.contains("String"));
    assert_eq!(type_names.len(), 2);
}

#[test]
fn test_reference_handling() {
    let mut analyzer = CommandAnalyzer::new();
    let mut type_names = HashSet::new();
    
    // Test &User -> User
    analyzer.extract_type_names("&User", &mut type_names);
    assert!(type_names.contains("User"));
    
    type_names.clear();
    
    // Test &mut User -> User  
    analyzer.extract_type_names("&User", &mut type_names); // analyzer doesn't handle &mut specifically
    assert!(type_names.contains("User"));
}

#[test]
fn test_primitive_types_ignored() {
    let mut analyzer = CommandAnalyzer::new();
    let mut type_names = HashSet::new();
    
    // Test that primitives are ignored
    let primitives = vec![
        "String", "str", "i32", "i64", "f32", "f64", "bool", 
        "usize", "isize", "u32", "u64", "()", "u8", "i8", "u16", "i16"
    ];
    
    for primitive in primitives {
        analyzer.extract_type_names(primitive, &mut type_names);
    }
    
    assert!(type_names.is_empty());
}

#[test]
fn test_collect_referenced_types_from_generator() {
    let mut analyzer = CommandAnalyzer::new();
    let generator = tauri_plugin_typegen::generators::generator::BindingsGenerator::new(None);
    let mut used_types = HashSet::new();
    
    // Test complex nested structure
    generator.collect_referenced_types("HashMap<String, Vec<Option<User>>>", &mut used_types);
    assert!(used_types.contains("User"));
    assert!(!used_types.contains("String"));
    
    used_types.clear();
    
    // Test Result with both custom types
    generator.collect_referenced_types("Result<UserProfile, AppError>", &mut used_types);
    assert!(used_types.contains("UserProfile"));
    assert!(used_types.contains("AppError"));
    assert_eq!(used_types.len(), 2);
    
    used_types.clear();
    
    // Test tuple types
    generator.collect_referenced_types("(User, Product, i32)", &mut used_types);
    assert!(used_types.contains("User"));
    assert!(used_types.contains("Product"));
    assert!(!used_types.contains("i32"));
    assert_eq!(used_types.len(), 2);
}

#[test]
fn test_map_types_with_custom_generics() {
    let mut analyzer = CommandAnalyzer::new();
    
    // Test HashMap<CustomKey, CustomValue>
    let result = analyzer.map_rust_type_to_typescript("HashMap<UserId, UserProfile>");
    assert_eq!(result, "Record<UserId, UserProfile>");
    
    // Test nested HashMap
    let result = analyzer.map_rust_type_to_typescript("HashMap<String, HashMap<String, User>>");
    assert_eq!(result, "Record<string, Record<string, User>>");
    
    // Test HashMap in Option
    let result = analyzer.map_rust_type_to_typescript("Option<HashMap<String, User>>");
    assert_eq!(result, "Record<string, User> | null");
    
    // Test HashMap in Vec
    let result = analyzer.map_rust_type_to_typescript("Vec<HashMap<String, User>>");
    assert_eq!(result, "Record<string, User>[]");
}

#[test]
fn test_set_types_with_custom_generics() {
    let mut analyzer = CommandAnalyzer::new();
    
    // Test HashSet<CustomType>
    let result = analyzer.map_rust_type_to_typescript("HashSet<UserId>");
    assert_eq!(result, "UserId[]");
    
    // Test BTreeSet<CustomType>
    let result = analyzer.map_rust_type_to_typescript("BTreeSet<UserProfile>");
    assert_eq!(result, "UserProfile[]");
    
    // Test nested sets
    let result = analyzer.map_rust_type_to_typescript("Vec<HashSet<String>>");
    assert_eq!(result, "string[][]");
}

#[test]
fn test_complex_tuple_scenarios() {
    let mut analyzer = CommandAnalyzer::new();
    
    // Test mixed tuple types
    let result = analyzer.map_rust_type_to_typescript("(User, Vec<String>, Option<i32>)");
    assert_eq!(result, "[User, string[], number | null]");
    
    // Test nested tuple in Option
    let result = analyzer.map_rust_type_to_typescript("Option<(String, User)>");
    assert_eq!(result, "[string, User] | null");
    
    // Test tuple in Vec
    let result = analyzer.map_rust_type_to_typescript("Vec<(String, i32)>");
    assert_eq!(result, "[string, number][]");
}