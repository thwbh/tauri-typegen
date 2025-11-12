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

/// Apply serde naming convention transformations
pub fn apply_naming_convention(field_name: &str, convention: &str) -> String {
    match convention {
        "camelCase" => to_camel_case(field_name),
        "PascalCase" => to_pascal_case(field_name),
        "snake_case" => to_snake_case(field_name),
        "SCREAMING_SNAKE_CASE" => to_screaming_snake_case(field_name),
        "kebab-case" => to_kebab_case(field_name),
        "SCREAMING-KEBAB-CASE" => to_screaming_kebab_case(field_name),
        _ => field_name.to_string(), // Unknown convention, return as-is
    }
}

/// Convert to camelCase (first letter lowercase, subsequent words capitalized)
fn to_camel_case(s: &str) -> String {
    let words = split_into_words(s);
    if words.is_empty() {
        return String::new();
    }

    let mut result = words[0].to_lowercase();
    for word in &words[1..] {
        result.push_str(&capitalize_first(word));
    }
    result
}

/// Convert to PascalCase (all words capitalized, no separators)
fn to_pascal_case(s: &str) -> String {
    split_into_words(s)
        .iter()
        .map(|word| capitalize_first(word))
        .collect::<String>()
}

/// Convert to snake_case (lowercase with underscores)
fn to_snake_case(s: &str) -> String {
    split_into_words(s)
        .iter()
        .map(|word| word.to_lowercase())
        .collect::<Vec<_>>()
        .join("_")
}

/// Convert to SCREAMING_SNAKE_CASE (uppercase with underscores)
fn to_screaming_snake_case(s: &str) -> String {
    split_into_words(s)
        .iter()
        .map(|word| word.to_uppercase())
        .collect::<Vec<_>>()
        .join("_")
}

/// Convert to kebab-case (lowercase with hyphens)
fn to_kebab_case(s: &str) -> String {
    split_into_words(s)
        .iter()
        .map(|word| word.to_lowercase())
        .collect::<Vec<_>>()
        .join("-")
}

/// Convert to SCREAMING-KEBAB-CASE (uppercase with hyphens)
fn to_screaming_kebab_case(s: &str) -> String {
    split_into_words(s)
        .iter()
        .map(|word| word.to_uppercase())
        .collect::<Vec<_>>()
        .join("-")
}

/// Split a string into words, handling snake_case, camelCase, and PascalCase
fn split_into_words(s: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current_word = String::new();
    let mut prev_was_lowercase = false;

    for ch in s.chars() {
        if ch == '_' || ch == '-' {
            if !current_word.is_empty() {
                words.push(current_word.clone());
                current_word.clear();
            }
            prev_was_lowercase = false;
        } else if ch.is_uppercase() {
            // New word starts if previous was lowercase
            if prev_was_lowercase && !current_word.is_empty() {
                words.push(current_word.clone());
                current_word.clear();
            }
            current_word.push(ch);
            prev_was_lowercase = false;
        } else {
            current_word.push(ch);
            prev_was_lowercase = true;
        }
    }

    if !current_word.is_empty() {
        words.push(current_word);
    }

    words
}

/// Capitalize the first character of a string
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut result = first.to_uppercase().to_string();
            result.push_str(&chars.as_str().to_lowercase());
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camel_case() {
        assert_eq!(to_camel_case("user_id"), "userId");
        assert_eq!(to_camel_case("user_name"), "userName");
        assert_eq!(to_camel_case("is_active"), "isActive");
        assert_eq!(to_camel_case("UserName"), "userName");
    }

    #[test]
    fn test_pascal_case() {
        assert_eq!(to_pascal_case("user_id"), "UserId");
        assert_eq!(to_pascal_case("user_name"), "UserName");
        assert_eq!(to_pascal_case("userName"), "UserName");
    }

    #[test]
    fn test_snake_case() {
        assert_eq!(to_snake_case("userId"), "user_id");
        assert_eq!(to_snake_case("UserName"), "user_name");
        assert_eq!(to_snake_case("user_name"), "user_name");
    }

    #[test]
    fn test_screaming_snake_case() {
        assert_eq!(to_screaming_snake_case("user_id"), "USER_ID");
        assert_eq!(to_screaming_snake_case("userName"), "USER_NAME");
    }

    #[test]
    fn test_kebab_case() {
        assert_eq!(to_kebab_case("user_id"), "user-id");
        assert_eq!(to_kebab_case("userName"), "user-name");
    }

    #[test]
    fn test_screaming_kebab_case() {
        assert_eq!(to_screaming_kebab_case("user_id"), "USER-ID");
        assert_eq!(to_screaming_kebab_case("userName"), "USER-NAME");
    }

    #[test]
    fn test_apply_naming_convention() {
        assert_eq!(
            apply_naming_convention("user_name", "camelCase"),
            "userName"
        );
        assert_eq!(
            apply_naming_convention("user_name", "PascalCase"),
            "UserName"
        );
        assert_eq!(
            apply_naming_convention("userName", "snake_case"),
            "user_name"
        );
    }
}
