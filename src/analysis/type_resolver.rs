use crate::models::TypeStructure;
use std::collections::HashSet;

/// Type resolver for mapping Rust types to TypeScript types
#[derive(Debug)]
pub struct TypeResolver {
    type_set: HashSet<String>,
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

        Self { type_set }
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
}

impl Default for TypeResolver {
    fn default() -> Self {
        Self::new()
    }
}
