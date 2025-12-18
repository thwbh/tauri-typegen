use crate::analysis::serde_parser::SerdeParser;
use crate::analysis::type_resolver::TypeResolver;
use crate::analysis::validator_parser::ValidatorParser;
use crate::models::{FieldInfo, StructInfo};
use quote::ToTokens;
use std::path::Path;
use syn::{Attribute, ItemEnum, ItemStruct, Type, Visibility};

/// Parser for Rust structs and enums
#[derive(Debug)]
pub struct StructParser {
    validator_parser: ValidatorParser,
    serde_parser: SerdeParser,
}

impl StructParser {
    pub fn new() -> Self {
        Self {
            validator_parser: ValidatorParser::new(),
            serde_parser: SerdeParser::new(),
        }
    }

    /// Check if a struct should be included in type generation
    pub fn should_include_struct(&self, item_struct: &ItemStruct) -> bool {
        // Check if struct has Serialize or Deserialize derive
        for attr in &item_struct.attrs {
            if self.should_include(attr) {
                return true;
            }
        }
        false
    }

    /// Check if an enum should be included in type generation
    pub fn should_include_enum(&self, item_enum: &ItemEnum) -> bool {
        // Check if enum has Serialize or Deserialize derive
        for attr in &item_enum.attrs {
            if self.should_include(attr) {
                return true;
            }
        }
        false
    }

