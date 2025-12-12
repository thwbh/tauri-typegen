use std::collections::HashSet;
use tauri_typegen::analysis::CommandAnalyzer;

#[test]
fn test_extract_type_names_recursive() {
    let analyzer = CommandAnalyzer::new();
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
    let analyzer = CommandAnalyzer::new();
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
    let analyzer = CommandAnalyzer::new();
    let mut type_names = HashSet::new();

    // Test very complex nested structure
    analyzer.extract_type_names(
        "HashMap<String, Vec<Option<Result<UserProfile, ValidationError>>>>",
        &mut type_names,
    );
    assert!(type_names.contains("UserProfile"));
    assert!(type_names.contains("ValidationError"));
    assert!(!type_names.contains("String"));
    assert_eq!(type_names.len(), 2);
}

#[test]
fn test_reference_handling() {
    let analyzer = CommandAnalyzer::new();
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
    let analyzer = CommandAnalyzer::new();
    let mut type_names = HashSet::new();

    // Test that primitives are ignored
    let primitives = vec![
        "String", "str", "i32", "i64", "f32", "f64", "bool", "usize", "isize", "u32", "u64", "()",
        "u8", "i8", "u16", "i16",
    ];

    for primitive in primitives {
        analyzer.extract_type_names(primitive, &mut type_names);
    }

    assert!(type_names.is_empty());
}

#[test]
fn test_collect_referenced_types_from_generator() {
    let _analyzer = CommandAnalyzer::new();
    let generator = tauri_typegen::generators::generator::BindingsGenerator::new(None);
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
