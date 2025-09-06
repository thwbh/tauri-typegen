use std::collections::HashMap;

/// Type resolver for mapping Rust types to TypeScript types
#[derive(Debug)]
pub struct TypeResolver {
    type_mappings: HashMap<String, String>,
}

impl TypeResolver {
    pub fn new() -> Self {
        let mut type_mappings = HashMap::new();

        // Basic Rust to TypeScript mappings
        type_mappings.insert("String".to_string(), "string".to_string());
        type_mappings.insert("&str".to_string(), "string".to_string());
        type_mappings.insert("str".to_string(), "string".to_string());
        type_mappings.insert("i8".to_string(), "number".to_string());
        type_mappings.insert("i16".to_string(), "number".to_string());
        type_mappings.insert("i32".to_string(), "number".to_string());
        type_mappings.insert("i64".to_string(), "number".to_string());
        type_mappings.insert("i128".to_string(), "number".to_string());
        type_mappings.insert("isize".to_string(), "number".to_string());
        type_mappings.insert("u8".to_string(), "number".to_string());
        type_mappings.insert("u16".to_string(), "number".to_string());
        type_mappings.insert("u32".to_string(), "number".to_string());
        type_mappings.insert("u64".to_string(), "number".to_string());
        type_mappings.insert("u128".to_string(), "number".to_string());
        type_mappings.insert("usize".to_string(), "number".to_string());
        type_mappings.insert("f32".to_string(), "number".to_string());
        type_mappings.insert("f64".to_string(), "number".to_string());
        type_mappings.insert("bool".to_string(), "boolean".to_string());
        type_mappings.insert("()".to_string(), "void".to_string());

        // Collection type mappings
        type_mappings.insert("HashMap".to_string(), "Map".to_string());
        type_mappings.insert("BTreeMap".to_string(), "Map".to_string());
        type_mappings.insert("HashSet".to_string(), "Set".to_string());
        type_mappings.insert("BTreeSet".to_string(), "Set".to_string());

        Self { type_mappings }
    }

    /// Map a Rust type string to TypeScript type string
    pub fn map_rust_type_to_typescript(&mut self, rust_type: &str) -> String {
        // Handle Option<T> -> T | null
        if let Some(inner_type) = self.extract_option_inner_type(rust_type) {
            let mapped_inner = self.map_rust_type_to_typescript(&inner_type);
            return format!("{} | null", mapped_inner);
        }

        // Handle Result<T, E> -> T (ignore error type for TypeScript)
        if let Some(ok_type) = self.extract_result_ok_type(rust_type) {
            return self.map_rust_type_to_typescript(&ok_type);
        }

        // Handle Vec<T> -> T[]
        if let Some(inner_type) = self.extract_vec_inner_type(rust_type) {
            let mapped_inner = self.map_rust_type_to_typescript(&inner_type);
            return format!("{}[]", mapped_inner);
        }

        // Handle HashMap<K, V> -> Map<K, V>
        if let Some((key_type, value_type)) = self.extract_hashmap_types(rust_type) {
            let mapped_key = self.map_rust_type_to_typescript(&key_type);
            let mapped_value = self.map_rust_type_to_typescript(&value_type);
            return format!("Map<{}, {}>", mapped_key, mapped_value);
        }

        // Handle BTreeMap<K, V> -> Map<K, V>
        if let Some((key_type, value_type)) = self.extract_btreemap_types(rust_type) {
            let mapped_key = self.map_rust_type_to_typescript(&key_type);
            let mapped_value = self.map_rust_type_to_typescript(&value_type);
            return format!("Map<{}, {}>", mapped_key, mapped_value);
        }

        // Handle HashSet<T> -> T[] (arrays for JSON compatibility)
        if let Some(inner_type) = self.extract_hashset_inner_type(rust_type) {
            let mapped_inner = self.map_rust_type_to_typescript(&inner_type);
            return format!("{}[]", mapped_inner);
        }

        // Handle BTreeSet<T> -> T[]
        if let Some(inner_type) = self.extract_btreeset_inner_type(rust_type) {
            let mapped_inner = self.map_rust_type_to_typescript(&inner_type);
            return format!("{}[]", mapped_inner);
        }

        // Handle tuple types (String, i32) -> [string, number]
        if let Some(tuple_types) = self.extract_tuple_types(rust_type) {
            if tuple_types.is_empty() {
                return "void".to_string();
            }
            let mapped_types: Vec<String> = tuple_types
                .iter()
                .map(|t| self.map_rust_type_to_typescript(t.trim()))
                .collect();
            return format!("[{}]", mapped_types.join(", "));
        }

        // Handle reference types &T -> T
        if let Some(inner_type) = self.extract_reference_type(rust_type) {
            return self.map_rust_type_to_typescript(&inner_type);
        }

        // Direct mapping lookup
        if let Some(mapped) = self.type_mappings.get(rust_type) {
            return mapped.clone();
        }

        // If no mapping found, assume it's a custom type and return as-is
        rust_type.to_string()
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
    pub fn get_type_mappings(&self) -> &HashMap<String, String> {
        &self.type_mappings
    }

    /// Add a custom type mapping
    pub fn add_type_mapping(&mut self, rust_type: String, typescript_type: String) {
        self.type_mappings.insert(rust_type, typescript_type);
    }
}

impl Default for TypeResolver {
    fn default() -> Self {
        Self::new()
    }
}
