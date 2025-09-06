use crate::models::StructInfo;
use std::collections::HashMap;

/// Common type conversion utilities shared between generators
pub struct TypeConverter {
    /// Map of known custom types
    known_types: HashMap<String, StructInfo>,
}

impl TypeConverter {
    pub fn new() -> Self {
        Self {
            known_types: HashMap::new(),
        }
    }

    pub fn with_known_types(known_types: HashMap<String, StructInfo>) -> Self {
        Self { known_types }
    }

    /// Update the known types registry
    pub fn set_known_types(&mut self, known_types: HashMap<String, StructInfo>) {
        self.known_types = known_types;
    }

    /// Check if a type is a custom/user-defined type
    pub fn is_custom_type(&self, type_name: &str) -> bool {
        self.known_types.contains_key(type_name)
    }

    /// Get information about a custom type
    pub fn get_custom_type(&self, type_name: &str) -> Option<&StructInfo> {
        self.known_types.get(type_name)
    }

    /// Extract the inner type from Result<T, E> -> T
    pub fn extract_result_ok_type(&self, type_str: &str) -> Option<String> {
        if type_str.starts_with("Result<") && type_str.ends_with('>') {
            let inner = &type_str[7..type_str.len() - 1];
            if let Some(comma_pos) = inner.find(',') {
                Some(inner[..comma_pos].trim().to_string())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Extract both types from Result<T, E> -> (T, E)
    pub fn extract_result_types(&self, type_str: &str) -> Option<(String, String)> {
        if type_str.starts_with("Result<") && type_str.ends_with('>') {
            let inner = &type_str[7..type_str.len() - 1];
            if let Some(comma_pos) = inner.find(',') {
                let ok_type = inner[..comma_pos].trim().to_string();
                let err_type = inner[comma_pos + 1..].trim().to_string();
                Some((ok_type, err_type))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Extract the inner type from Option<T> -> T
    pub fn extract_option_inner_type(&self, type_str: &str) -> Option<String> {
        if type_str.starts_with("Option<") && type_str.ends_with('>') {
            let inner = &type_str[7..type_str.len() - 1];
            Some(inner.trim().to_string())
        } else {
            None
        }
    }

    /// Extract the inner type from Vec<T> -> T
    pub fn extract_vec_inner_type(&self, type_str: &str) -> Option<String> {
        if type_str.starts_with("Vec<") && type_str.ends_with('>') {
            let inner = &type_str[4..type_str.len() - 1];
            Some(inner.trim().to_string())
        } else {
            None
        }
    }

    /// Extract key and value types from HashMap<K, V> -> (K, V)
    pub fn extract_hashmap_types(&self, type_str: &str) -> Option<(String, String)> {
        if type_str.starts_with("HashMap<") && type_str.ends_with('>') {
            let inner = &type_str[8..type_str.len() - 1];
            if let Some(comma_pos) = inner.find(',') {
                let key_type = inner[..comma_pos].trim().to_string();
                let value_type = inner[comma_pos + 1..].trim().to_string();
                Some((key_type, value_type))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Extract key and value types from BTreeMap<K, V> -> (K, V) 
    pub fn extract_btreemap_types(&self, type_str: &str) -> Option<(String, String)> {
        if type_str.starts_with("BTreeMap<") && type_str.ends_with('>') {
            let inner = &type_str[9..type_str.len() - 1];
            if let Some(comma_pos) = inner.find(',') {
                let key_type = inner[..comma_pos].trim().to_string();
                let value_type = inner[comma_pos + 1..].trim().to_string();
                Some((key_type, value_type))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Extract the inner type from HashSet<T> -> T
    pub fn extract_hashset_inner_type(&self, type_str: &str) -> Option<String> {
        if type_str.starts_with("HashSet<") && type_str.ends_with('>') {
            let inner = &type_str[8..type_str.len() - 1];
            Some(inner.trim().to_string())
        } else {
            None
        }
    }

    /// Extract the inner type from BTreeSet<T> -> T
    pub fn extract_btreeset_inner_type(&self, type_str: &str) -> Option<String> {
        if type_str.starts_with("BTreeSet<") && type_str.ends_with('>') {
            let inner = &type_str[9..type_str.len() - 1];
            Some(inner.trim().to_string())
        } else {
            None
        }
    }

    /// Parse tuple types like (T1, T2, T3) -> Vec<String>
    pub fn extract_tuple_types(&self, type_str: &str) -> Option<Vec<String>> {
        if type_str.starts_with('(') && type_str.ends_with(')') {
            let inner = &type_str[1..type_str.len() - 1];
            if inner.trim().is_empty() {
                // Unit type ()
                Some(vec![])
            } else {
                let types = inner
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
                Some(types)
            }
        } else {
            None
        }
    }

    /// Remove reference markers (&, &mut) from types
    pub fn strip_reference(&self, type_str: &str) -> String {
        type_str
            .trim_start_matches("&mut ")
            .trim_start_matches('&')
            .trim()
            .to_string()
    }

    /// Check if a Rust type is a primitive type
    pub fn is_primitive_type(&self, type_str: &str) -> bool {
        matches!(
            type_str,
            "String" | "str" | "&str" | "&String" |
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" |
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize" |
            "f32" | "f64" |
            "bool" |
            "()" | "number" | "string" | "boolean" | "void" | "unknown"
        )
    }

    /// Map primitive Rust types to TypeScript types
    pub fn map_primitive_type(&self, rust_type: &str) -> Option<String> {
        match rust_type {
            "String" | "str" | "&str" | "&String" => Some("string".to_string()),
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" |
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize" |
            "f32" | "f64" => Some("number".to_string()),
            "bool" => Some("boolean".to_string()),
            "()" => Some("void".to_string()),
            _ => None,
        }
    }

    /// Recursively collect all referenced type names from a complex type
    pub fn collect_referenced_types(&self, type_str: &str, used_types: &mut std::collections::HashSet<String>) {
        let cleaned_type = self.strip_reference(type_str);

        // Handle Result<T, E>
        if let Some((ok_type, err_type)) = self.extract_result_types(&cleaned_type) {
            self.collect_referenced_types(&ok_type, used_types);
            self.collect_referenced_types(&err_type, used_types);
            return;
        }

        // Handle Option<T>
        if let Some(inner) = self.extract_option_inner_type(&cleaned_type) {
            self.collect_referenced_types(&inner, used_types);
            return;
        }

        // Handle Vec<T>
        if let Some(inner) = self.extract_vec_inner_type(&cleaned_type) {
            self.collect_referenced_types(&inner, used_types);
            return;
        }

        // Handle HashMap<K, V> and BTreeMap<K, V>
        if let Some((key_type, value_type)) = self.extract_hashmap_types(&cleaned_type)
            .or_else(|| self.extract_btreemap_types(&cleaned_type))
        {
            self.collect_referenced_types(&key_type, used_types);
            self.collect_referenced_types(&value_type, used_types);
            return;
        }

        // Handle HashSet<T> and BTreeSet<T>
        if let Some(inner) = self.extract_hashset_inner_type(&cleaned_type)
            .or_else(|| self.extract_btreeset_inner_type(&cleaned_type))
        {
            self.collect_referenced_types(&inner, used_types);
            return;
        }

        // Handle tuple types
        if let Some(tuple_types) = self.extract_tuple_types(&cleaned_type) {
            for tuple_type in tuple_types {
                self.collect_referenced_types(&tuple_type, used_types);
            }
            return;
        }

        // If it's not a primitive type, add it as a custom type
        if !self.is_primitive_type(&cleaned_type) {
            used_types.insert(cleaned_type);
        }
    }

    /// Generate a parameter name for a command from its Rust type
    pub fn generate_param_name(&self, rust_type: &str) -> String {
        let cleaned = self.strip_reference(rust_type);
        
        // Extract base type name for parameter naming
        if let Some(inner) = self.extract_option_inner_type(&cleaned) {
            return self.generate_param_name(&inner);
        }
        
        if let Some(inner) = self.extract_vec_inner_type(&cleaned) {
            return format!("{}List", self.generate_param_name(&inner));
        }
        
        if let Some(ok_type) = self.extract_result_ok_type(&cleaned) {
            return self.generate_param_name(&ok_type);
        }
        
        // Convert PascalCase to camelCase
        let mut result = String::new();
        let mut chars = cleaned.chars().peekable();
        
        if let Some(first) = chars.next() {
            result.push(first.to_lowercase().next().unwrap_or(first));
            
            for ch in chars {
                result.push(ch);
            }
        }
        
        result
    }
}

impl Default for TypeConverter {
    fn default() -> Self {
        Self::new()
    }
}