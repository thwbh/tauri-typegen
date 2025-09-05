use crate::models::{CommandInfo, FieldInfo, ParameterInfo, StructInfo, ValidatorAttributes, LengthConstraint, RangeConstraint};
use quote::ToTokens;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use syn::{Attribute, FnArg, ItemEnum, ItemFn, ItemStruct, PatType, ReturnType, Type, Visibility, File as SynFile};
use walkdir::WalkDir;

/// Cache entry for a parsed Rust file
#[derive(Debug, Clone)]
struct ParsedFile {
    /// The parsed AST
    ast: SynFile,
    /// File path for reference
    path: PathBuf,
    // Last modified time for cache invalidation (if needed later)
    // modified: std::time::SystemTime,
}

/// Dependency graph for lazy type resolution
#[derive(Debug, Default)]
struct TypeDependencyGraph {
    /// Maps type name to the files where it's defined
    type_definitions: HashMap<String, PathBuf>,
    /// Maps type name to types it depends on
    dependencies: HashMap<String, HashSet<String>>,
    /// Maps type name to its resolved StructInfo
    resolved_types: HashMap<String, StructInfo>,
}

pub struct CommandAnalyzer {
    type_mappings: HashMap<String, String>,
    discovered_structs: HashMap<String, StructInfo>,
    /// Cache of parsed ASTs to avoid re-parsing files
    ast_cache: HashMap<PathBuf, ParsedFile>,
    /// Dependency graph for efficient type resolution
    dependency_graph: TypeDependencyGraph,
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
        
        // Collection type mappings
        type_mappings.insert("HashMap".to_string(), "Record".to_string());
        type_mappings.insert("BTreeMap".to_string(), "Record".to_string());
        type_mappings.insert("HashSet".to_string(), "Set".to_string());
        type_mappings.insert("BTreeSet".to_string(), "Set".to_string());

