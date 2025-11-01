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
                        let mut chars = rest.chars().enumerate();
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
