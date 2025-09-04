use crate::models::{CommandInfo, ParameterInfo, StructInfo, FieldInfo};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use quote::ToTokens;
use syn::{FnArg, ItemEnum, ItemFn, ItemStruct, PatType, ReturnType, Type, Visibility, Meta, MetaList, Attribute};
use syn::spanned::Spanned;
use walkdir::WalkDir;

pub struct CommandAnalyzer {
    type_mappings: HashMap<String, String>,
    discovered_structs: HashMap<String, StructInfo>,
}

impl CommandAnalyzer {
    pub fn new() -> Self {
        let mut type_mappings = HashMap::new();

        // Basic Rust to TypeScript mappings
        type_mappings.insert("String".to_string(), "string".to_string());
        type_mappings.insert("&str".to_string(), "string".to_string());
        type_mappings.insert("str".to_string(), "string".to_string());
        type_mappings.insert("i8".to_string(), "number".to_string());
        type_mappings.insert("i16".to_string(), "number".to_string());
        type_mappings.insert("i32".to_string(), "number".to_string());
        type_mappings.insert("i64".to_string(), "number".to_string());
        type_mappings.insert("i128".to_string(), "number".to_string());
        type_mappings.insert("isize".to_string(), "number".to_string());
        type_mappings.insert("u8".to_string(), "number".to_string());
        type_mappings.insert("u16".to_string(), "number".to_string());
        type_mappings.insert("u32".to_string(), "number".to_string());
        type_mappings.insert("u64".to_string(), "number".to_string());
        type_mappings.insert("u128".to_string(), "number".to_string());
        type_mappings.insert("usize".to_string(), "number".to_string());
        type_mappings.insert("f32".to_string(), "number".to_string());
        type_mappings.insert("f64".to_string(), "number".to_string());
        type_mappings.insert("bool".to_string(), "boolean".to_string());
        type_mappings.insert("()".to_string(), "void".to_string());

        Self {
            type_mappings,
            discovered_structs: HashMap::new(),
        }
    }

    pub fn analyze_project(&mut self, project_path: &str) -> Result<Vec<CommandInfo>, Box<dyn std::error::Error>> {
        let mut commands = Vec::new();
        let mut type_names_to_discover = HashSet::new();

        // First pass: Find all commands and collect type names
        for entry in WalkDir::new(project_path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                if let Some(extension) = entry.path().extension() {
                    if extension == "rs" {
                        println!("🔍 Scanning file: {}", entry.path().display());

                        // Analyze commands in this file
                        let file_commands = self.analyze_file(entry.path())?;

                        // Collect type names from command parameters and return types
                        for cmd in &file_commands {
                            for param in &cmd.parameters {
                                self.extract_type_names(&param.rust_type, &mut type_names_to_discover);
                            }
                            self.extract_type_names(&cmd.return_type, &mut type_names_to_discover);
                        }

                        commands.extend(file_commands);

                        // Also discover all structs in this file for completeness
                        if let Err(e) = self.discover_types_in_file(entry.path()) {
                            eprintln!("Warning: Failed to discover types in {}: {}", entry.path().display(), e);
                        }
                    }
                }
            }
        }

        println!("🔍 Type names to discover: {:?}", type_names_to_discover);

