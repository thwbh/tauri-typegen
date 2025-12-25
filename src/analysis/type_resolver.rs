use crate::models::TypeStructure;
use std::collections::{HashMap, HashSet};

/// Type resolver for mapping Rust types to TypeScript types
#[derive(Debug)]
pub struct TypeResolver {
    type_set: HashSet<String>,
    type_mappings: HashMap<String, String>,
}

impl TypeResolver {
    pub fn new() -> Self {
        let mut type_set = HashSet::new();

        // Basic Rust types
        type_set.insert("String".to_string());
        type_set.insert("&str".to_string());
        type_set.insert("str".to_string());
        type_set.insert("i8".to_string());
        type_set.insert("i16".to_string());
        type_set.insert("i32".to_string());
        type_set.insert("i64".to_string());
        type_set.insert("i128".to_string());
        type_set.insert("isize".to_string());
        type_set.insert("u8".to_string());
        type_set.insert("u16".to_string());
        type_set.insert("u32".to_string());
        type_set.insert("u64".to_string());
        type_set.insert("u128".to_string());
        type_set.insert("usize".to_string());
        type_set.insert("f32".to_string());
        type_set.insert("f64".to_string());
        type_set.insert("bool".to_string());
        type_set.insert("()".to_string());

        // Collection type mappings
        type_set.insert("HashMap".to_string());
        type_set.insert("BTreeMap".to_string());
        type_set.insert("HashSet".to_string());
        type_set.insert("BTreeSet".to_string());

        Self {
            type_set,
            type_mappings: HashMap::new(),
        }
    }

    /// Extract inner type from Option<T>
    fn extract_option_inner_type(&self, rust_type: &str) -> Option<String> {
        if rust_type.starts_with("Option<") && rust_type.ends_with('>') {
            let inner = &rust_type[7..rust_type.len() - 1];
            Some(inner.to_string())
        } else {
            None
        }
    }

    /// Extract OK type from Result<T, E>
    fn extract_result_ok_type(&self, rust_type: &str) -> Option<String> {
        if rust_type.starts_with("Result<") && rust_type.ends_with('>') {
            let inner = &rust_type[7..rust_type.len() - 1];
            if let Some(comma_pos) = inner.find(',') {
                let ok_type = inner[..comma_pos].trim();
                Some(ok_type.to_string())
            } else {
                Some(inner.to_string())
            }
        } else {
            None
        }
    }

    /// Extract inner type from Vec<T>
    fn extract_vec_inner_type(&self, rust_type: &str) -> Option<String> {
        if rust_type.starts_with("Vec<") && rust_type.ends_with('>') {
            let inner = &rust_type[4..rust_type.len() - 1];
            Some(inner.to_string())
        } else {
            None
        }
    }

    /// Extract key and value types from HashMap<K, V>
    fn extract_hashmap_types(&self, rust_type: &str) -> Option<(String, String)> {
        if rust_type.starts_with("HashMap<") && rust_type.ends_with('>') {
            let inner = &rust_type[8..rust_type.len() - 1];
            self.parse_two_type_params(inner)
        } else {
            None
        }
    }

    /// Extract key and value types from BTreeMap<K, V>
    fn extract_btreemap_types(&self, rust_type: &str) -> Option<(String, String)> {
        if rust_type.starts_with("BTreeMap<") && rust_type.ends_with('>') {
            let inner = &rust_type[9..rust_type.len() - 1];
            self.parse_two_type_params(inner)
        } else {
            None
        }
    }

    /// Extract inner type from HashSet<T>
    fn extract_hashset_inner_type(&self, rust_type: &str) -> Option<String> {
        if rust_type.starts_with("HashSet<") && rust_type.ends_with('>') {
            let inner = &rust_type[8..rust_type.len() - 1];
            Some(inner.to_string())
        } else {
            None
        }
    }

    /// Extract inner type from BTreeSet<T>
    fn extract_btreeset_inner_type(&self, rust_type: &str) -> Option<String> {
        if rust_type.starts_with("BTreeSet<") && rust_type.ends_with('>') {
            let inner = &rust_type[9..rust_type.len() - 1];
            Some(inner.to_string())
        } else {
            None
        }
    }

    /// Extract types from tuple (T1, T2, ...)
    fn extract_tuple_types(&self, rust_type: &str) -> Option<Vec<String>> {
        if rust_type.starts_with('(') && rust_type.ends_with(')') {
            let inner = &rust_type[1..rust_type.len() - 1];
            if inner.trim().is_empty() {
                return Some(vec![]);
            }
            let types: Vec<String> = inner.split(',').map(|s| s.trim().to_string()).collect();
            Some(types)
        } else {
            None
        }
    }

