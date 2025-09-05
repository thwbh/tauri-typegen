use crate::analysis::validator_parser::ValidatorParser;
use crate::analysis::type_resolver::TypeResolver;
use crate::models::{StructInfo, FieldInfo};
use quote::ToTokens;
use std::path::PathBuf;
use syn::{Attribute, ItemEnum, ItemStruct, Type, Visibility};

/// Parser for Rust structs and enums
#[derive(Debug)]
pub struct StructParser {
    validator_parser: ValidatorParser,
}

impl StructParser {
    pub fn new() -> Self {
        Self {
            validator_parser: ValidatorParser::new(),
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

                if tokens_str.contains("Serialize") || tokens_str.contains("Deserialize") {
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Parse a Rust struct into StructInfo
    pub fn parse_struct(&self, item_struct: &ItemStruct, file_path: &PathBuf, type_resolver: &mut TypeResolver) -> Option<StructInfo> {
        let mut fields = Vec::new();

        match &item_struct.fields {
            syn::Fields::Named(fields_named) => {
                for field in &fields_named.named {
                    if let Some(field_info) = self.parse_field(field, type_resolver) {
                        fields.push(field_info);
                    }
                }
            }
            syn::Fields::Unnamed(_) => {
                // Handle tuple structs if needed
                return None;
            }
            syn::Fields::Unit => {
                // Unit struct
            }
        }

        Some(StructInfo {
            name: item_struct.ident.to_string(),
            fields,
            file_path: file_path.to_string_lossy().to_string(),
            is_enum: false,
        })
    }

    /// Parse a Rust enum into StructInfo
    pub fn parse_enum(&self, item_enum: &ItemEnum, file_path: &PathBuf, type_resolver: &mut TypeResolver) -> Option<StructInfo> {
        let mut fields = Vec::new();

        for variant in &item_enum.variants {
            match &variant.fields {
                syn::Fields::Unit => {
                    // Unit variant: Variant
                    let field_info = FieldInfo {
                        name: variant.ident.to_string(),
                        rust_type: "enum_variant".to_string(),
                        typescript_type: format!("\"{}\"", variant.ident.to_string()),
                        is_optional: false,
                        is_public: true,
                        validator_attributes: None,
                    };
                    fields.push(field_info);
                },
                syn::Fields::Unnamed(fields_unnamed) => {
                    // Tuple variant: Variant(T, U)
                    let types: Vec<String> = fields_unnamed.unnamed.iter()
                        .map(|field| type_resolver.map_rust_type_to_typescript(&self.type_to_string(&field.ty)))
                        .collect();
                    let field_info = FieldInfo {
                        name: variant.ident.to_string(),
                        rust_type: "enum_variant_tuple".to_string(),
                        typescript_type: format!("{{ type: \"{}\", data: [{}] }}", variant.ident.to_string(), types.join(", ")),
                        is_optional: false,
                        is_public: true,
                        validator_attributes: None,
                    };
                    fields.push(field_info);
                },
                syn::Fields::Named(fields_named) => {
                    // Struct variant: Variant { field: T }
                    let mut struct_fields = Vec::new();
                    for field in &fields_named.named {
                        if let Some(field_name) = &field.ident {
                            let field_type = type_resolver.map_rust_type_to_typescript(&self.type_to_string(&field.ty));
                            struct_fields.push(format!("{}: {}", field_name, field_type));
                        }
                    }
                    let field_info = FieldInfo {
                        name: variant.ident.to_string(),
                        rust_type: "enum_variant_struct".to_string(),
                        typescript_type: format!("{{ type: \"{}\", data: {{ {} }} }}", variant.ident.to_string(), struct_fields.join(", ")),
                        is_optional: false,
                        is_public: true,
                        validator_attributes: None,
                    };
                    fields.push(field_info);
                }
            }
        }

        Some(StructInfo {
            name: item_enum.ident.to_string(),
            fields,
            file_path: file_path.to_string_lossy().to_string(),
            is_enum: true,
        })
    }

    /// Parse a struct field into FieldInfo
    fn parse_field(&self, field: &syn::Field, type_resolver: &mut TypeResolver) -> Option<FieldInfo> {
        let name = field.ident.as_ref()?.to_string();
        let is_public = matches!(field.vis, Visibility::Public(_));
        let is_optional = self.is_optional_type(&field.ty);
        let rust_type = self.type_to_string(&field.ty);
        let typescript_type = type_resolver.map_rust_type_to_typescript(&rust_type);
        let validator_attributes = self.validator_parser.parse_validator_attributes(&field.attrs);

        Some(FieldInfo {
            name,
            rust_type,
            typescript_type,
            is_optional,
            is_public,
            validator_attributes,
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
    fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::Path(type_path) => {
                let path = &type_path.path;
                let segments: Vec<String> = path.segments.iter().map(|segment| {
                    let ident = segment.ident.to_string();
                    match &segment.arguments {
                        syn::PathArguments::None => ident,
                        syn::PathArguments::AngleBracketed(args) => {
                            let generic_args: Vec<String> = args.args.iter().map(|arg| {
                                match arg {
                                    syn::GenericArgument::Type(t) => self.type_to_string(t),
                                    _ => "unknown".to_string(),
                                }
                            }).collect();
                            format!("{}<{}>", ident, generic_args.join(", "))
                        },
                        syn::PathArguments::Parenthesized(_) => ident, // Function types, not common in structs
                    }
                }).collect();
                segments.join("::")
            },
            Type::Reference(type_ref) => {
                format!("&{}", self.type_to_string(&type_ref.elem))
            },
            Type::Tuple(type_tuple) => {
                let elements: Vec<String> = type_tuple.elems.iter()
                    .map(|elem| self.type_to_string(elem))
                    .collect();
                format!("({})", elements.join(", "))
            },
            Type::Array(type_array) => {
                format!("[{}; _]", self.type_to_string(&type_array.elem))
            },
            Type::Slice(type_slice) => {
                format!("[{}]", self.type_to_string(&type_slice.elem))
            },
            _ => "unknown".to_string(),
        }
    }
}

impl Default for StructParser {
    fn default() -> Self {
        Self::new()
    }
}