        // Second pass: Discover specific types that are referenced by commands
        for entry in WalkDir::new(project_path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                if let Some(extension) = entry.path().extension() {
                    if extension == "rs" {
                        if let Err(e) = self.discover_specific_types_in_file(entry.path(), &type_names_to_discover) {
                            eprintln!("Warning: Failed to discover specific types in {}: {}", entry.path().display(), e);
                        }
                    }
                }
            }
        }

        println!("🏗️  Discovered {} structs total", self.discovered_structs.len());
        for (name, info) in &self.discovered_structs {
            println!("  - {}: {} fields", name, info.fields.len());
        }

        Ok(commands)
    }

    fn extract_type_names(&self, rust_type: &str, type_names: &mut HashSet<String>) {
        // Handle various type patterns to extract custom type names
        let cleaned_type = rust_type
            .replace("Option<", "")
            .replace("Result<*>", "")
            .replace("Vec<", "")
            .replace("&", "")
            .replace(",", " ")
            .replace("String", "")
            .replace("()", "");

        for word in cleaned_type.split_whitespace() {
            let word = word.trim();
            if !word.is_empty()
                && !self.type_mappings.contains_key(word)
                && !word.starts_with(char::is_lowercase) // Skip built-in types
                && word.chars().next().map_or(false, char::is_alphabetic) {
                type_names.insert(word.to_string());
            }
        }
    }

    fn discover_specific_types_in_file(&mut self, file_path: &Path, target_types: &HashSet<String>) -> Result<(), Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path)?;
        let syntax = syn::parse_file(&content)?;

        for item in syntax.items {
            match item {
                syn::Item::Struct(item_struct) => {
                    let struct_name = item_struct.ident.to_string();
                    if target_types.contains(&struct_name) && self.should_include_struct(&item_struct) {
                        if let Some(struct_info) = self.parse_struct(&item_struct, file_path) {
                            self.discovered_structs.insert(struct_name, struct_info);
                        }
                    }
                }
                syn::Item::Enum(item_enum) => {
                    let enum_name = item_enum.ident.to_string();
                    if target_types.contains(&enum_name) && self.should_include_enum(&item_enum) {
                        if let Some(enum_info) = self.parse_enum(&item_enum, file_path) {
                            self.discovered_structs.insert(enum_name, enum_info);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn discover_types_in_file(&mut self, file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path)?;
        let syntax = syn::parse_file(&content)?;

        for item in syntax.items {
            match item {
                syn::Item::Struct(item_struct) => {
                    if self.should_include_struct(&item_struct) {
                        if let Some(struct_info) = self.parse_struct(&item_struct, file_path) {
                            let struct_name = item_struct.ident.to_string();
                            self.discovered_structs.insert(struct_name, struct_info);
                        }
                    }
                }
                syn::Item::Enum(item_enum) => {
                    if self.should_include_enum(&item_enum) {
                        if let Some(enum_info) = self.parse_enum(&item_enum, file_path) {
                            let enum_name = item_enum.ident.to_string();
                            self.discovered_structs.insert(enum_name, enum_info);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn should_include_struct(&self, item_struct: &ItemStruct) -> bool {
        // Check if struct has Serialize or Deserialize derive
        for attr in &item_struct.attrs {
            if self.should_include(attr) {
                return true;
            }
        }

        false
    }

    fn should_include_enum(&self, item_enum: &ItemEnum) -> bool {
        // Check if enum has Serialize or Deserialize derive
        for attr in &item_enum.attrs {
            if self.should_include(attr) {
                return true;
            }
        }
        false
    }

    fn should_include(&self, attr: &Attribute) -> bool {
        if let Ok(meta_list) = attr.meta.require_list() {
            if meta_list.path.is_ident("derive") {
                let tokens_str = meta_list.to_token_stream().to_string();

                if tokens_str.contains("Serialize") || tokens_str.contains("Deserialize") {
                    true
                } else {
                    false
                }
            } else  {
                false
            }
        } else {
            false
        }
    }

    fn parse_struct(&self, item_struct: &ItemStruct, file_path: &Path) -> Option<StructInfo> {
        let mut fields = Vec::new();

        match &item_struct.fields {
            syn::Fields::Named(fields_named) => {
                for field in &fields_named.named {
                    if let Some(field_info) = self.parse_field(field) {
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

    fn parse_enum(&self, item_enum: &ItemEnum, file_path: &Path) -> Option<StructInfo> {
        let mut fields = Vec::new();

        for variant in &item_enum.variants {
            let field_info = FieldInfo {
                name: variant.ident.to_string(),
                rust_type: "enum_variant".to_string(),
                typescript_type: format!("\"{}\"", variant.ident.to_string()),
                is_optional: false,
                is_public: true,
            };
            fields.push(field_info);
        }

        Some(StructInfo {
            name: item_enum.ident.to_string(),
            fields,
            file_path: file_path.to_string_lossy().to_string(),
            is_enum: true,
        })
    }

    fn parse_field(&self, field: &syn::Field) -> Option<FieldInfo> {
        let name = field.ident.as_ref()?.to_string();
        let is_public = matches!(field.vis, Visibility::Public(_));
        let is_optional = self.is_optional_type(&field.ty);
        let rust_type = self.type_to_string(&field.ty);
        let typescript_type = self.map_rust_type_to_typescript(&rust_type);

        Some(FieldInfo {
            name,
            rust_type,
            typescript_type,
            is_optional,
            is_public,
        })
    }

    pub fn analyze_file(&self, file_path: &Path) -> Result<Vec<CommandInfo>, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path)?;
        let syntax = syn::parse_file(&content)?;
        let mut commands = Vec::new();

        for item in syntax.items {
            if let syn::Item::Fn(func) = item {
                if self.is_tauri_command(&func) {
                    if let Some(command_info) = self.extract_command_info(&func, file_path) {
                        commands.push(command_info);
                    }
                }
            }
        }

        Ok(commands)
    }

    fn is_tauri_command(&self, func: &ItemFn) -> bool {
        func.attrs.iter().any(|attr| {
            attr.path().segments.len() == 2
                && attr.path().segments[0].ident == "tauri"
                && attr.path().segments[1].ident == "command"
                || attr.path().is_ident("command")
        })
    }

    fn extract_command_info(&self, func: &ItemFn, file_path: &Path) -> Option<CommandInfo> {
        let name = func.sig.ident.to_string();
        let parameters = self.extract_parameters(&func.sig.inputs);
        let return_type = self.extract_return_type(&func.sig.output);
        let is_async = func.sig.asyncness.is_some();

        // Get line number from the function's span
        let line_number = func.sig.ident.span().start().line;

        Some(CommandInfo {
            name,
            parameters,
            return_type,
            file_path: file_path.to_string_lossy().to_string(),
            line_number,
            is_async,
        })
    }

    fn extract_parameters(&self, inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>) -> Vec<ParameterInfo> {
        let mut parameters = Vec::new();

        for input in inputs {
            if let FnArg::Typed(PatType { pat, ty, .. }) = input {
                if let syn::Pat::Ident(pat_ident) = pat.as_ref() {
                    let name = pat_ident.ident.to_string();
                    let rust_type = self.type_to_string(ty);
                    let typescript_type = self.map_rust_type_to_typescript(&rust_type);
                    let is_optional = self.is_optional_type(ty);

                    parameters.push(ParameterInfo {
                        name,
                        rust_type,
                        typescript_type,
                        is_optional,
                    });
                }
            }
        }

        parameters
    }

    fn extract_return_type(&self, output: &ReturnType) -> String {
        match output {
            ReturnType::Default => "void".to_string(),
            ReturnType::Type(_, ty) => {
                let rust_type = self.type_to_string(ty);
                self.map_rust_type_to_typescript(&rust_type)
            }
        }
    }

    fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::Path(type_path) => {
                let segments: Vec<String> = type_path.path.segments.iter()
                    .map(|segment| {
                        if segment.arguments.is_empty() {
                            segment.ident.to_string()
                        } else {
                            match &segment.arguments {
                                syn::PathArguments::AngleBracketed(args) => {
                                    let inner_types: Vec<String> = args.args.iter()
                                        .filter_map(|arg| {
                                            if let syn::GenericArgument::Type(inner_ty) = arg {
                                                Some(self.type_to_string(inner_ty))
                                            } else {
                                                None
                                            }
                                        })
                                        .collect();
                                    format!("{}<{}>", segment.ident, inner_types.join(", "))
                                }
                                _ => segment.ident.to_string(),
                            }
                        }
                    })
                    .collect();
                segments.join("::")
            }
            Type::Reference(type_ref) => {
                format!("&{}", self.type_to_string(&type_ref.elem))
            }
            Type::Tuple(type_tuple) => {
                if type_tuple.elems.is_empty() {
                    "()".to_string()
                } else {
                    let types: Vec<String> = type_tuple.elems.iter()
                        .map(|t| self.type_to_string(t))
                        .collect();
                    format!("({})", types.join(", "))
                }
            }
            _ => "unknown".to_string(),
        }
    }

    pub fn map_rust_type_to_typescript(&self, rust_type: &str) -> String {
        // Handle Result<T, E> -> T
        if rust_type.starts_with("Result<") {
            if let Some(inner) = rust_type.strip_prefix("Result<").and_then(|s| s.strip_suffix('>')) {
                if let Some(comma_pos) = inner.find(',') {
                    let success_type = inner[..comma_pos].trim();
                    return self.map_rust_type_to_typescript(success_type);
                }
            }
        }

        // Handle Option<T> -> T | null
        if rust_type.starts_with("Option<") {
            if let Some(inner) = rust_type.strip_prefix("Option<").and_then(|s| s.strip_suffix('>')) {
                let inner_ts = self.map_rust_type_to_typescript(inner.trim());
                return format!("{} | null", inner_ts);
            }
        }

        // Handle Vec<T> -> T[]
        if rust_type.starts_with("Vec<") {
            if let Some(inner) = rust_type.strip_prefix("Vec<").and_then(|s| s.strip_suffix('>')) {
                let inner_ts = self.map_rust_type_to_typescript(inner.trim());
                return format!("{}[]", inner_ts);
            }
        }

        // Handle references
        if rust_type.starts_with('&') {
            return self.map_rust_type_to_typescript(&rust_type[1..]);
        }

        // Check built-in mappings
        if let Some(ts_type) = self.type_mappings.get(rust_type) {
            return ts_type.clone();
        }

        // Default to the rust type name for custom types
        rust_type.to_string()
    }

    fn is_optional_type(&self, ty: &Type) -> bool {
        if let Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                return segment.ident == "Option";
            }
        }
        false
    }

    pub fn get_discovered_structs(&self) -> &HashMap<String, StructInfo> {
        &self.discovered_structs
    }
}