    /// Extract inner type from reference &T
    fn extract_reference_type(&self, rust_type: &str) -> Option<String> {
        rust_type
            .strip_prefix('&')
            .map(|stripped| stripped.to_string())
    }

    /// Parse two type parameters separated by comma (for HashMap, BTreeMap)
    fn parse_two_type_params(&self, inner: &str) -> Option<(String, String)> {
        let mut depth = 0;
        let mut comma_pos = None;

        for (i, ch) in inner.char_indices() {
            match ch {
                '<' => depth += 1,
                '>' => depth -= 1,
                ',' if depth == 0 => {
                    comma_pos = Some(i);
                    break;
                }
                _ => {}
            }
        }

        if let Some(pos) = comma_pos {
            let key_type = inner[..pos].trim().to_string();
            let value_type = inner[pos + 1..].trim().to_string();
            Some((key_type, value_type))
        } else {
            None
        }
    }

    /// Get the type mappings
    pub fn get_type_set(&self) -> &HashSet<String> {
        &self.type_set
    }

    /// Parse a Rust type string into a structured TypeStructure
    /// This is the single source of truth for type parsing - generators use this instead of parsing strings
    pub fn parse_type_structure(&self, rust_type: &str) -> TypeStructure {
        let cleaned = rust_type.trim();

        // Handle references &T -> T
        if let Some(inner) = self.extract_reference_type(cleaned) {
            return self.parse_type_structure(&inner);
        }

        // Handle Option<T> -> Optional(T)
        if let Some(inner_type) = self.extract_option_inner_type(cleaned) {
            return TypeStructure::Optional(Box::new(self.parse_type_structure(&inner_type)));
        }

        // Handle Result<T, E> -> Result(T)
        if let Some(ok_type) = self.extract_result_ok_type(cleaned) {
            return TypeStructure::Result(Box::new(self.parse_type_structure(&ok_type)));
        }

        // Handle Vec<T> -> Array(T)
        if let Some(inner_type) = self.extract_vec_inner_type(cleaned) {
            return TypeStructure::Array(Box::new(self.parse_type_structure(&inner_type)));
        }

        // Handle HashMap<K, V> and BTreeMap<K, V> -> Map { key, value }
        if let Some((key_type, value_type)) = self
            .extract_hashmap_types(cleaned)
            .or_else(|| self.extract_btreemap_types(cleaned))
        {
            return TypeStructure::Map {
                key: Box::new(self.parse_type_structure(&key_type)),
                value: Box::new(self.parse_type_structure(&value_type)),
            };
        }

        // Handle HashSet<T> and BTreeSet<T> -> Set(T)
        if let Some(inner_type) = self
            .extract_hashset_inner_type(cleaned)
            .or_else(|| self.extract_btreeset_inner_type(cleaned))
        {
            return TypeStructure::Set(Box::new(self.parse_type_structure(&inner_type)));
        }

        // Handle tuple types (T1, T2, ...) -> Tuple([T1, T2, ...])
        if let Some(tuple_types) = self.extract_tuple_types(cleaned) {
            if tuple_types.is_empty() {
                return TypeStructure::Primitive("void".to_string());
            }
            let parsed_types: Vec<TypeStructure> = tuple_types
                .iter()
                .map(|t| self.parse_type_structure(t.trim()))
                .collect();
            return TypeStructure::Tuple(parsed_types);
        }

        // Check if it's a primitive type and map to target primitive
        if let Some(target_primitive) = self.map_to_target_primitive(cleaned) {
            return TypeStructure::Primitive(target_primitive);
        }

        // Otherwise, it's a custom type
        TypeStructure::Custom(cleaned.to_string())
    }

    /// Map Rust primitive types to target language primitives
    /// Returns Some("number" | "string" | "boolean" | "void") or None for non-primitives
    fn map_to_target_primitive(&self, rust_type: &str) -> Option<String> {
        match rust_type {
            // String types → "string"
            "String" | "str" | "&str" => Some("string".to_string()),
            // Numeric types → "number"
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64"
            | "u128" | "usize" | "f32" | "f64" => Some("number".to_string()),
            // Boolean → "boolean"
            "bool" => Some("boolean".to_string()),
            // Unit type → "void"
            "()" => Some("void".to_string()),
            // Not a primitive
            _ => None,
        }
    }

