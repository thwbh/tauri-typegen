use quote::ToTokens;
use syn::Attribute;

/// Parser for serde attributes from Rust struct/enum definitions and fields
#[derive(Debug)]
pub struct SerdeParser;

impl SerdeParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse struct-level serde attributes (e.g., rename_all)
    pub fn parse_struct_serde_attrs(&self, attrs: &[Attribute]) -> SerdeStructAttributes {
        let mut result = SerdeStructAttributes { rename_all: None };

        for attr in attrs {
            if attr.path().is_ident("serde") {
                if let Ok(tokens) = syn::parse2::<syn::MetaList>(attr.meta.to_token_stream()) {
                    let tokens_str = tokens.tokens.to_string();

                    // Parse rename_all = "convention"
                    if let Some(convention) = self.parse_rename_all(&tokens_str) {
                        result.rename_all = Some(convention);
                    }
                }
            }
        }

        result
    }

    /// Parse field-level serde attributes (e.g., rename, skip)
    pub fn parse_field_serde_attrs(&self, attrs: &[Attribute]) -> SerdeFieldAttributes {
        let mut result = SerdeFieldAttributes {
            rename: None,
            skip: false,
        };

        for attr in attrs {
            if attr.path().is_ident("serde") {
                if let Ok(tokens) = syn::parse2::<syn::MetaList>(attr.meta.to_token_stream()) {
                    let tokens_str = tokens.tokens.to_string();

                    // Check for skip flag
                    if tokens_str.contains("skip") && !tokens_str.contains("skip_serializing") {
                        result.skip = true;
                    }

                    // Parse rename = "value"
                    if let Some(rename) = self.parse_rename(&tokens_str) {
                        result.rename = Some(rename);
                    }
                }
            }
        }

        result
    }

    /// Parse rename_all value like "camelCase", "snake_case", "PascalCase", etc.
    fn parse_rename_all(&self, tokens: &str) -> Option<String> {
        if let Some(start) = tokens.find("rename_all") {
            if let Some(eq_pos) = tokens[start..].find('=') {
                let after_eq = &tokens[start + eq_pos + 1..].trim_start();

                // Extract value from quotes
                if let Some(quote_start) = after_eq.find('"') {
                    if let Some(quote_end) = after_eq[quote_start + 1..].find('"') {
                        let value = &after_eq[quote_start + 1..quote_start + 1 + quote_end];
                        return Some(value.to_string());
                    }
                }
            }
        }
        None
    }

    /// Parse rename value from field attribute
    fn parse_rename(&self, tokens: &str) -> Option<String> {
        // Look for "rename" but not "rename_all"
        let mut search_start = 0;
        while let Some(pos) = tokens[search_start..].find("rename") {
            let abs_pos = search_start + pos;

            // Check if this is followed by "_all"
            let after_rename = &tokens[abs_pos + 6..];
            if after_rename.trim_start().starts_with("_all") {
                // This is rename_all, skip it
                search_start = abs_pos + 10; // Move past "rename_all"
                continue;
            }

            // This is a plain "rename", extract the value
            if let Some(eq_pos) = after_rename.find('=') {
                let after_eq = &after_rename[eq_pos + 1..].trim_start();

                // Extract value from quotes
                if let Some(quote_start) = after_eq.find('"') {
                    if let Some(quote_end) = after_eq[quote_start + 1..].find('"') {
                        let value = &after_eq[quote_start + 1..quote_start + 1 + quote_end];
                        return Some(value.to_string());
                    }
                }
            }

            break;
        }
        None
    }
}

impl Default for SerdeParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Struct-level serde attributes
#[derive(Debug, Default, Clone)]
pub struct SerdeStructAttributes {
    pub rename_all: Option<String>,
}

/// Field-level serde attributes
#[derive(Debug, Default, Clone)]
pub struct SerdeFieldAttributes {
    pub rename: Option<String>,
    pub skip: bool,
}
