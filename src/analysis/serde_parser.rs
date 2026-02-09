use serde_rename_rule::RenameRule;
use syn::{Attribute, Expr, ExprLit, Lit};

/// Parser for serde attributes from Rust struct/enum definitions and fields
#[derive(Debug)]
pub struct SerdeParser;

impl SerdeParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse struct-level serde attributes (e.g., rename_all, tag, content)
    pub fn parse_struct_serde_attrs(&self, attrs: &[Attribute]) -> SerdeStructAttributes {
        let mut result = SerdeStructAttributes {
            rename_all: None,
            tag: None,
            content: None,
        };

        for attr in attrs {
            if attr.path().is_ident("serde") {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename_all") {
                        if let Some(value) = parse_string_value(&meta)? {
                            result.rename_all = RenameRule::from_rename_all_str(&value).ok();
                        }
                    } else if meta.path.is_ident("tag") {
                        result.tag = parse_string_value(&meta)?;
                    } else if meta.path.is_ident("content") {
                        result.content = parse_string_value(&meta)?;
                    }
                    Ok(())
                });
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
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename") {
                        result.rename = parse_string_value(&meta)?;
                    } else if meta.path.is_ident("skip") {
                        // skip is a flag, no value needed
                        result.skip = true;
                    }
                    // Note: skip_serializing and skip_deserializing are NOT the same as skip
                    // They only affect one direction, so we don't set the skip flag for them
                    Ok(())
                });
            }
        }

        result
    }
}

