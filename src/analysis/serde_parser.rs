use quote::ToTokens;
use serde_rename_rule::RenameRule;
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

    /// Parse rename_all value like "camelCase", "snake_case", "PascalCase", etc. to
    /// find a matching `serde_rename_rule::RenameRule`.
    fn parse_rename_all(&self, tokens: &str) -> Option<RenameRule> {
        if let Some(start) = tokens.find("rename_all") {
            if let Some(eq_pos) = tokens[start..].find('=') {
                let after_eq = &tokens[start + eq_pos + 1..].trim_start();

                // Extract value from quotes
                if let Some(quote_start) = after_eq.find('"') {
                    if let Some(quote_end) = after_eq[quote_start + 1..].find('"') {
                        let value = &after_eq[quote_start + 1..quote_start + 1 + quote_end];

                        return RenameRule::from_rename_all_str(value).ok();
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
    pub rename_all: Option<RenameRule>,
}

/// Field-level serde attributes
#[derive(Debug, Default, Clone)]
pub struct SerdeFieldAttributes {
    pub rename: Option<String>,
    pub skip: bool,
}
#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_parse_rename_all_camel_case() {
        let parser = SerdeParser::new();
        let result = parser.parse_rename_all(r#"rename_all = "camelCase""#);

        assert!(result.is_some());
        assert!(matches!(result.unwrap(), RenameRule::CamelCase));
    }

    #[test]
    fn test_parse_rename_all_snake_case() {
        let parser = SerdeParser::new();
        let result = parser.parse_rename_all(r#"rename_all = "snake_case""#);

        assert!(result.is_some());
        assert!(matches!(result.unwrap(), RenameRule::SnakeCase));
    }

    #[test]
    fn test_parse_rename_all_pascal_case() {
        let parser = SerdeParser::new();
        let result = parser.parse_rename_all(r#"rename_all = "PascalCase""#);

        assert!(result.is_some());
        assert!(matches!(result.unwrap(), RenameRule::PascalCase));
    }

    #[test]
    fn test_parse_rename_all_screaming_snake_case() {
        let parser = SerdeParser::new();
        let result = parser.parse_rename_all(r#"rename_all = "SCREAMING_SNAKE_CASE""#);

        assert!(result.is_some());
        assert!(matches!(result.unwrap(), RenameRule::ScreamingSnakeCase));
    }

    #[test]
    fn test_parse_rename_all_kebab_case() {
        let parser = SerdeParser::new();
        let result = parser.parse_rename_all(r#"rename_all = "kebab-case""#);

        assert!(result.is_some());
        assert!(matches!(result.unwrap(), RenameRule::KebabCase));
    }

    #[test]
    fn test_parse_rename_all_not_present() {
        let parser = SerdeParser::new();
        let result = parser.parse_rename_all(r#"skip_serializing_if = "Option::is_none""#);

        assert!(result.is_none());
    }

    #[test]
    fn test_parse_rename() {
        let parser = SerdeParser::new();

        let result = parser.parse_rename(r#"rename = "customName""#);
        assert_eq!(result, Some("customName".to_string()));

        let result = parser.parse_rename(r#"rename = "id""#);
        assert_eq!(result, Some("id".to_string()));
    }

    #[test]
    fn test_parse_rename_not_rename_all() {
        let parser = SerdeParser::new();

        // Should not match rename_all
        let result = parser.parse_rename(r#"rename_all = "camelCase""#);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_rename_with_rename_all_present() {
        let parser = SerdeParser::new();

        // Should find "rename" even if rename_all is also present
        let result = parser.parse_rename(r#"rename_all = "camelCase", rename = "id""#);
        assert_eq!(result, Some("id".to_string()));
    }

    #[test]
    fn test_parse_struct_serde_attrs_with_rename_all() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![parse_quote!(#[serde(rename_all = "camelCase")])];

        let result = parser.parse_struct_serde_attrs(&attrs);
        assert!(result.rename_all.is_some());
        assert!(matches!(result.rename_all.unwrap(), RenameRule::CamelCase));
    }

    #[test]
    fn test_parse_struct_serde_attrs_no_serde() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![parse_quote!(#[derive(Debug)])];

        let result = parser.parse_struct_serde_attrs(&attrs);
        assert!(result.rename_all.is_none());
    }

    #[test]
    fn test_parse_field_serde_attrs_with_rename() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![parse_quote!(#[serde(rename = "customName")])];

        let result = parser.parse_field_serde_attrs(&attrs);
        assert_eq!(result.rename, Some("customName".to_string()));
        assert!(!result.skip);
    }

    #[test]
    fn test_parse_field_serde_attrs_with_skip() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![parse_quote!(#[serde(skip)])];

        let result = parser.parse_field_serde_attrs(&attrs);
        assert!(result.skip);
        assert!(result.rename.is_none());
    }

    #[test]
    fn test_parse_field_serde_attrs_skip_serializing_not_skip() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![parse_quote!(#[serde(skip_serializing)])];

        let result = parser.parse_field_serde_attrs(&attrs);
        // skip_serializing should not set skip flag
        assert!(!result.skip);
    }

    #[test]
    fn test_parse_field_serde_attrs_multiple() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![
            parse_quote!(#[serde(rename = "id")]),
            parse_quote!(#[derive(Debug)]),
        ];

        let result = parser.parse_field_serde_attrs(&attrs);
        assert_eq!(result.rename, Some("id".to_string()));
    }

    #[test]
    fn test_parse_field_serde_attrs_no_serde() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![parse_quote!(#[derive(Debug)])];

        let result = parser.parse_field_serde_attrs(&attrs);
        assert!(result.rename.is_none());
        assert!(!result.skip);
    }

    #[test]
    fn test_default_impl() {
        let parser = SerdeParser;
        let result = parser.parse_rename(r#"rename = "test""#);
        assert_eq!(result, Some("test".to_string()));
    }
}