    /// Check if an attribute indicates the type should be included
    fn should_include(&self, attr: &Attribute) -> bool {
        if let Ok(meta_list) = attr.meta.require_list() {
            if meta_list.path.is_ident("derive") {
                let tokens_str = meta_list.to_token_stream().to_string();

                tokens_str.contains("Serialize") || tokens_str.contains("Deserialize")
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Parse a Rust struct into StructInfo
    pub fn parse_struct(
        &self,
        item_struct: &ItemStruct,
        file_path: &Path,
        type_resolver: &mut TypeResolver,
    ) -> Option<StructInfo> {
        // Parse struct-level serde attributes
        let struct_serde_attrs = self
            .serde_parser
            .parse_struct_serde_attrs(&item_struct.attrs);

        let fields = match &item_struct.fields {
            syn::Fields::Named(fields_named) => fields_named
                .named
                .iter()
                .filter_map(|field| self.parse_field(field, type_resolver))
                .collect(),
            syn::Fields::Unnamed(_) => {
                // Handle tuple structs if needed
                return None;
            }
            syn::Fields::Unit => {
                // Unit struct
                Vec::new()
            }
        };

        Some(StructInfo {
            name: item_struct.ident.to_string(),
            fields,
            file_path: file_path.to_string_lossy().to_string(),
            is_enum: false,
            serde_rename_all: struct_serde_attrs.rename_all,
        })
    }

    /// Parse a Rust enum into StructInfo
    pub fn parse_enum(&self, item_enum: &ItemEnum, file_path: &Path) -> Option<StructInfo> {
        // Parse enum-level serde attributes
        let enum_serde_attrs = self.serde_parser.parse_struct_serde_attrs(&item_enum.attrs);

        let fields = item_enum
            .variants
            .iter()
            .map(|variant| {
                let variant_name = variant.ident.to_string();

                // Parse variant-level serde attributes
                let variant_serde_attrs = self.serde_parser.parse_field_serde_attrs(&variant.attrs);

                match &variant.fields {
                    syn::Fields::Unit => {
                        // Unit variant: Variant
                        FieldInfo {
                            name: variant_name,
                            rust_type: "enum_variant".to_string(),
                            is_optional: false,
                            is_public: true,
                            validator_attributes: None,
                            serde_rename: variant_serde_attrs.rename,
                            type_structure: crate::models::TypeStructure::Primitive(
                                "string".to_string(),
                            ),
                        }
                    }
                    syn::Fields::Unnamed(_fields_unnamed) => {
                        // Tuple variant: Variant(T, U)
                        // Note: Complex enum variants are not fully supported yet
                        FieldInfo {
                            name: variant_name,
                            rust_type: "enum_variant_tuple".to_string(),
                            is_optional: false,
                            is_public: true,
                            validator_attributes: None,
                            serde_rename: variant_serde_attrs.rename,
                            // For enum variants, type structure is not used by generators
                            type_structure: crate::models::TypeStructure::Custom(
                                "enum_variant".to_string(),
                            ),
                        }
                    }
                    syn::Fields::Named(_fields_named) => {
                        // Struct variant: Variant { field: T }
                        // Note: Complex enum variants are not fully supported yet
                        FieldInfo {
                            name: variant_name,
                            rust_type: "enum_variant_struct".to_string(),
                            is_optional: false,
                            is_public: true,
                            validator_attributes: None,
                            serde_rename: variant_serde_attrs.rename,
                            // For enum variants, type structure is not used by generators
                            type_structure: crate::models::TypeStructure::Custom(
                                "enum_variant".to_string(),
                            ),
                        }
                    }
                }
            })
            .collect();

        Some(StructInfo {
            name: item_enum.ident.to_string(),
            fields,
            file_path: file_path.to_string_lossy().to_string(),
            is_enum: true,
            serde_rename_all: enum_serde_attrs.rename_all,
        })
    }

    /// Parse a struct field into FieldInfo
    fn parse_field(
        &self,
        field: &syn::Field,
        type_resolver: &mut TypeResolver,
    ) -> Option<FieldInfo> {
        let name = field.ident.as_ref()?.to_string();

        // Parse field-level serde attributes
        let field_serde_attrs = self.serde_parser.parse_field_serde_attrs(&field.attrs);

        // Skip fields with #[serde(skip)]
        if field_serde_attrs.skip {
            return None;
        }

        let is_public = matches!(field.vis, Visibility::Public(_));
        let is_optional = self.is_optional_type(&field.ty);
        let rust_type = Self::type_to_string(&field.ty);
        let type_structure = type_resolver.parse_type_structure(&rust_type);
        let validator_attributes = self
            .validator_parser
            .parse_validator_attributes(&field.attrs);

        Some(FieldInfo {
            name,
            rust_type,
            is_optional,
            is_public,
            validator_attributes,
            serde_rename: field_serde_attrs.rename,
            type_structure,
        })
    }

    /// Check if a type is Option<T>
    fn is_optional_type(&self, ty: &Type) -> bool {
        if let Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                segment.ident == "Option"
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Convert a Type to its string representation
    fn type_to_string(ty: &Type) -> String {
        match ty {
            Type::Path(type_path) => {
                let path = &type_path.path;
                let segments: Vec<String> = path
                    .segments
                    .iter()
                    .map(|segment| {
                        let ident = segment.ident.to_string();
                        match &segment.arguments {
                            syn::PathArguments::None => ident,
                            syn::PathArguments::AngleBracketed(args) => {
                                let generic_args: Vec<String> = args
                                    .args
                                    .iter()
                                    .map(|arg| match arg {
                                        syn::GenericArgument::Type(t) => Self::type_to_string(t),
                                        _ => "unknown".to_string(),
                                    })
                                    .collect();
                                format!("{}<{}>", ident, generic_args.join(", "))
                            }
                            syn::PathArguments::Parenthesized(_) => ident, // Function types, not common in structs
                        }
                    })
                    .collect();
                segments.join("::")
            }
            Type::Reference(type_ref) => {
                format!("&{}", Self::type_to_string(&type_ref.elem))
            }
            Type::Tuple(type_tuple) => {
                let elements: Vec<String> =
                    type_tuple.elems.iter().map(Self::type_to_string).collect();
                format!("({})", elements.join(", "))
            }
            Type::Array(type_array) => {
                format!("[{}; _]", Self::type_to_string(&type_array.elem))
            }
            Type::Slice(type_slice) => {
                format!("[{}]", Self::type_to_string(&type_slice.elem))
            }
            _ => "unknown".to_string(),
        }
    }
}

impl Default for StructParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_rename_rule::RenameRule;
    use syn::parse_quote;

    // Helper to create a test struct parser
    fn parser() -> StructParser {
        StructParser::new()
    }

    // Helper to create a test type resolver
    fn type_resolver() -> TypeResolver {
        TypeResolver::new()
    }

    mod derive_attribute_detection {
        use super::*;

        #[test]
        fn test_should_include_struct_with_serialize() {
            let parser = parser();
            let item: ItemStruct = parse_quote! {
                #[derive(Serialize)]
                pub struct User {
                    name: String,
                }
            };
            assert!(parser.should_include_struct(&item));
        }

        #[test]
        fn test_should_include_struct_with_deserialize() {
            let parser = parser();
            let item: ItemStruct = parse_quote! {
                #[derive(Deserialize)]
                pub struct User {
                    name: String,
                }
            };
            assert!(parser.should_include_struct(&item));
        }

        #[test]
        fn test_should_include_struct_with_both() {
            let parser = parser();
            let item: ItemStruct = parse_quote! {
                #[derive(Serialize, Deserialize)]
                pub struct User {
                    name: String,
                }
            };
            assert!(parser.should_include_struct(&item));
        }

        #[test]
        fn test_should_not_include_struct_without_serde() {
            let parser = parser();
            let item: ItemStruct = parse_quote! {
                #[derive(Debug, Clone)]
                pub struct User {
                    name: String,
                }
            };
            assert!(!parser.should_include_struct(&item));
        }

        #[test]
        fn test_should_include_enum_with_serialize() {
            let parser = parser();
            let item: ItemEnum = parse_quote! {
                #[derive(Serialize)]
                pub enum Status {
                    Active,
                    Inactive,
                }
            };
            assert!(parser.should_include_enum(&item));
        }

        #[test]
        fn test_should_not_include_enum_without_serde() {
            let parser = parser();
            let item: ItemEnum = parse_quote! {
                #[derive(Debug, Clone)]
                pub enum Status {
                    Active,
                    Inactive,
                }
            };
            assert!(!parser.should_include_enum(&item));
        }
    }

    mod struct_parsing {
        use super::*;

        #[test]
        fn test_parse_simple_struct() {
            let parser = parser();
            let mut resolver = type_resolver();
            let item: ItemStruct = parse_quote! {
                #[derive(Serialize)]
                pub struct User {
                    pub name: String,
                    pub age: i32,
                }
            };
            let path = Path::new("test.rs");
            let result = parser.parse_struct(&item, path, &mut resolver);

            assert!(result.is_some());
            let struct_info = result.unwrap();
            assert_eq!(struct_info.name, "User");
            assert_eq!(struct_info.fields.len(), 2);
            assert!(!struct_info.is_enum);
            assert_eq!(struct_info.file_path, "test.rs");
        }

        #[test]
        fn test_parse_struct_with_optional_fields() {
            let parser = parser();
            let mut resolver = type_resolver();
            let item: ItemStruct = parse_quote! {
                #[derive(Serialize)]
                pub struct User {
                    pub name: String,
                    pub email: Option<String>,
                }
            };
            let path = Path::new("test.rs");
            let result = parser.parse_struct(&item, path, &mut resolver).unwrap();

            assert_eq!(result.fields.len(), 2);
            assert!(!result.fields[0].is_optional);
            assert!(result.fields[1].is_optional);
        }

        #[test]
        fn test_parse_struct_with_serde_skip() {
            let parser = parser();
            let mut resolver = type_resolver();
            let item: ItemStruct = parse_quote! {
                #[derive(Serialize)]
                pub struct User {
                    pub name: String,
                    #[serde(skip)]
                    pub password: String,
                }
            };
            let path = Path::new("test.rs");
            let result = parser.parse_struct(&item, path, &mut resolver).unwrap();

            // Password field should be skipped
            assert_eq!(result.fields.len(), 1);
            assert_eq!(result.fields[0].name, "name");
        }

        #[test]
        fn test_parse_struct_with_serde_rename() {
            let parser = parser();
            let mut resolver = type_resolver();
            let item: ItemStruct = parse_quote! {
                #[derive(Serialize)]
                pub struct User {
                    #[serde(rename = "userName")]
                    pub user_name: String,
                }
            };
            let path = Path::new("test.rs");
            let result = parser.parse_struct(&item, path, &mut resolver).unwrap();

            assert_eq!(result.fields[0].serde_rename, Some("userName".to_string()));
        }

        #[test]
        fn test_parse_struct_with_rename_all() {
            let parser = parser();
            let mut resolver = type_resolver();
            let item: ItemStruct = parse_quote! {
                #[derive(Serialize)]
                #[serde(rename_all = "camelCase")]
                pub struct User {
                    pub user_name: String,
                }
            };
            let path = Path::new("test.rs");
            let result = parser.parse_struct(&item, path, &mut resolver).unwrap();

            assert_eq!(result.serde_rename_all, Some(RenameRule::CamelCase));
        }

        #[test]
        fn test_parse_unit_struct() {
            let parser = parser();
            let mut resolver = type_resolver();
            let item: ItemStruct = parse_quote! {
                #[derive(Serialize)]
                pub struct Unit;
            };
            let path = Path::new("test.rs");
            let result = parser.parse_struct(&item, path, &mut resolver).unwrap();

            assert_eq!(result.name, "Unit");
            assert_eq!(result.fields.len(), 0);
        }

        #[test]
        fn test_parse_tuple_struct_returns_none() {
            let parser = parser();
            let mut resolver = type_resolver();
            let item: ItemStruct = parse_quote! {
                #[derive(Serialize)]
                pub struct Point(i32, i32);
            };
            let path = Path::new("test.rs");
            let result = parser.parse_struct(&item, path, &mut resolver);

            // Tuple structs are not supported
            assert!(result.is_none());
        }

        #[test]
        fn test_parse_struct_with_private_fields() {
            let parser = parser();
            let mut resolver = type_resolver();
            let item: ItemStruct = parse_quote! {
                #[derive(Serialize)]
                pub struct User {
                    pub name: String,
                    age: i32,
                }
            };
            let path = Path::new("test.rs");
            let result = parser.parse_struct(&item, path, &mut resolver).unwrap();

            assert_eq!(result.fields.len(), 2);
            assert!(result.fields[0].is_public);
            assert!(!result.fields[1].is_public);
        }
    }

    mod enum_parsing {
        use super::*;

        #[test]
        fn test_parse_simple_enum() {
            let parser = parser();
            let item: ItemEnum = parse_quote! {
                #[derive(Serialize)]
                pub enum Status {
                    Active,
                    Inactive,
                }
            };
            let path = Path::new("test.rs");
            let result = parser.parse_enum(&item, path);

            assert!(result.is_some());
            let enum_info = result.unwrap();
            assert_eq!(enum_info.name, "Status");
            assert_eq!(enum_info.fields.len(), 2);
            assert!(enum_info.is_enum);
        }

        #[test]
        fn test_parse_enum_unit_variants() {
            let parser = parser();
            let item: ItemEnum = parse_quote! {
                #[derive(Serialize)]
                pub enum Status {
                    Active,
                    Inactive,
                    Pending,
                }
            };
            let path = Path::new("test.rs");
            let result = parser.parse_enum(&item, path).unwrap();

            assert_eq!(result.fields.len(), 3);
            assert_eq!(result.fields[0].name, "Active");
            assert_eq!(result.fields[0].rust_type, "enum_variant");
            assert_eq!(result.fields[1].name, "Inactive");
            assert_eq!(result.fields[2].name, "Pending");
        }

        #[test]
        fn test_parse_enum_tuple_variant() {
            let parser = parser();
            let item: ItemEnum = parse_quote! {
                #[derive(Serialize)]
                pub enum Message {
                    Text(String),
                    Number(i32),
                }
            };
            let path = Path::new("test.rs");
            let result = parser.parse_enum(&item, path).unwrap();

            assert_eq!(result.fields.len(), 2);
            assert_eq!(result.fields[0].rust_type, "enum_variant_tuple");
            assert_eq!(result.fields[1].rust_type, "enum_variant_tuple");
        }

        #[test]
        fn test_parse_enum_struct_variant() {
            let parser = parser();
            let item: ItemEnum = parse_quote! {
                #[derive(Serialize)]
                pub enum Message {
                    User { id: i32, name: String },
                }
            };
            let path = Path::new("test.rs");
            let result = parser.parse_enum(&item, path).unwrap();

            assert_eq!(result.fields.len(), 1);
            assert_eq!(result.fields[0].rust_type, "enum_variant_struct");
        }

        #[test]
        fn test_parse_enum_with_serde_rename_variant() {
            let parser = parser();
            let item: ItemEnum = parse_quote! {
                #[derive(Serialize)]
                pub enum Status {
                    #[serde(rename = "active")]
                    Active,
                    #[serde(rename = "inactive")]
                    Inactive,
                }
            };
            let path = Path::new("test.rs");
            let result = parser.parse_enum(&item, path).unwrap();

            assert_eq!(result.fields[0].serde_rename, Some("active".to_string()));
            assert_eq!(result.fields[1].serde_rename, Some("inactive".to_string()));
        }

        #[test]
        fn test_parse_enum_with_rename_all() {
            let parser = parser();
            let item: ItemEnum = parse_quote! {
                #[derive(Serialize)]
                #[serde(rename_all = "snake_case")]
                pub enum Status {
                    ActiveUser,
                    InactiveUser,
                }
            };
            let path = Path::new("test.rs");
            let result = parser.parse_enum(&item, path).unwrap();

            assert_eq!(result.serde_rename_all, Some(RenameRule::SnakeCase));
        }
    }

    mod type_detection {
        use super::*;

        #[test]
        fn test_is_optional_type_with_option() {
            let parser = parser();
            let ty: Type = parse_quote!(Option<String>);
            assert!(parser.is_optional_type(&ty));
        }

        #[test]
        fn test_is_optional_type_with_plain_type() {
            let parser = parser();
            let ty: Type = parse_quote!(String);
            assert!(!parser.is_optional_type(&ty));
        }

        #[test]
        fn test_is_optional_type_with_nested_option() {
            let parser = parser();
            let ty: Type = parse_quote!(Option<Option<String>>);
            assert!(parser.is_optional_type(&ty));
        }

        #[test]
        fn test_is_optional_type_with_vec() {
            let parser = parser();
            let ty: Type = parse_quote!(Vec<String>);
            assert!(!parser.is_optional_type(&ty));
        }
    }

    mod type_to_string_conversion {
        use super::*;

        #[test]
        fn test_simple_type() {
            let ty: Type = parse_quote!(String);
            assert_eq!(StructParser::type_to_string(&ty), "String");
        }

        #[test]
        fn test_generic_type() {
            let ty: Type = parse_quote!(Vec<String>);
            assert_eq!(StructParser::type_to_string(&ty), "Vec<String>");
        }

        #[test]
        fn test_nested_generic_type() {
            let ty: Type = parse_quote!(Vec<Option<String>>);
            assert_eq!(StructParser::type_to_string(&ty), "Vec<Option<String>>");
        }

        #[test]
        fn test_multiple_generic_args() {
            let ty: Type = parse_quote!(HashMap<String, i32>);
            assert_eq!(StructParser::type_to_string(&ty), "HashMap<String, i32>");
        }

        #[test]
        fn test_reference_type() {
            let ty: Type = parse_quote!(&String);
            assert_eq!(StructParser::type_to_string(&ty), "&String");
        }

        #[test]
        fn test_tuple_type() {
            let ty: Type = parse_quote!((String, i32));
            assert_eq!(StructParser::type_to_string(&ty), "(String, i32)");
        }

        #[test]
        fn test_tuple_three_elements() {
            let ty: Type = parse_quote!((String, i32, bool));
            assert_eq!(StructParser::type_to_string(&ty), "(String, i32, bool)");
        }

        #[test]
        fn test_array_type() {
            let ty: Type = parse_quote!([i32; 5]);
            assert_eq!(StructParser::type_to_string(&ty), "[i32; _]");
        }

        #[test]
        fn test_slice_type() {
            let ty: Type = parse_quote!([String]);
            assert_eq!(StructParser::type_to_string(&ty), "[String]");
        }

        #[test]
        fn test_path_with_segments() {
            let ty: Type = parse_quote!(std::collections::HashMap<String, i32>);
            assert_eq!(
                StructParser::type_to_string(&ty),
                "std::collections::HashMap<String, i32>"
            );
        }

        #[test]
        fn test_complex_nested_type() {
            let ty: Type = parse_quote!(HashMap<String, Vec<Option<User>>>);
            assert_eq!(
                StructParser::type_to_string(&ty),
                "HashMap<String, Vec<Option<User>>>"
            );
        }
    }

    mod field_parsing {
        use super::*;

        #[test]
        fn test_parse_field_public() {
            let parser = parser();
            let mut resolver = type_resolver();
            let item: ItemStruct = parse_quote! {
                struct Test {
                    pub field: String,
                }
            };
            if let syn::Fields::Named(fields) = &item.fields {
                let field = fields.named.first().unwrap();
                let result = parser.parse_field(field, &mut resolver).unwrap();
                assert!(result.is_public);
            }
        }

        #[test]
        fn test_parse_field_private() {
            let parser = parser();
            let mut resolver = type_resolver();
            let item: ItemStruct = parse_quote! {
                struct Test {
                    field: String,
                }
            };
            if let syn::Fields::Named(fields) = &item.fields {
                let field = fields.named.first().unwrap();
                let result = parser.parse_field(field, &mut resolver).unwrap();
                assert!(!result.is_public);
            }
        }

        #[test]
        fn test_parse_field_with_serde_skip() {
            let parser = parser();
            let mut resolver = type_resolver();
            let item: ItemStruct = parse_quote! {
                struct Test {
                    #[serde(skip)]
                    field: String,
                }
            };
            if let syn::Fields::Named(fields) = &item.fields {
                let field = fields.named.first().unwrap();
                let result = parser.parse_field(field, &mut resolver);
                assert!(result.is_none());
            }
        }

        #[test]
        fn test_parse_field_with_validator() {
            let parser = parser();
            let mut resolver = type_resolver();
            let item: ItemStruct = parse_quote! {
                struct Test {
                    #[validate(length(min = 1, max = 100))]
                    field: String,
                }
            };
            if let syn::Fields::Named(fields) = &item.fields {
                let field = fields.named.first().unwrap();
                let result = parser.parse_field(field, &mut resolver).unwrap();
                assert!(result.validator_attributes.is_some());
            }
        }
    }

    mod integration {
        use super::*;

        #[test]
        fn test_parse_full_struct_with_all_features() {
            let parser = parser();
            let mut resolver = type_resolver();
            let item: ItemStruct = parse_quote! {
                #[derive(Serialize, Deserialize)]
                #[serde(rename_all = "camelCase")]
                pub struct User {
                    pub id: i32,
                    #[serde(rename = "userName")]
                    pub user_name: String,
                    pub email: Option<String>,
                    #[serde(skip)]
                    password: String,
                    #[validate(length(min = 1, max = 100))]
                    pub bio: String,
                }
            };
            let path = Path::new("models.rs");
            let result = parser.parse_struct(&item, path, &mut resolver).unwrap();

            assert_eq!(result.name, "User");
            assert_eq!(result.fields.len(), 4); // password skipped
            assert_eq!(result.serde_rename_all, Some(RenameRule::CamelCase));
            assert_eq!(result.file_path, "models.rs");

            // Check specific fields
            assert_eq!(result.fields[0].name, "id");
            assert_eq!(result.fields[1].name, "user_name");
            assert_eq!(result.fields[1].serde_rename, Some("userName".to_string()));
            assert!(result.fields[2].is_optional);
            assert!(result.fields[3].validator_attributes.is_some());
        }

        #[test]
        fn test_parse_full_enum_with_all_features() {
            let parser = parser();
            let item: ItemEnum = parse_quote! {
                #[derive(Serialize, Deserialize)]
                #[serde(rename_all = "snake_case")]
                pub enum Message {
                    #[serde(rename = "simple")]
                    Simple,
                    Text(String),
                    User { id: i32 },
                }
            };
            let path = Path::new("models.rs");
            let result = parser.parse_enum(&item, path).unwrap();

            assert_eq!(result.name, "Message");
            assert_eq!(result.fields.len(), 3);
            assert_eq!(result.serde_rename_all, Some(RenameRule::SnakeCase));
            assert!(result.is_enum);

            // Check variant types
            assert_eq!(result.fields[0].rust_type, "enum_variant");
            assert_eq!(result.fields[0].serde_rename, Some("simple".to_string()));
            assert_eq!(result.fields[1].rust_type, "enum_variant_tuple");
            assert_eq!(result.fields[2].rust_type, "enum_variant_struct");
        }
    }
}