        Self {
            type_mappings,
            discovered_structs: HashMap::new(),
            ast_cache: HashMap::new(),
            dependency_graph: TypeDependencyGraph::default(),
        }
    }

    pub fn analyze_project(
        &mut self,
        project_path: &str,
    ) -> Result<Vec<CommandInfo>, Box<dyn std::error::Error>> {
        // Single pass: Parse all Rust files and cache ASTs
        self.parse_and_cache_all_files(project_path)?;
        
        // Extract commands from cached ASTs
        let mut commands = Vec::new();
        let mut type_names_to_discover = HashSet::new();
        
        // First, collect all the files we need to process
        let file_paths: Vec<PathBuf> = self.ast_cache.keys().cloned().collect();
        
        for file_path in file_paths {
            if let Some(parsed_file) = self.ast_cache.get(&file_path).cloned() {
                println!("🔍 Analyzing file: {}", parsed_file.path.display());
                
                // Extract commands from this file's AST
                let file_commands = self.extract_commands_from_ast(&parsed_file.ast, &parsed_file.path)?;
                
                // Collect type names from command parameters and return types
                for cmd in &file_commands {
                    for param in &cmd.parameters {
                        self.extract_type_names(&param.rust_type, &mut type_names_to_discover);
                    }
                    self.extract_type_names(&cmd.return_type, &mut type_names_to_discover);
                }
                
                commands.extend(file_commands);
                
                // Build type definition index from this file
                self.index_type_definitions(&parsed_file.ast, &parsed_file.path);
            }
        }
        
        println!("🔍 Type names to discover: {:?}", type_names_to_discover);
        
        // Lazy type resolution: Resolve types on demand using dependency graph
        self.resolve_types_lazily(&type_names_to_discover)?;
        
        println!(
            "🏗️  Discovered {} structs total",
            self.discovered_structs.len()
        );
        for (name, info) in &self.discovered_structs {
            println!("  - {}: {} fields", name, info.fields.len());
        }

        Ok(commands)
    }

    /// Parse and cache all Rust files in a single traversal
    fn parse_and_cache_all_files(&mut self, project_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        for entry in WalkDir::new(project_path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                if let Some(extension) = entry.path().extension() {
                    if extension == "rs" {
                        let file_path = entry.path().to_path_buf();
                        
                        // Skip if already cached (in case of duplicate paths)
                        if self.ast_cache.contains_key(&file_path) {
                            continue;
                        }
                        
                        // Parse and cache the AST
                        match self.parse_and_cache_file(&file_path) {
                            Ok(_) => {
                                println!("📁 Cached AST for: {}", file_path.display());
                            }
                            Err(e) => {
                                eprintln!("⚠️  Failed to parse {}: {}", file_path.display(), e);
                                // Continue with other files even if one fails
                            }
                        }
                    }
                }
            }
        }
        
        println!("🗂️  Cached {} ASTs", self.ast_cache.len());
        Ok(())
    }
    
    /// Parse a single file and add it to the cache
    fn parse_and_cache_file(&mut self, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path)?;
        let ast = syn::parse_file(&content)?;
        
        let parsed_file = ParsedFile {
            ast,
            path: file_path.clone(),
        };
        
        self.ast_cache.insert(file_path.clone(), parsed_file);
        Ok(())
    }
    
    /// Extract commands from a cached AST (replaces analyze_file)
    fn extract_commands_from_ast(
        &self,
        ast: &SynFile,
        file_path: &PathBuf,
    ) -> Result<Vec<CommandInfo>, Box<dyn std::error::Error>> {
        let mut commands = Vec::new();

        for item in &ast.items {
            if let syn::Item::Fn(func) = item {
                if self.is_tauri_command(func) {
                    if let Some(command_info) = self.extract_command_info(func, file_path) {
                        commands.push(command_info);
                    }
                }
            }
        }

        Ok(commands)
    }
    
    /// Build an index of type definitions from an AST
    fn index_type_definitions(&mut self, ast: &SynFile, file_path: &PathBuf) {
        for item in &ast.items {
            match item {
                syn::Item::Struct(item_struct) => {
                    if self.should_include_struct(item_struct) {
                        let struct_name = item_struct.ident.to_string();
                        self.dependency_graph.type_definitions.insert(struct_name, file_path.clone());
                    }
                }
                syn::Item::Enum(item_enum) => {
                    if self.should_include_enum(item_enum) {
                        let enum_name = item_enum.ident.to_string();
                        self.dependency_graph.type_definitions.insert(enum_name, file_path.clone());
                    }
                }
                _ => {}
            }
        }
    }
    
    /// Lazily resolve types using the dependency graph
    fn resolve_types_lazily(&mut self, initial_types: &HashSet<String>) -> Result<(), Box<dyn std::error::Error>> {
        let mut types_to_resolve: Vec<String> = initial_types.iter().cloned().collect();
        let mut resolved_types = HashSet::new();
        
        while let Some(type_name) = types_to_resolve.pop() {
            // Skip if already resolved
            if resolved_types.contains(&type_name) || self.discovered_structs.contains_key(&type_name) {
                continue;
            }
            
            // Try to resolve this type
            if let Some(file_path) = self.dependency_graph.type_definitions.get(&type_name).cloned() {
                if let Some(parsed_file) = self.ast_cache.get(&file_path) {
                    // Find and parse the specific type from the cached AST
                    if let Some(struct_info) = self.extract_type_from_ast(&parsed_file.ast, &type_name, &file_path) {
                        // Collect dependencies of this type
                        let mut type_dependencies = HashSet::new();
                        for field in &struct_info.fields {
                            self.extract_type_names(&field.rust_type, &mut type_dependencies);
                        }
                        
                        // Add dependencies to the resolution queue
                        for dep_type in &type_dependencies {
                            if !resolved_types.contains(dep_type) 
                                && !self.discovered_structs.contains_key(dep_type) 
                                && self.dependency_graph.type_definitions.contains_key(dep_type) {
                                types_to_resolve.push(dep_type.clone());
                            }
                        }
                        
                        // Store the resolved type
                        self.dependency_graph.dependencies.insert(type_name.clone(), type_dependencies);
                        self.discovered_structs.insert(type_name.clone(), struct_info);
                        resolved_types.insert(type_name);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Extract a specific type from a cached AST
    fn extract_type_from_ast(&self, ast: &SynFile, type_name: &str, file_path: &PathBuf) -> Option<StructInfo> {
        for item in &ast.items {
            match item {
                syn::Item::Struct(item_struct) => {
                    if item_struct.ident.to_string() == type_name && self.should_include_struct(item_struct) {
                        return self.parse_struct(item_struct, file_path);
                    }
                }
                syn::Item::Enum(item_enum) => {
                    if item_enum.ident.to_string() == type_name && self.should_include_enum(item_enum) {
                        return self.parse_enum(item_enum, file_path);
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub fn extract_type_names(&self, rust_type: &str, type_names: &mut HashSet<String>) {
        self.extract_type_names_recursive(rust_type, type_names);
    }
    
    fn extract_type_names_recursive(&self, rust_type: &str, type_names: &mut HashSet<String>) {
        let rust_type = rust_type.trim();
        
        // Handle Result<T, E> - extract both T and E
        if rust_type.starts_with("Result<") {
            if let Some(inner) = rust_type.strip_prefix("Result<").and_then(|s| s.strip_suffix(">")) {
                if let Some(comma_pos) = inner.find(',') {
                    let ok_type = inner[..comma_pos].trim();
                    let err_type = inner[comma_pos + 1..].trim();
                    self.extract_type_names_recursive(ok_type, type_names);
                    self.extract_type_names_recursive(err_type, type_names);
                }
            }
            return;
        }
        
        // Handle Option<T> - extract T
        if rust_type.starts_with("Option<") {
            if let Some(inner) = rust_type.strip_prefix("Option<").and_then(|s| s.strip_suffix(">")) {
                self.extract_type_names_recursive(inner, type_names);
            }
            return;
        }
        
        // Handle Vec<T> - extract T
        if rust_type.starts_with("Vec<") {
            if let Some(inner) = rust_type.strip_prefix("Vec<").and_then(|s| s.strip_suffix(">")) {
                self.extract_type_names_recursive(inner, type_names);
            }
            return;
        }
        
        // Handle HashMap<K, V> and BTreeMap<K, V> - extract K and V
        if rust_type.starts_with("HashMap<") || rust_type.starts_with("BTreeMap<") {
            let prefix = if rust_type.starts_with("HashMap<") { "HashMap<" } else { "BTreeMap<" };
            if let Some(inner) = rust_type.strip_prefix(prefix).and_then(|s| s.strip_suffix(">")) {
                if let Some(comma_pos) = inner.find(',') {
                    let key_type = inner[..comma_pos].trim();
                    let value_type = inner[comma_pos + 1..].trim();
                    self.extract_type_names_recursive(key_type, type_names);
                    self.extract_type_names_recursive(value_type, type_names);
                }
            }
            return;
        }
        
        // Handle HashSet<T> and BTreeSet<T> - extract T
        if rust_type.starts_with("HashSet<") || rust_type.starts_with("BTreeSet<") {
            let prefix = if rust_type.starts_with("HashSet<") { "HashSet<" } else { "BTreeSet<" };
            if let Some(inner) = rust_type.strip_prefix(prefix).and_then(|s| s.strip_suffix(">")) {
                self.extract_type_names_recursive(inner, type_names);
            }
            return;
        }
        
        // Handle tuple types like (T, U, V)
        if rust_type.starts_with('(') && rust_type.ends_with(')') && rust_type != "()" {
            let inner = &rust_type[1..rust_type.len()-1];
            for part in inner.split(',') {
                self.extract_type_names_recursive(part.trim(), type_names);
            }
            return;
        }
        
        // Handle references
        if rust_type.starts_with('&') {
            let without_ref = rust_type.trim_start_matches('&');
            self.extract_type_names_recursive(without_ref, type_names);
            return;
        }
        
        // Check if this is a custom type name
        if !rust_type.is_empty()
            && !self.type_mappings.contains_key(rust_type)
            && !rust_type.starts_with(char::is_lowercase) // Skip built-in types
            && rust_type.chars().next().map_or(false, char::is_alphabetic)
            && !rust_type.contains('<') // Skip generic type names with parameters
        {
            type_names.insert(rust_type.to_string());
        }
    }

    // Note: discover_specific_types_in_file and discover_types_in_file methods
    // have been removed as they're replaced by the optimized AST caching system

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
            } else {
                false
            }
        } else {
            false
        }
    }

    fn parse_struct(&self, item_struct: &ItemStruct, file_path: &PathBuf) -> Option<StructInfo> {
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

    fn parse_enum(&self, item_enum: &ItemEnum, file_path: &PathBuf) -> Option<StructInfo> {
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
                        .map(|field| self.map_rust_type_to_typescript(&self.type_to_string(&field.ty)))
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
                            let field_type = self.map_rust_type_to_typescript(&self.type_to_string(&field.ty));
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

    fn parse_field(&self, field: &syn::Field) -> Option<FieldInfo> {
        let name = field.ident.as_ref()?.to_string();
        let is_public = matches!(field.vis, Visibility::Public(_));
        let is_optional = self.is_optional_type(&field.ty);
        let rust_type = self.type_to_string(&field.ty);
        let typescript_type = self.map_rust_type_to_typescript(&rust_type);
        let validator_attributes = self.parse_validator_attributes(&field.attrs);

        Some(FieldInfo {
            name,
            rust_type,
            typescript_type,
            is_optional,
            is_public,
            validator_attributes,
        })
    }

    fn parse_validator_attributes(&self, attrs: &[syn::Attribute]) -> Option<ValidatorAttributes> {
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

    fn parse_length_from_tokens(&self, tokens: &str) -> Option<LengthConstraint> {
        if !tokens.contains("length") {
            return None;
        }
        
        let mut constraint = LengthConstraint {
            min: None,
            max: None,
            message: None,
        };
        
        // Simple regex-like parsing for length(min = X, max = Y)
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
                }
            }
        }
        
        Some(constraint)
    }

    fn parse_range_from_tokens(&self, tokens: &str) -> Option<RangeConstraint> {
        if !tokens.contains("range") {
            return None;
        }
        
        let mut constraint = RangeConstraint {
            min: None,
            max: None,
            message: None,
        };
        
        // Simple regex-like parsing for range(min = X, max = Y)
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
                }
            }
        }
        
        Some(constraint)
    }

    /// Analyze a single file for Tauri commands (backward compatibility for tests)
    pub fn analyze_file(
        &mut self,
        file_path: &std::path::Path,
    ) -> Result<Vec<CommandInfo>, Box<dyn std::error::Error>> {
        let path_buf = file_path.to_path_buf();
        
        // Parse and cache this single file - handle syntax errors gracefully
        match self.parse_and_cache_file(&path_buf) {
            Ok(_) => {
                // Extract commands from the cached AST
                if let Some(parsed_file) = self.ast_cache.get(&path_buf) {
                    self.extract_commands_from_ast(&parsed_file.ast, &path_buf)
                } else {
                    Ok(vec![])
                }
            }
            Err(_) => {
                // Return empty vector for files with syntax errors (backward compatibility)
                Ok(vec![])
            }
        }
    }

    fn is_tauri_command(&self, func: &ItemFn) -> bool {
        func.attrs.iter().any(|attr| {
            attr.path().segments.len() == 2
                && attr.path().segments[0].ident == "tauri"
                && attr.path().segments[1].ident == "command"
                || attr.path().is_ident("command")
        })
    }

    fn extract_command_info(&self, func: &ItemFn, file_path: &PathBuf) -> Option<CommandInfo> {
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

    fn extract_parameters(
        &self,
        inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
    ) -> Vec<ParameterInfo> {
        let mut parameters = Vec::new();

        for input in inputs {
            if let FnArg::Typed(PatType { pat, ty, .. }) = input {
                if let syn::Pat::Ident(pat_ident) = pat.as_ref() {
                    let name = pat_ident.ident.to_string();
                    let rust_type = self.type_to_string(ty);
                    
                    // Skip Tauri-specific parameters
                    if self.is_tauri_parameter(&name, &rust_type) {
                        continue;
                    }
                    
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
    
    fn is_tauri_parameter(&self, name: &str, rust_type: &str) -> bool {
        // Common Tauri parameter names
        if matches!(name, "app" | "window" | "state" | "handle") {
            return true;
        }
        
        // Common Tauri parameter types
        if rust_type.contains("AppHandle") 
            || rust_type.contains("Window") 
            || rust_type.contains("State") 
            || rust_type.contains("Manager") {
            return true;
        }
        
        false
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
                let segments: Vec<String> = type_path
                    .path
                    .segments
                    .iter()
                    .map(|segment| {
                        if segment.arguments.is_empty() {
                            segment.ident.to_string()
                        } else {
                            match &segment.arguments {
                                syn::PathArguments::AngleBracketed(args) => {
                                    let inner_types: Vec<String> = args
                                        .args
                                        .iter()
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
                    let types: Vec<String> = type_tuple
                        .elems
                        .iter()
                        .map(|t| self.type_to_string(t))
                        .collect();
                    format!("({})", types.join(", "))
                }
            }
            _ => "unknown".to_string(),
        }
    }

    pub fn map_rust_type_to_typescript(&self, rust_type: &str) -> String {
        let rust_type = rust_type.trim();
        
        // Handle Result<T, E> -> T
        if rust_type.starts_with("Result<") {
            if let Some(inner) = rust_type
                .strip_prefix("Result<")
                .and_then(|s| s.strip_suffix('>'))
            {
                if let Some(comma_pos) = inner.find(',') {
                    let success_type = inner[..comma_pos].trim();
                    return self.map_rust_type_to_typescript(success_type);
                }
            }
        }

        // Handle Option<T> -> T | null
        if rust_type.starts_with("Option<") {
            if let Some(inner) = rust_type
                .strip_prefix("Option<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let inner_ts = self.map_rust_type_to_typescript(inner.trim());
                return format!("{} | null", inner_ts);
            }
        }

        // Handle Vec<T> -> T[]
        if rust_type.starts_with("Vec<") {
            if let Some(inner) = rust_type
                .strip_prefix("Vec<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let inner_ts = self.map_rust_type_to_typescript(inner.trim());
                // Add parentheses if the inner type contains operators like |
                if inner_ts.contains(" | ") {
                    return format!("({})[]", inner_ts);
                } else {
                    return format!("{}[]", inner_ts);
                }
            }
        }
        
        // Handle HashMap<K, V> and BTreeMap<K, V> -> Record<K, V>
        if rust_type.starts_with("HashMap<") || rust_type.starts_with("BTreeMap<") {
            let prefix = if rust_type.starts_with("HashMap<") { "HashMap<" } else { "BTreeMap<" };
            if let Some(inner) = rust_type
                .strip_prefix(prefix)
                .and_then(|s| s.strip_suffix('>'))
            {
                if let Some(comma_pos) = inner.find(',') {
                    let key_type = inner[..comma_pos].trim();
                    let value_type = inner[comma_pos + 1..].trim();
                    let key_ts = self.map_rust_type_to_typescript(key_type);
                    let value_ts = self.map_rust_type_to_typescript(value_type);
                    return format!("Record<{}, {}>", key_ts, value_ts);
                }
            }
        }
        
        // Handle HashSet<T> and BTreeSet<T> -> Set<T> (for TypeScript)
        if rust_type.starts_with("HashSet<") || rust_type.starts_with("BTreeSet<") {
            let prefix = if rust_type.starts_with("HashSet<") { "HashSet<" } else { "BTreeSet<" };
            if let Some(inner) = rust_type
                .strip_prefix(prefix)
                .and_then(|s| s.strip_suffix('>'))
            {
                let inner_ts = self.map_rust_type_to_typescript(inner.trim());
                // For now, represent sets as arrays since TypeScript Set isn't JSON serializable
                if inner_ts.contains(" | ") {
                    return format!("({})[]", inner_ts);
                } else {
                    return format!("{}[]", inner_ts);
                }
            }
        }
        
        // Handle tuple types like (T, U, V) -> [T, U, V]
        if rust_type.starts_with('(') && rust_type.ends_with(')') {
            if rust_type == "()" {
                return "void".to_string();
            }
            let inner = &rust_type[1..rust_type.len()-1];
            let parts: Vec<String> = inner
                .split(',')
                .map(|part| self.map_rust_type_to_typescript(part.trim()))
                .collect();
            return format!("[{}]", parts.join(", "));
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
    
    /// Get the dependency graph for visualization
    pub fn get_dependency_graph(&self) -> &TypeDependencyGraph {
        &self.dependency_graph
    }

    /// Sort types topologically to ensure dependencies are declared before being used
    pub fn topological_sort_types(&self, types: &HashSet<String>) -> Vec<String> {
        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();
        
        for type_name in types {
            if !visited.contains(type_name) {
                self.topological_visit(type_name, &mut sorted, &mut visited, &mut visiting);
            }
        }
        
        sorted
    }
    
    fn topological_visit(
        &self, 
        type_name: &str, 
        sorted: &mut Vec<String>, 
        visited: &mut HashSet<String>, 
        visiting: &mut HashSet<String>
    ) {
        if visited.contains(type_name) {
            return;
        }
        
        if visiting.contains(type_name) {
            // Circular dependency detected - skip to avoid infinite loop
            return;
        }
        
        visiting.insert(type_name.to_string());
        
        // Visit all dependencies first
        if let Some(deps) = self.dependency_graph.dependencies.get(type_name) {
            for dep in deps {
                self.topological_visit(dep, sorted, visited, visiting);
            }
        }
        
        visiting.remove(type_name);
        visited.insert(type_name.to_string());
        sorted.push(type_name.to_string());
    }
    
    /// Generate a text-based visualization of the dependency graph
    pub fn visualize_dependencies(&self, commands: &[CommandInfo]) -> String {
        let mut output = String::new();
        
        output.push_str("🌐 Type Dependency Graph\n");
        output.push_str("======================\n\n");
        
        // Show command entry points
        output.push_str("📋 Command Entry Points:\n");
        for cmd in commands {
            output.push_str(&format!("• {} ({}:{})\n", cmd.name, 
                cmd.file_path.split('/').last().unwrap_or(&cmd.file_path), 
                cmd.line_number));
            
            // Show parameter types
            for param in &cmd.parameters {
                let clean_type = self.clean_type_name(&param.rust_type);
                output.push_str(&format!("  ├─ {}: {} → {}\n", 
                    param.name, clean_type, param.typescript_type));
            }
            
            // Show return type
            let clean_return = self.clean_type_name(&cmd.return_type);
            output.push_str(&format!("  └─ returns: {} → {}\n", 
                clean_return, cmd.return_type));
        }
        
        output.push_str("\n🏗️  Discovered Types:\n");
        for (type_name, struct_info) in &self.discovered_structs {
            let type_kind = if struct_info.is_enum { "enum" } else { "struct" };
            let file_name = struct_info.file_path.split('/').last().unwrap_or(&struct_info.file_path);
            
            output.push_str(&format!("• {} ({}) - {} fields - defined in {}\n", 
                type_name, type_kind, struct_info.fields.len(), file_name));
                
            // Show field dependencies
            if let Some(deps) = self.dependency_graph.dependencies.get(type_name) {
                if !deps.is_empty() {
                    output.push_str("  └─ depends on: ");
                    let dep_list: Vec<String> = deps.iter().cloned().collect();
                    output.push_str(&dep_list.join(", "));
                    output.push_str("\n");
                }
            }
        }
        
        // Show dependency chains
        output.push_str("\n🔗 Dependency Chains:\n");
        let mut visited = HashSet::new();
        for type_name in self.discovered_structs.keys() {
            if !visited.contains(type_name) {
                self.show_dependency_chain(type_name, &mut output, &mut visited, 0);
            }
        }
        
        output.push_str("\n📊 Summary:\n");
        output.push_str(&format!("• {} commands analyzed\n", commands.len()));
        output.push_str(&format!("• {} types discovered\n", self.discovered_structs.len()));
        output.push_str(&format!("• {} files with type definitions\n", 
            self.dependency_graph.type_definitions.values().collect::<HashSet<_>>().len()));
        
        output
    }
    
    fn clean_type_name<'a>(&self, rust_type: &'a str) -> &'a str {
        // Remove common wrapper types for cleaner display
        if let Some(inner) = rust_type.strip_prefix("Result<") {
            if let Some(comma_pos) = inner.find(',') {
                return inner[..comma_pos].trim();
            }
        }
        if let Some(inner) = rust_type.strip_prefix("Option<") {
            if let Some(suffix_pos) = inner.rfind('>') {
                return inner[..suffix_pos].trim();
            }
        }
        rust_type
    }
    
    fn show_dependency_chain(&self, type_name: &str, output: &mut String, visited: &mut HashSet<String>, depth: usize) {
        if visited.contains(type_name) || depth > 10 {
            return;
        }
        visited.insert(type_name.to_string());
        
        let indent = "  ".repeat(depth);
        output.push_str(&format!("{}├─ {}\n", indent, type_name));
        
        if let Some(deps) = self.dependency_graph.dependencies.get(type_name) {
            for dep in deps {
                self.show_dependency_chain(dep, output, visited, depth + 1);
            }
        }
    }
    
    /// Generate a DOT graph format for advanced visualization tools
    pub fn generate_dot_graph(&self, commands: &[CommandInfo]) -> String {
        let mut dot = String::new();
        dot.push_str("digraph TypeDependencies {\n");
        dot.push_str("  rankdir=TB;\n");
        dot.push_str("  node [shape=box, style=rounded];\n\n");
        
        // Command nodes
        dot.push_str("  // Commands\n");
        for cmd in commands {
            dot.push_str(&format!("  \"cmd_{}\" [label=\"{}\\n(command)\", color=blue, style=\"filled,rounded\"];\n", 
                cmd.name, cmd.name));
        }
        
        // Type nodes
        dot.push_str("\n  // Types\n");
        for (type_name, struct_info) in &self.discovered_structs {
            let color = if struct_info.is_enum { "orange" } else { "lightgreen" };
            dot.push_str(&format!("  \"{}\" [label=\"{}\\n({} fields)\", color={}, style=\"filled,rounded\"];\n", 
                type_name, type_name, struct_info.fields.len(), color));
        }
        
        // Command to type edges
        dot.push_str("\n  // Command dependencies\n");
        for cmd in commands {
            for param in &cmd.parameters {
                let clean_type = self.clean_type_name(&param.rust_type);
                if self.discovered_structs.contains_key(clean_type) {
                    dot.push_str(&format!("  \"cmd_{}\" -> \"{}\" [label=\"{}\", color=blue];\n", 
                        cmd.name, clean_type, param.name));
                }
            }
        }
        
        // Type to type edges
        dot.push_str("\n  // Type dependencies\n");
        for (type_name, deps) in &self.dependency_graph.dependencies {
            for dep in deps {
                if self.discovered_structs.contains_key(dep) {
                    dot.push_str(&format!("  \"{}\" -> \"{}\" [color=gray];\n", type_name, dep));
                }
            }
        }
        
        dot.push_str("}\n");
        dot
    }
    
    // Note: discover_nested_dependencies method removed - replaced by the
    // lazy type resolution system that handles dependencies more efficiently
}
