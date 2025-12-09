use crate::analysis::serde_parser::{apply_naming_convention, SerdeParser};
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
                .filter_map(|field| {
                    self.parse_field(
                        field,
                        type_resolver,
                        struct_serde_attrs.rename_all.as_deref(),
                    )
                })
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
        })
    }

    /// Parse a Rust enum into StructInfo
    pub fn parse_enum(
        &self,
        item_enum: &ItemEnum,
        file_path: &Path,
        type_resolver: &mut TypeResolver,
    ) -> Option<StructInfo> {
        // Parse enum-level serde attributes
        let enum_serde_attrs = self.serde_parser.parse_struct_serde_attrs(&item_enum.attrs);

        let fields = item_enum
            .variants
            .iter()
            .map(|variant| {
                let variant_name = variant.ident.to_string();

                // Parse variant-level serde attributes
                let variant_serde_attrs = self.serde_parser.parse_field_serde_attrs(&variant.attrs);

                // Calculate serialized name for the variant
                let serialized_name = if let Some(rename) = variant_serde_attrs.rename {
                    // Explicit variant-level rename takes precedence
                    Some(rename)
                } else {
                    enum_serde_attrs
                        .rename_all
                        .as_deref()
                        .map(|convention| apply_naming_convention(&variant_name, convention))
                };

                match &variant.fields {
                    syn::Fields::Unit => {
                        // Unit variant: Variant
                        FieldInfo {
                            name: variant_name,
                            rust_type: "enum_variant".to_string(),
                            typescript_type: format!("\"{}\"", variant.ident),
                            is_optional: false,
                            is_public: true,
                            validator_attributes: None,
                            serialized_name,
                            type_structure: crate::models::TypeStructure::Primitive(
                                "string".to_string(),
                            ),
                        }
                    }
                    syn::Fields::Unnamed(fields_unnamed) => {
                        // Tuple variant: Variant(T, U)
                        let types: Vec<String> = fields_unnamed
                            .unnamed
                            .iter()
                            .map(|field| {
                                type_resolver
                                    .map_rust_type_to_typescript(&Self::type_to_string(&field.ty))
                            })
                            .collect();
                        FieldInfo {
                            name: variant_name,
                            rust_type: "enum_variant_tuple".to_string(),
                            typescript_type: format!(
                                "{{ type: \"{}\", data: [{}] }}",
                                variant.ident,
                                types.join(", ")
                            ),
                            is_optional: false,
                            is_public: true,
                            validator_attributes: None,
                            serialized_name,
                            // For enum variants, type structure is not used by generators
                            type_structure: crate::models::TypeStructure::Custom(
                                "enum_variant".to_string(),
                            ),
                        }
                    }
                    syn::Fields::Named(fields_named) => {
                        // Struct variant: Variant { field: T }
                        let struct_fields: Vec<String> = fields_named
                            .named
                            .iter()
                            .filter_map(|field| {
                                field.ident.as_ref().map(|field_name| {
                                    let field_type = type_resolver.map_rust_type_to_typescript(
                                        &Self::type_to_string(&field.ty),
                                    );
                                    format!("{}: {}", field_name, field_type)
                                })
                            })
                            .collect();
                        FieldInfo {
                            name: variant_name,
                            rust_type: "enum_variant_struct".to_string(),
                            typescript_type: format!(
                                "{{ type: \"{}\", data: {{ {} }} }}",
                                variant.ident,
                                struct_fields.join(", ")
                            ),
                            is_optional: false,
                            is_public: true,
                            validator_attributes: None,
                            serialized_name,
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
        })
    }

    /// Parse a struct field into FieldInfo
    fn parse_field(
        &self,
        field: &syn::Field,
        type_resolver: &mut TypeResolver,
        struct_rename_all: Option<&str>,
    ) -> Option<FieldInfo> {
        let name = field.ident.as_ref()?.to_string();

        // Parse field-level serde attributes
        let field_serde_attrs = self.serde_parser.parse_field_serde_attrs(&field.attrs);

        // Skip fields with #[serde(skip)]
        if field_serde_attrs.skip {
            return None;
        }

        // Calculate the serialized name based on serde attributes
        let serialized_name = if let Some(rename) = field_serde_attrs.rename {
            // Explicit field-level rename takes precedence
            Some(rename)
        } else {
            struct_rename_all.map(|convention| apply_naming_convention(&name, convention))
        };

        let is_public = matches!(field.vis, Visibility::Public(_));
        let is_optional = self.is_optional_type(&field.ty);
        let rust_type = Self::type_to_string(&field.ty);
        let typescript_type = type_resolver.map_rust_type_to_typescript(&rust_type);
        let type_structure = type_resolver.parse_type_structure(&rust_type);
        let validator_attributes = self
            .validator_parser
            .parse_validator_attributes(&field.attrs);

        Some(FieldInfo {
            name,
            rust_type,
            typescript_type,
            is_optional,
            is_public,
            validator_attributes,
            serialized_name,
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