/// Parse a string value from a meta item like `name = "value"`
fn parse_string_value(meta: &syn::meta::ParseNestedMeta) -> syn::Result<Option<String>> {
    let expr: Expr = meta.value()?.parse()?;
    if let Expr::Lit(ExprLit {
        lit: Lit::Str(lit_str),
        ..
    }) = expr
    {
        Ok(Some(lit_str.value()))
    } else {
        Ok(None)
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
    /// Tag attribute for internally-tagged enum representation: #[serde(tag = "type")]
    pub tag: Option<String>,
    /// Content attribute for adjacently-tagged enum representation: #[serde(content = "data")]
    pub content: Option<String>,
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
    fn test_parse_struct_serde_attrs_with_rename_all_camel_case() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![parse_quote!(#[serde(rename_all = "camelCase")])];

        let result = parser.parse_struct_serde_attrs(&attrs);
        assert!(result.rename_all.is_some());
        assert!(matches!(result.rename_all.unwrap(), RenameRule::CamelCase));
    }

    #[test]
    fn test_parse_struct_serde_attrs_with_rename_all_snake_case() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![parse_quote!(#[serde(rename_all = "snake_case")])];

        let result = parser.parse_struct_serde_attrs(&attrs);
        assert!(result.rename_all.is_some());
        assert!(matches!(result.rename_all.unwrap(), RenameRule::SnakeCase));
    }

    #[test]
    fn test_parse_struct_serde_attrs_with_rename_all_pascal_case() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![parse_quote!(#[serde(rename_all = "PascalCase")])];

        let result = parser.parse_struct_serde_attrs(&attrs);
        assert!(result.rename_all.is_some());
        assert!(matches!(result.rename_all.unwrap(), RenameRule::PascalCase));
    }

    #[test]
    fn test_parse_struct_serde_attrs_with_rename_all_screaming_snake_case() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> =
            vec![parse_quote!(#[serde(rename_all = "SCREAMING_SNAKE_CASE")])];

        let result = parser.parse_struct_serde_attrs(&attrs);
        assert!(result.rename_all.is_some());
        assert!(matches!(
            result.rename_all.unwrap(),
            RenameRule::ScreamingSnakeCase
        ));
    }

    #[test]
    fn test_parse_struct_serde_attrs_with_rename_all_kebab_case() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![parse_quote!(#[serde(rename_all = "kebab-case")])];

        let result = parser.parse_struct_serde_attrs(&attrs);
        assert!(result.rename_all.is_some());
        assert!(matches!(result.rename_all.unwrap(), RenameRule::KebabCase));
    }

    #[test]
    fn test_parse_struct_serde_attrs_no_serde() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![parse_quote!(#[derive(Debug)])];

        let result = parser.parse_struct_serde_attrs(&attrs);
        assert!(result.rename_all.is_none());
        assert!(result.tag.is_none());
        assert!(result.content.is_none());
    }

    #[test]
    fn test_parse_struct_serde_attrs_with_tag() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![parse_quote!(#[serde(tag = "type")])];

        let result = parser.parse_struct_serde_attrs(&attrs);
        assert_eq!(result.tag, Some("type".to_string()));
    }

    #[test]
    fn test_parse_struct_serde_attrs_with_custom_tag() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![parse_quote!(#[serde(tag = "kind")])];

        let result = parser.parse_struct_serde_attrs(&attrs);
        assert_eq!(result.tag, Some("kind".to_string()));
    }

    #[test]
    fn test_parse_struct_serde_attrs_with_content() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![parse_quote!(#[serde(content = "data")])];

        let result = parser.parse_struct_serde_attrs(&attrs);
        assert_eq!(result.content, Some("data".to_string()));
    }

    #[test]
    fn test_parse_struct_serde_attrs_with_tag_and_content() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![parse_quote!(#[serde(tag = "kind", content = "data")])];

        let result = parser.parse_struct_serde_attrs(&attrs);
        assert_eq!(result.tag, Some("kind".to_string()));
        assert_eq!(result.content, Some("data".to_string()));
    }

    #[test]
    fn test_parse_struct_serde_attrs_with_all_attributes() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> =
            vec![parse_quote!(#[serde(rename_all = "camelCase", tag = "type", content = "value")])];

        let result = parser.parse_struct_serde_attrs(&attrs);
        assert!(matches!(result.rename_all, Some(RenameRule::CamelCase)));
        assert_eq!(result.tag, Some("type".to_string()));
        assert_eq!(result.content, Some("value".to_string()));
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
    fn test_parse_field_serde_attrs_skip_deserializing_not_skip() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![parse_quote!(#[serde(skip_deserializing)])];

        let result = parser.parse_field_serde_attrs(&attrs);
        // skip_deserializing should not set skip flag
        assert!(!result.skip);
    }

    #[test]
    fn test_parse_field_serde_attrs_multiple_attributes() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![
            parse_quote!(#[serde(rename = "id")]),
            parse_quote!(#[derive(Debug)]),
        ];

        let result = parser.parse_field_serde_attrs(&attrs);
        assert_eq!(result.rename, Some("id".to_string()));
    }

    #[test]
    fn test_parse_field_serde_attrs_rename_and_skip() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![parse_quote!(#[serde(rename = "id", skip)])];

        let result = parser.parse_field_serde_attrs(&attrs);
        assert_eq!(result.rename, Some("id".to_string()));
        assert!(result.skip);
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
    fn test_parse_field_serde_attrs_empty() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![];

        let result = parser.parse_field_serde_attrs(&attrs);
        assert!(result.rename.is_none());
        assert!(!result.skip);
    }

    #[test]
    fn test_default_impl() {
        let parser = SerdeParser::default();
        let attrs: Vec<Attribute> = vec![parse_quote!(#[serde(rename = "test")])];
        let result = parser.parse_field_serde_attrs(&attrs);
        assert_eq!(result.rename, Some("test".to_string()));
    }

    #[test]
    fn test_parse_struct_serde_attrs_ignores_other_attributes() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![parse_quote!(
            #[serde(rename_all = "camelCase", deny_unknown_fields)]
        )];

        let result = parser.parse_struct_serde_attrs(&attrs);
        assert!(matches!(result.rename_all, Some(RenameRule::CamelCase)));
        // deny_unknown_fields is ignored, no error
    }

    #[test]
    fn test_parse_field_serde_attrs_ignores_other_attributes() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![
            parse_quote!(#[serde(rename = "id", default, skip_serializing_if = "Option::is_none")]),
        ];

        let result = parser.parse_field_serde_attrs(&attrs);
        assert_eq!(result.rename, Some("id".to_string()));
        // default and skip_serializing_if are ignored, no error
    }

    #[test]
    fn test_parse_multiple_serde_attributes() {
        let parser = SerdeParser::new();
        let attrs: Vec<Attribute> = vec![
            parse_quote!(#[serde(rename_all = "camelCase")]),
            parse_quote!(#[serde(tag = "type")]),
        ];

        let result = parser.parse_struct_serde_attrs(&attrs);
        assert!(matches!(result.rename_all, Some(RenameRule::CamelCase)));
        assert_eq!(result.tag, Some("type".to_string()));
    }
}
