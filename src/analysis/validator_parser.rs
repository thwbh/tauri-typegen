use crate::models::{LengthConstraint, RangeConstraint, ValidatorAttributes};
use quote::ToTokens;
use syn::Attribute;

/// Parser for validator attributes from Rust struct fields
#[derive(Debug)]
pub struct ValidatorParser;

impl ValidatorParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse validator attributes from field attributes
    pub fn parse_validator_attributes(&self, attrs: &[Attribute]) -> Option<ValidatorAttributes> {
        let mut validator_attrs = ValidatorAttributes {
            length: None,
            range: None,
            email: false,
            url: false,
            custom_message: None,
        };

        let mut found_validator = false;

        for attr in attrs {
            if attr.path().is_ident("validate") {
                found_validator = true;
                // Parse the tokens inside the validate attribute
                if let Ok(tokens) = syn::parse2::<syn::MetaList>(attr.meta.to_token_stream()) {
                    // Convert tokens to string and do basic parsing for now
                    let tokens_str = tokens.tokens.to_string();

                    if tokens_str.contains("email") {
                        validator_attrs.email = true;
                    }

                    if tokens_str.contains("url") {
                        validator_attrs.url = true;
                    }

                    // Parse length constraints
                    if let Some(length_constraint) = self.parse_length_from_tokens(&tokens_str) {
                        validator_attrs.length = Some(length_constraint);
                    }

                    // Parse range constraints
                    if let Some(range_constraint) = self.parse_range_from_tokens(&tokens_str) {
                        validator_attrs.range = Some(range_constraint);
                    }
                }
            }
        }

        if found_validator {
            Some(validator_attrs)
        } else {
            None
        }
    }

    /// Parse length constraints from validator tokens
    fn parse_length_from_tokens(&self, tokens: &str) -> Option<LengthConstraint> {
        if !tokens.contains("length") {
            return None;
        }

        let mut constraint = LengthConstraint {
            min: None,
            max: None,
            message: None,
        };

        // Simple regex-like parsing for length(min = X, max = Y, message = "...")
        if let Some(start) = tokens.find("length") {
            if let Some(paren_start) = tokens[start..].find('(') {
                if let Some(paren_end) = tokens[start + paren_start..].find(')') {
                    let content = &tokens[start + paren_start + 1..start + paren_start + paren_end];

                    // Parse min = value
                    if let Some(min_pos) = content.find("min") {
                        if let Some(eq_pos) = content[min_pos..].find('=') {
                            let after_eq = &content[min_pos + eq_pos + 1..];
                            if let Some(comma_pos) = after_eq.find(',') {
                                let value_str = after_eq[..comma_pos].trim();
                                if let Ok(value) = value_str.parse::<u64>() {
                                    constraint.min = Some(value);
                                }
                            } else {
                                let value_str = after_eq.trim();
                                if let Ok(value) = value_str.parse::<u64>() {
                                    constraint.min = Some(value);
                                }
                            }
                        }
                    }

                    // Parse max = value
                    if let Some(max_pos) = content.find("max") {
                        if let Some(eq_pos) = content[max_pos..].find('=') {
                            let after_eq = &content[max_pos + eq_pos + 1..];
                            if let Some(comma_pos) = after_eq.find(',') {
                                let value_str = after_eq[..comma_pos].trim();
                                if let Ok(value) = value_str.parse::<u64>() {
                                    constraint.max = Some(value);
                                }
                            } else {
                                let value_str = after_eq.trim();
                                if let Ok(value) = value_str.parse::<u64>() {
                                    constraint.max = Some(value);
                                }
                            }
                        }
                    }

                    // Parse message = "..."
                    constraint.message = self.parse_message_from_content(content);
                }
            }
        }

        Some(constraint)
    }

    /// Parse range constraints from validator tokens
    fn parse_range_from_tokens(&self, tokens: &str) -> Option<RangeConstraint> {
        if !tokens.contains("range") {
            return None;
        }

        let mut constraint = RangeConstraint {
            min: None,
            max: None,
            message: None,
        };

        // Simple regex-like parsing for range(min = X, max = Y, message = "...")
        if let Some(start) = tokens.find("range") {
            if let Some(paren_start) = tokens[start..].find('(') {
                if let Some(paren_end) = tokens[start + paren_start..].find(')') {
                    let content = &tokens[start + paren_start + 1..start + paren_start + paren_end];

                    // Parse min = value
                    if let Some(min_pos) = content.find("min") {
                        if let Some(eq_pos) = content[min_pos..].find('=') {
                            let after_eq = &content[min_pos + eq_pos + 1..];
                            if let Some(comma_pos) = after_eq.find(',') {
                                let value_str = after_eq[..comma_pos].trim();
                                if let Ok(value) = value_str.parse::<f64>() {
                                    constraint.min = Some(value);
                                }
                            } else {
                                let value_str = after_eq.trim();
                                if let Ok(value) = value_str.parse::<f64>() {
                                    constraint.min = Some(value);
                                }
                            }
                        }
                    }

                    // Parse max = value
                    if let Some(max_pos) = content.find("max") {
                        if let Some(eq_pos) = content[max_pos..].find('=') {
                            let after_eq = &content[max_pos + eq_pos + 1..];
                            if let Some(comma_pos) = after_eq.find(',') {
                                let value_str = after_eq[..comma_pos].trim();
                                if let Ok(value) = value_str.parse::<f64>() {
                                    constraint.max = Some(value);
                                }
                            } else {
                                let value_str = after_eq.trim();
                                if let Ok(value) = value_str.parse::<f64>() {
                                    constraint.max = Some(value);
                                }
                            }
                        }
                    }

                    // Parse message = "..."
                    constraint.message = self.parse_message_from_content(content);
                }
            }
        }

        Some(constraint)
    }

    /// Parse message parameter from validator content
    /// Handles both "message = \"text\"" and "message = 'text'" formats
    fn parse_message_from_content(&self, content: &str) -> Option<String> {
        if let Some(msg_pos) = content.find("message") {
            if let Some(eq_pos) = content[msg_pos..].find('=') {
                let after_eq = &content[msg_pos + eq_pos + 1..].trim_start();

                // Try to find string in quotes (either " or ')
                if let Some(quote_char) = after_eq.chars().next() {
                    if quote_char == '"' || quote_char == '\'' {
                        // Find the closing quote, handling escaped quotes
                        let rest = &after_eq[1..];
                        let chars = rest.chars().enumerate();
                        let mut escaped = false;

                        for (i, ch) in chars {
                            if escaped {
                                escaped = false;
                                continue;
                            }
                            if ch == '\\' {
                                escaped = true;
                                continue;
                            }
                            if ch == quote_char {
                                // Found closing quote
                                let message = &rest[..i];
                                // Unescape common escape sequences
                                let unescaped = message
                                    .replace("\\\"", "\"")
                                    .replace("\\'", "'")
                                    .replace("\\n", "\n")
                                    .replace("\\t", "\t")
                                    .replace("\\\\", "\\");
                                return Some(unescaped);
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

impl Default for ValidatorParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, Field};

    // Helper to create a test validator parser
    fn parser() -> ValidatorParser {
        ValidatorParser::new()
    }

    mod basic_validators {
        use super::*;

        #[test]
        fn test_no_validator_attribute() {
            let parser = parser();
            let field: Field = parse_quote!(pub name: String);
            let result = parser.parse_validator_attributes(&field.attrs);
            assert!(result.is_none());
        }

        #[test]
        fn test_email_validator() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(email)]
                pub email: String
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            assert!(result.email);
            assert!(!result.url);
        }

        #[test]
        fn test_url_validator() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(url)]
                pub website: String
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            assert!(result.url);
            assert!(!result.email);
        }

        #[test]
        fn test_email_and_url_validator() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(email, url)]
                pub field: String
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            assert!(result.email);
            assert!(result.url);
        }

        #[test]
        fn test_non_validator_attribute() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[serde(rename = "userName")]
                pub user_name: String
            };
            let result = parser.parse_validator_attributes(&field.attrs);
            assert!(result.is_none());
        }
    }

    mod length_constraints {
        use super::*;

        #[test]
        fn test_length_with_min() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(length(min = 5))]
                pub password: String
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            assert!(result.length.is_some());
            let length = result.length.unwrap();
            assert_eq!(length.min, Some(5));
            assert_eq!(length.max, None);
        }

        #[test]
        fn test_length_with_max() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(length(max = 100))]
                pub username: String
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            assert!(result.length.is_some());
            let length = result.length.unwrap();
            assert_eq!(length.min, None);
            assert_eq!(length.max, Some(100));
        }

        #[test]
        fn test_length_with_min_and_max() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(length(min = 5, max = 100))]
                pub bio: String
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            assert!(result.length.is_some());
            let length = result.length.unwrap();
            assert_eq!(length.min, Some(5));
            assert_eq!(length.max, Some(100));
        }

        #[test]
        fn test_length_with_message() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(length(min = 5, message = "Must be at least 5 chars"))]
                pub field: String
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            assert!(result.length.is_some());
            let length = result.length.unwrap();
            assert_eq!(length.message, Some("Must be at least 5 chars".to_string()));
        }

        #[test]
        fn test_length_with_all_params() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(length(min = 1, max = 50, message = "Between 1 and 50"))]
                pub field: String
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            let length = result.length.unwrap();
            assert_eq!(length.min, Some(1));
            assert_eq!(length.max, Some(50));
            assert_eq!(length.message, Some("Between 1 and 50".to_string()));
        }
    }

    mod range_constraints {
        use super::*;

        #[test]
        fn test_range_with_min() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(range(min = 0))]
                pub age: i32
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            assert!(result.range.is_some());
            let range = result.range.unwrap();
            assert_eq!(range.min, Some(0.0));
            assert_eq!(range.max, None);
        }

        #[test]
        fn test_range_with_max() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(range(max = 100))]
                pub score: i32
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            assert!(result.range.is_some());
            let range = result.range.unwrap();
            assert_eq!(range.min, None);
            assert_eq!(range.max, Some(100.0));
        }

        #[test]
        fn test_range_with_min_and_max() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(range(min = 1, max = 10))]
                pub rating: i32
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            assert!(result.range.is_some());
            let range = result.range.unwrap();
            assert_eq!(range.min, Some(1.0));
            assert_eq!(range.max, Some(10.0));
        }

        #[test]
        fn test_range_with_decimal_values() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(range(min = 0.5, max = 99.9))]
                pub percentage: f64
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            assert!(result.range.is_some());
            let range = result.range.unwrap();
            assert_eq!(range.min, Some(0.5));
            assert_eq!(range.max, Some(99.9));
        }

        #[test]
        fn test_range_with_message() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(range(min = 1, max = 10, message = "Must be between 1 and 10"))]
                pub rating: i32
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            let range = result.range.unwrap();
            assert_eq!(range.min, Some(1.0));
            assert_eq!(range.max, Some(10.0));
            assert_eq!(range.message, Some("Must be between 1 and 10".to_string()));
        }
    }

    mod message_parsing {
        use super::*;

        #[test]
        fn test_message_with_double_quotes() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(length(min = 5, message = "Too short"))]
                pub field: String
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            let length = result.length.unwrap();
            assert_eq!(length.message, Some("Too short".to_string()));
        }

        #[test]
        fn test_message_with_single_quotes() {
            // Testing the parse_message_from_content method directly
            let parser = parser();
            let content = "min = 5, message = 'Too short'";
            let result = parser.parse_message_from_content(content);
            assert_eq!(result, Some("Too short".to_string()));
        }

        #[test]
        fn test_message_with_special_characters() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(length(min = 5, message = "Length must be >= 5"))]
                pub field: String
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            let length = result.length.unwrap();
            assert_eq!(length.message, Some("Length must be >= 5".to_string()));
        }

        #[test]
        fn test_message_with_escaped_quotes() {
            let parser = parser();
            let content = r#"min = 5, message = "Must be \"valid\"""#;
            let result = parser.parse_message_from_content(content);
            assert_eq!(result, Some(r#"Must be "valid""#.to_string()));
        }

        #[test]
        fn test_message_with_newline() {
            let parser = parser();
            let content = r#"min = 5, message = "Line 1\nLine 2""#;
            let result = parser.parse_message_from_content(content);
            assert_eq!(result, Some("Line 1\nLine 2".to_string()));
        }

        #[test]
        fn test_no_message() {
            let parser = parser();
            let content = "min = 5, max = 100";
            let result = parser.parse_message_from_content(content);
            assert_eq!(result, None);
        }
    }

    mod combined_validators {
        use super::*;

        #[test]
        fn test_email_with_length() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(email, length(max = 255))]
                pub email: String
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            assert!(result.email);
            assert!(result.length.is_some());
            let length = result.length.unwrap();
            assert_eq!(length.max, Some(255));
        }

        #[test]
        fn test_length_and_range() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(length(min = 1, max = 50), range(min = 0, max = 100))]
                pub field: String
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            assert!(result.length.is_some());
            assert!(result.range.is_some());
        }

        #[test]
        fn test_all_validators_combined() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(email, url, length(min = 1), range(min = 0))]
                pub field: String
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            assert!(result.email);
            assert!(result.url);
            assert!(result.length.is_some());
            assert!(result.range.is_some());
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn test_empty_validate_attribute() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate]
                pub field: String
            };
            // Should still return Some (validator found) even if empty
            let result = parser.parse_validator_attributes(&field.attrs);
            assert!(result.is_some());
        }

        #[test]
        fn test_malformed_length_no_parentheses() {
            let parser = parser();
            let tokens = "length min = 5";
            let result = parser.parse_length_from_tokens(tokens);
            // Should still create a length constraint (found "length")
            assert!(result.is_some());
        }

        #[test]
        fn test_length_with_only_parameter_names() {
            let parser = parser();
            let tokens = "length(min, max)";
            let result = parser.parse_length_from_tokens(tokens);
            assert!(result.is_some());
            let length = result.unwrap();
            // No values parsed
            assert_eq!(length.min, None);
            assert_eq!(length.max, None);
        }

        #[test]
        fn test_range_without_values() {
            let parser = parser();
            let tokens = "range()";
            let result = parser.parse_range_from_tokens(tokens);
            assert!(result.is_some());
            let range = result.unwrap();
            assert_eq!(range.min, None);
            assert_eq!(range.max, None);
        }

        #[test]
        fn test_multiple_validator_attributes() {
            let parser = parser();
            let field: Field = parse_quote! {
                #[validate(email)]
                #[validate(length(min = 5))]
                pub field: String
            };
            let result = parser.parse_validator_attributes(&field.attrs).unwrap();
            // Should capture both validators
            assert!(result.email);
            assert!(result.length.is_some());
        }
    }

    mod parse_helpers {
        use super::*;

        #[test]
        fn test_parse_length_from_tokens() {
            let parser = parser();
            let tokens = "length(min = 5, max = 100)";
            let result = parser.parse_length_from_tokens(tokens).unwrap();
            assert_eq!(result.min, Some(5));
            assert_eq!(result.max, Some(100));
        }

        #[test]
        fn test_parse_range_from_tokens() {
            let parser = parser();
            let tokens = "range(min = 0.5, max = 99.9)";
            let result = parser.parse_range_from_tokens(tokens).unwrap();
            assert_eq!(result.min, Some(0.5));
            assert_eq!(result.max, Some(99.9));
        }

        #[test]
        fn test_parse_message_from_content_double_quotes() {
            let parser = parser();
            let content = r#"min = 5, message = "Test message""#;
            let result = parser.parse_message_from_content(content).unwrap();
            assert_eq!(result, "Test message");
        }

        #[test]
        fn test_parse_message_from_content_single_quotes() {
            let parser = parser();
            let content = "min = 5, message = 'Test message'";
            let result = parser.parse_message_from_content(content).unwrap();
            assert_eq!(result, "Test message");
        }

        #[test]
        fn test_no_length_in_tokens() {
            let parser = parser();
            let tokens = "email, url";
            let result = parser.parse_length_from_tokens(tokens);
            assert!(result.is_none());
        }

        #[test]
        fn test_no_range_in_tokens() {
            let parser = parser();
            let tokens = "email, url";
            let result = parser.parse_range_from_tokens(tokens);
            assert!(result.is_none());
        }
    }
}