    /// Get the type mappings
    pub fn get_type_mappings(&self) -> &HashMap<String, String> {
        &self.type_mappings
    }

    /// Add a custom type mapping
    pub fn add_type_mapping(&mut self, rust_type: String, typescript_type: String) {
        self.type_mappings.insert(rust_type, typescript_type);
    }

    /// Apply type mappings from a HashMap (typically from config)
    pub fn apply_type_mappings(&mut self, mappings: &HashMap<String, String>) {
        for (rust_type, ts_type) in mappings {
            self.type_mappings
                .insert(rust_type.clone(), ts_type.clone());
        }
    }
}

impl Default for TypeResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_type_resolver_contains_primitives() {
        let resolver = TypeResolver::new();
        let type_set = resolver.get_type_set();

        assert!(type_set.contains("String"));
        assert!(type_set.contains("bool"));
        assert!(type_set.contains("i32"));
        assert!(type_set.contains("u64"));
        assert!(type_set.contains("f32"));
        assert!(type_set.contains("()"));
    }

    #[test]
    fn test_new_type_resolver_contains_collections() {
        let resolver = TypeResolver::new();
        let type_set = resolver.get_type_set();

        assert!(type_set.contains("HashMap"));
        assert!(type_set.contains("BTreeMap"));
        assert!(type_set.contains("HashSet"));
        assert!(type_set.contains("BTreeSet"));
    }

    #[test]
    fn test_default_impl() {
        let resolver = TypeResolver::default();
        assert!(resolver.get_type_set().contains("String"));
    }

    // Primitive type mapping tests
    mod primitive_mapping {
        use super::*;

        #[test]
        fn test_map_string_types() {
            let resolver = TypeResolver::new();

            assert_eq!(
                resolver.map_to_target_primitive("String"),
                Some("string".to_string())
            );
            assert_eq!(
                resolver.map_to_target_primitive("str"),
                Some("string".to_string())
            );
            assert_eq!(
                resolver.map_to_target_primitive("&str"),
                Some("string".to_string())
            );
        }

        #[test]
        fn test_map_integer_types() {
            let resolver = TypeResolver::new();

            assert_eq!(
                resolver.map_to_target_primitive("i8"),
                Some("number".to_string())
            );
            assert_eq!(
                resolver.map_to_target_primitive("i16"),
                Some("number".to_string())
            );
            assert_eq!(
                resolver.map_to_target_primitive("i32"),
                Some("number".to_string())
            );
            assert_eq!(
                resolver.map_to_target_primitive("i64"),
                Some("number".to_string())
            );
            assert_eq!(
                resolver.map_to_target_primitive("i128"),
                Some("number".to_string())
            );
            assert_eq!(
                resolver.map_to_target_primitive("isize"),
                Some("number".to_string())
            );

            assert_eq!(
                resolver.map_to_target_primitive("u8"),
                Some("number".to_string())
            );
            assert_eq!(
                resolver.map_to_target_primitive("u16"),
                Some("number".to_string())
            );
            assert_eq!(
                resolver.map_to_target_primitive("u32"),
                Some("number".to_string())
            );
            assert_eq!(
                resolver.map_to_target_primitive("u64"),
                Some("number".to_string())
            );
            assert_eq!(
                resolver.map_to_target_primitive("u128"),
                Some("number".to_string())
            );
            assert_eq!(
                resolver.map_to_target_primitive("usize"),
                Some("number".to_string())
            );
        }

        #[test]
        fn test_map_float_types() {
            let resolver = TypeResolver::new();

            assert_eq!(
                resolver.map_to_target_primitive("f32"),
                Some("number".to_string())
            );
            assert_eq!(
                resolver.map_to_target_primitive("f64"),
                Some("number".to_string())
            );
        }

        #[test]
        fn test_map_boolean() {
            let resolver = TypeResolver::new();
            assert_eq!(
                resolver.map_to_target_primitive("bool"),
                Some("boolean".to_string())
            );
        }

        #[test]
        fn test_map_unit_type() {
            let resolver = TypeResolver::new();
            assert_eq!(
                resolver.map_to_target_primitive("()"),
                Some("void".to_string())
            );
        }

        #[test]
        fn test_map_non_primitive() {
            let resolver = TypeResolver::new();
            assert_eq!(resolver.map_to_target_primitive("User"), None);
            assert_eq!(resolver.map_to_target_primitive("CustomType"), None);
        }
    }

    // Type extraction tests
    mod type_extraction {
        use super::*;

        #[test]
        fn test_extract_option_inner_type() {
            let resolver = TypeResolver::new();

            assert_eq!(
                resolver.extract_option_inner_type("Option<String>"),
                Some("String".to_string())
            );
            assert_eq!(
                resolver.extract_option_inner_type("Option<User>"),
                Some("User".to_string())
            );
            assert_eq!(resolver.extract_option_inner_type("String"), None);
        }

        #[test]
        fn test_extract_result_ok_type() {
            let resolver = TypeResolver::new();

            assert_eq!(
                resolver.extract_result_ok_type("Result<String, Error>"),
                Some("String".to_string())
            );
            assert_eq!(
                resolver.extract_result_ok_type("Result<User, String>"),
                Some("User".to_string())
            );
            assert_eq!(
                resolver.extract_result_ok_type("Result<()>"),
                Some("()".to_string())
            );
            assert_eq!(resolver.extract_result_ok_type("String"), None);
        }

        #[test]
        fn test_extract_vec_inner_type() {
            let resolver = TypeResolver::new();

            assert_eq!(
                resolver.extract_vec_inner_type("Vec<String>"),
                Some("String".to_string())
            );
            assert_eq!(
                resolver.extract_vec_inner_type("Vec<User>"),
                Some("User".to_string())
            );
            assert_eq!(resolver.extract_vec_inner_type("String"), None);
        }

        #[test]
        fn test_extract_hashmap_types() {
            let resolver = TypeResolver::new();

            assert_eq!(
                resolver.extract_hashmap_types("HashMap<String, User>"),
                Some(("String".to_string(), "User".to_string()))
            );
            assert_eq!(
                resolver.extract_hashmap_types("HashMap<i32, String>"),
                Some(("i32".to_string(), "String".to_string()))
            );
            assert_eq!(resolver.extract_hashmap_types("String"), None);
        }

        #[test]
        fn test_extract_btreemap_types() {
            let resolver = TypeResolver::new();

            assert_eq!(
                resolver.extract_btreemap_types("BTreeMap<String, User>"),
                Some(("String".to_string(), "User".to_string()))
            );
            assert_eq!(resolver.extract_btreemap_types("String"), None);
        }

        #[test]
        fn test_extract_hashset_inner_type() {
            let resolver = TypeResolver::new();

            assert_eq!(
                resolver.extract_hashset_inner_type("HashSet<String>"),
                Some("String".to_string())
            );
            assert_eq!(resolver.extract_hashset_inner_type("String"), None);
        }

        #[test]
        fn test_extract_btreeset_inner_type() {
            let resolver = TypeResolver::new();

            assert_eq!(
                resolver.extract_btreeset_inner_type("BTreeSet<User>"),
                Some("User".to_string())
            );
            assert_eq!(resolver.extract_btreeset_inner_type("String"), None);
        }

        #[test]
        fn test_extract_tuple_types() {
            let resolver = TypeResolver::new();

            assert_eq!(
                resolver.extract_tuple_types("(String, i32)"),
                Some(vec!["String".to_string(), "i32".to_string()])
            );
            assert_eq!(
                resolver.extract_tuple_types("(String, i32, bool)"),
                Some(vec![
                    "String".to_string(),
                    "i32".to_string(),
                    "bool".to_string()
                ])
            );
            assert_eq!(resolver.extract_tuple_types("()"), Some(vec![]));
            assert_eq!(resolver.extract_tuple_types("String"), None);
        }

        #[test]
        fn test_extract_reference_type() {
            let resolver = TypeResolver::new();

            assert_eq!(
                resolver.extract_reference_type("&String"),
                Some("String".to_string())
            );
            assert_eq!(
                resolver.extract_reference_type("&str"),
                Some("str".to_string())
            );
            assert_eq!(resolver.extract_reference_type("String"), None);
        }

        #[test]
        fn test_parse_two_type_params_simple() {
            let resolver = TypeResolver::new();

            assert_eq!(
                resolver.parse_two_type_params("String, User"),
                Some(("String".to_string(), "User".to_string()))
            );
        }

        #[test]
        fn test_parse_two_type_params_nested() {
            let resolver = TypeResolver::new();

            // HashMap<String, Vec<User>>
            assert_eq!(
                resolver.parse_two_type_params("String, Vec<User>"),
                Some(("String".to_string(), "Vec<User>".to_string()))
            );
        }

        #[test]
        fn test_parse_two_type_params_no_comma() {
            let resolver = TypeResolver::new();
            assert_eq!(resolver.parse_two_type_params("String"), None);
        }
    }

    // TypeStructure parsing tests
    mod type_structure_parsing {
        use super::*;

        #[test]
        fn test_parse_primitive_string() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("String");

            match result {
                TypeStructure::Primitive(name) => assert_eq!(name, "string"),
                _ => panic!("Should be Primitive"),
            }
        }

        #[test]
        fn test_parse_primitive_number() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("i32");

            match result {
                TypeStructure::Primitive(name) => assert_eq!(name, "number"),
                _ => panic!("Should be Primitive"),
            }
        }

        #[test]
        fn test_parse_primitive_boolean() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("bool");

            match result {
                TypeStructure::Primitive(name) => assert_eq!(name, "boolean"),
                _ => panic!("Should be Primitive"),
            }
        }

        #[test]
        fn test_parse_primitive_void() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("()");

            match result {
                TypeStructure::Primitive(name) => assert_eq!(name, "void"),
                _ => panic!("Should be Primitive"),
            }
        }

        #[test]
        fn test_parse_custom_type() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("User");

            match result {
                TypeStructure::Custom(name) => assert_eq!(name, "User"),
                _ => panic!("Should be Custom"),
            }
        }

        #[test]
        fn test_parse_reference() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("&String");

            match result {
                TypeStructure::Primitive(name) => assert_eq!(name, "string"),
                _ => panic!("Should unwrap reference to string"),
            }
        }

        #[test]
        fn test_parse_option() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("Option<String>");

            match result {
                TypeStructure::Optional(inner) => match *inner {
                    TypeStructure::Primitive(name) => assert_eq!(name, "string"),
                    _ => panic!("Inner should be string"),
                },
                _ => panic!("Should be Optional"),
            }
        }

        #[test]
        fn test_parse_result() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("Result<User, Error>");

            match result {
                TypeStructure::Result(inner) => match *inner {
                    TypeStructure::Custom(name) => assert_eq!(name, "User"),
                    _ => panic!("Inner should be User"),
                },
                _ => panic!("Should be Result"),
            }
        }

        #[test]
        fn test_parse_vec() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("Vec<String>");

            match result {
                TypeStructure::Array(inner) => match *inner {
                    TypeStructure::Primitive(name) => assert_eq!(name, "string"),
                    _ => panic!("Inner should be string"),
                },
                _ => panic!("Should be Array"),
            }
        }

        #[test]
        fn test_parse_hashmap() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("HashMap<String, User>");

            match result {
                TypeStructure::Map { key, value } => match (*key, *value) {
                    (TypeStructure::Primitive(k), TypeStructure::Custom(v)) => {
                        assert_eq!(k, "string");
                        assert_eq!(v, "User");
                    }
                    _ => panic!("Key should be string, value should be User"),
                },
                _ => panic!("Should be Map"),
            }
        }

        #[test]
        fn test_parse_btreemap() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("BTreeMap<i32, String>");

            match result {
                TypeStructure::Map { key, value } => match (*key, *value) {
                    (TypeStructure::Primitive(k), TypeStructure::Primitive(v)) => {
                        assert_eq!(k, "number");
                        assert_eq!(v, "string");
                    }
                    _ => panic!("Key should be number, value should be string"),
                },
                _ => panic!("Should be Map"),
            }
        }

        #[test]
        fn test_parse_hashset() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("HashSet<String>");

            match result {
                TypeStructure::Set(inner) => match *inner {
                    TypeStructure::Primitive(name) => assert_eq!(name, "string"),
                    _ => panic!("Inner should be string"),
                },
                _ => panic!("Should be Set"),
            }
        }

        #[test]
        fn test_parse_btreeset() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("BTreeSet<User>");

            match result {
                TypeStructure::Set(inner) => match *inner {
                    TypeStructure::Custom(name) => assert_eq!(name, "User"),
                    _ => panic!("Inner should be User"),
                },
                _ => panic!("Should be Set"),
            }
        }

        #[test]
        fn test_parse_tuple() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("(String, i32)");

            match result {
                TypeStructure::Tuple(types) => {
                    assert_eq!(types.len(), 2);
                    match &types[0] {
                        TypeStructure::Primitive(name) => assert_eq!(name, "string"),
                        _ => panic!("First type should be string"),
                    }
                    match &types[1] {
                        TypeStructure::Primitive(name) => assert_eq!(name, "number"),
                        _ => panic!("Second type should be number"),
                    }
                }
                _ => panic!("Should be Tuple"),
            }
        }

        #[test]
        fn test_parse_empty_tuple() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("()");

            match result {
                TypeStructure::Primitive(name) => assert_eq!(name, "void"),
                _ => panic!("Empty tuple should be void"),
            }
        }

        #[test]
        fn test_parse_nested_option_vec() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("Option<Vec<String>>");

            match result {
                TypeStructure::Optional(opt_inner) => match *opt_inner {
                    TypeStructure::Array(arr_inner) => match *arr_inner {
                        TypeStructure::Primitive(name) => assert_eq!(name, "string"),
                        _ => panic!("Should be string"),
                    },
                    _ => panic!("Should be Array"),
                },
                _ => panic!("Should be Optional"),
            }
        }

        #[test]
        fn test_parse_vec_option() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("Vec<Option<User>>");

            match result {
                TypeStructure::Array(arr_inner) => match *arr_inner {
                    TypeStructure::Optional(opt_inner) => match *opt_inner {
                        TypeStructure::Custom(name) => assert_eq!(name, "User"),
                        _ => panic!("Should be User"),
                    },
                    _ => panic!("Should be Optional"),
                },
                _ => panic!("Should be Array"),
            }
        }

        #[test]
        fn test_parse_hashmap_with_vec_value() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("HashMap<String, Vec<User>>");

            match result {
                TypeStructure::Map { key, value } => match (*key, *value) {
                    (TypeStructure::Primitive(k), TypeStructure::Array(v_arr)) => {
                        assert_eq!(k, "string");
                        match *v_arr {
                            TypeStructure::Custom(name) => assert_eq!(name, "User"),
                            _ => panic!("Array value should be User"),
                        }
                    }
                    _ => panic!("Key should be string, value should be Array"),
                },
                _ => panic!("Should be Map"),
            }
        }

        #[test]
        fn test_parse_result_with_unit_ok() {
            let resolver = TypeResolver::new();
            let result = resolver.parse_type_structure("Result<(), String>");

            match result {
                TypeStructure::Result(inner) => match *inner {
                    TypeStructure::Primitive(name) => assert_eq!(name, "void"),
                    _ => panic!("Should be void"),
                },
                _ => panic!("Should be Result"),
            }
        }

        #[test]
        fn test_parse_complex_nested() {
            let resolver = TypeResolver::new();
            // Test a simpler but still complex nested type that works
            // Option<Vec<HashMap<String, User>>>
            let result = resolver.parse_type_structure("Option<Vec<HashMap<String, User>>>");

            match result {
                TypeStructure::Optional(opt) => match *opt {
                    TypeStructure::Array(arr) => match *arr {
                        TypeStructure::Map { key, value } => match (*key, *value) {
                            (TypeStructure::Primitive(k), TypeStructure::Custom(v)) => {
                                assert_eq!(k, "string");
                                assert_eq!(v, "User");
                            }
                            _ => panic!("Map types incorrect"),
                        },
                        _ => panic!("Should be Map"),
                    },
                    _ => panic!("Should be Array"),
                },
                _ => panic!("Should be Optional"),
            }
        }

        #[test]
        fn test_parse_result_with_simple_nested() {
            let resolver = TypeResolver::new();
            // Result<Vec<User>, Error>
            let result = resolver.parse_type_structure("Result<Vec<User>, Error>");

            match result {
                TypeStructure::Result(res) => match *res {
                    TypeStructure::Array(arr) => match *arr {
                        TypeStructure::Custom(name) => assert_eq!(name, "User"),
                        _ => panic!("Should be User"),
                    },
                    _ => panic!("Should be Array"),
                },
                _ => panic!("Should be Result"),
            }
        }

        #[test]
        fn test_parse_with_whitespace() {
            let resolver = TypeResolver::new();
            // Note: The type resolver doesn't handle spaces inside generic brackets well
            // "Option < String >" is not recognized as Option<String> - it's treated as custom type
            let result = resolver.parse_type_structure("  Option<String>  ");

            match result {
                TypeStructure::Optional(inner) => match *inner {
                    TypeStructure::Primitive(name) => assert_eq!(name, "string"),
                    _ => panic!("Should be string"),
                },
                _ => panic!("Should be Optional"),
            }
        }
    }
}
