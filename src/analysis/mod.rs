pub mod ast_cache;
pub mod command_parser;
pub mod dependency_graph;
pub mod struct_parser;
pub mod type_resolver;
pub mod validator_parser;

use crate::models::{CommandInfo, StructInfo};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use ast_cache::AstCache;
use command_parser::CommandParser;
use dependency_graph::TypeDependencyGraph;
use struct_parser::StructParser;
use type_resolver::TypeResolver;

/// Comprehensive analyzer that orchestrates all analysis sub-modules
pub struct CommandAnalyzer {
    /// AST cache for parsed files
    ast_cache: AstCache,
    /// Command parser for extracting Tauri commands
    command_parser: CommandParser,
    /// Struct parser for extracting type definitions
    struct_parser: StructParser,
    /// Type resolver for Rust to TypeScript type mappings
    type_resolver: TypeResolver,
    /// Dependency graph for type resolution
    dependency_graph: TypeDependencyGraph,
    /// Discovered struct definitions
    discovered_structs: HashMap<String, StructInfo>,
}

impl CommandAnalyzer {
    pub fn new() -> Self {
        Self {
            ast_cache: AstCache::new(),
            command_parser: CommandParser::new(),
            struct_parser: StructParser::new(),
            type_resolver: TypeResolver::new(),
            dependency_graph: TypeDependencyGraph::new(),
            discovered_structs: HashMap::new(),
        }
    }

    /// Analyze a complete project for Tauri commands and types
    pub fn analyze_project(
        &mut self,
        project_path: &str,
    ) -> Result<Vec<CommandInfo>, Box<dyn std::error::Error>> {
        // Single pass: Parse all Rust files and cache ASTs
        self.ast_cache.parse_and_cache_all_files(project_path)?;

        // Extract commands from cached ASTs
        let mut commands = Vec::new();
        let mut type_names_to_discover = HashSet::new();

        // Collect file paths from cache
        let file_paths: Vec<PathBuf> = self.ast_cache.keys().cloned().collect();

        for file_path in file_paths {
            if let Some(parsed_file) = self.ast_cache.get_cloned(&file_path) {
                println!("🔍 Analyzing file: {}", parsed_file.path.display());

                // Extract commands from this file's AST
                let file_commands = self.command_parser.extract_commands_from_ast(
                    &parsed_file.ast,
                    parsed_file.path.as_path(),
                    &mut self.type_resolver,
                )?;

                // Collect type names from command parameters and return types
                for cmd in &file_commands {
                    for param in &cmd.parameters {
                        self.extract_type_names(&param.rust_type, &mut type_names_to_discover);
                    }
                    self.extract_type_names(&cmd.return_type, &mut type_names_to_discover);
                }

                commands.extend(file_commands);

                // Build type definition index from this file
                self.index_type_definitions(&parsed_file.ast, parsed_file.path.as_path());
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

    /// Analyze a single file for Tauri commands (backward compatibility for tests)
    pub fn analyze_file(
        &mut self,
        file_path: &std::path::Path,
    ) -> Result<Vec<CommandInfo>, Box<dyn std::error::Error>> {
        let path_buf = file_path.to_path_buf();

        // Parse and cache this single file - handle syntax errors gracefully
        match self.ast_cache.parse_and_cache_file(&path_buf) {
            Ok(_) => {
                // Extract commands from the cached AST
                if let Some(parsed_file) = self.ast_cache.get_cloned(&path_buf) {
                    self.command_parser.extract_commands_from_ast(
                        &parsed_file.ast,
                        path_buf.as_path(),
                        &mut self.type_resolver,
                    )
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

    /// Build an index of type definitions from an AST
    fn index_type_definitions(&mut self, ast: &syn::File, file_path: &Path) {
        for item in &ast.items {
            match item {
                syn::Item::Struct(item_struct) => {
                    if self.struct_parser.should_include_struct(item_struct) {
                        let struct_name = item_struct.ident.to_string();
                        self.dependency_graph
                            .add_type_definition(struct_name, file_path.to_path_buf());
                    }
                }
                syn::Item::Enum(item_enum) => {
                    if self.struct_parser.should_include_enum(item_enum) {
                        let enum_name = item_enum.ident.to_string();
                        self.dependency_graph
                            .add_type_definition(enum_name, file_path.to_path_buf());
                    }
                }
                _ => {}
            }
        }
    }

    /// Lazily resolve types using the dependency graph
    fn resolve_types_lazily(
        &mut self,
        initial_types: &HashSet<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut types_to_resolve: Vec<String> = initial_types.iter().cloned().collect();
        let mut resolved_types = HashSet::new();

        while let Some(type_name) = types_to_resolve.pop() {
            // Skip if already resolved
            if resolved_types.contains(&type_name)
                || self.discovered_structs.contains_key(&type_name)
            {
                continue;
            }

            // Try to resolve this type
            if let Some(file_path) = self
                .dependency_graph
                .get_type_definition_path(&type_name)
                .cloned()
            {
                if let Some(parsed_file) = self.ast_cache.get_cloned(&file_path) {
                    // Find and parse the specific type from the cached AST
                    if let Some(struct_info) =
                        self.extract_type_from_ast(&parsed_file.ast, &type_name, file_path.as_path())
                    {
                        // Collect dependencies of this type
                        let mut type_dependencies = HashSet::new();
                        for field in &struct_info.fields {
                            self.extract_type_names(&field.rust_type, &mut type_dependencies);
                        }

                        // Add dependencies to the resolution queue
                        for dep_type in &type_dependencies {
                            if !resolved_types.contains(dep_type)
                                && !self.discovered_structs.contains_key(dep_type)
                                && self
                                    .dependency_graph
                                    .has_type_definition(dep_type)
                            {
                                types_to_resolve.push(dep_type.clone());
                            }
                        }

                        // Store the resolved type
                        self.dependency_graph
                            .add_dependencies(type_name.clone(), type_dependencies.clone());
                        self.dependency_graph
                            .add_resolved_type(type_name.clone(), struct_info.clone());
                        self.discovered_structs
                            .insert(type_name.clone(), struct_info);
                        resolved_types.insert(type_name);
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract a specific type from a cached AST
    fn extract_type_from_ast(
        &mut self,
        ast: &syn::File,
        type_name: &str,
        file_path: &Path,
    ) -> Option<StructInfo> {
        for item in &ast.items {
            match item {
                syn::Item::Struct(item_struct) => {
                    if item_struct.ident == type_name
                        && self.struct_parser.should_include_struct(item_struct)
                    {
                        return self.struct_parser.parse_struct(item_struct, file_path, &mut self.type_resolver);
                    }
                }
                syn::Item::Enum(item_enum) => {
                    if item_enum.ident == type_name
                        && self.struct_parser.should_include_enum(item_enum)
                    {
                        return self.struct_parser.parse_enum(item_enum, file_path, &mut self.type_resolver);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Extract type names from a Rust type string
    pub fn extract_type_names(&self, rust_type: &str, type_names: &mut HashSet<String>) {
        self.extract_type_names_recursive(rust_type, type_names);
    }

    /// Recursively extract type names from complex types
    fn extract_type_names_recursive(&self, rust_type: &str, type_names: &mut HashSet<String>) {
        let rust_type = rust_type.trim();

        // Handle Result<T, E> - extract both T and E
        if rust_type.starts_with("Result<") {
            if let Some(inner) = rust_type
                .strip_prefix("Result<")
                .and_then(|s| s.strip_suffix(">"))
            {
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
            if let Some(inner) = rust_type
                .strip_prefix("Option<")
                .and_then(|s| s.strip_suffix(">"))
            {
                self.extract_type_names_recursive(inner, type_names);
            }
            return;
        }

        // Handle Vec<T> - extract T
        if rust_type.starts_with("Vec<") {
            if let Some(inner) = rust_type
                .strip_prefix("Vec<")
                .and_then(|s| s.strip_suffix(">"))
            {
                self.extract_type_names_recursive(inner, type_names);
            }
            return;
        }

        // Handle HashMap<K, V> and BTreeMap<K, V> - extract K and V
        if rust_type.starts_with("HashMap<") || rust_type.starts_with("BTreeMap<") {
            let prefix = if rust_type.starts_with("HashMap<") {
                "HashMap<"
            } else {
                "BTreeMap<"
            };
            if let Some(inner) = rust_type
                .strip_prefix(prefix)
                .and_then(|s| s.strip_suffix(">"))
            {
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
            let prefix = if rust_type.starts_with("HashSet<") {
                "HashSet<"
            } else {
                "BTreeSet<"
            };
            if let Some(inner) = rust_type
                .strip_prefix(prefix)
                .and_then(|s| s.strip_suffix(">"))
            {
                self.extract_type_names_recursive(inner, type_names);
            }
            return;
        }

        // Handle tuple types like (T, U, V)
        if rust_type.starts_with('(') && rust_type.ends_with(')') && rust_type != "()" {
            let inner = &rust_type[1..rust_type.len() - 1];
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
            && !self.type_resolver.get_type_mappings().contains_key(rust_type)
            && !rust_type.starts_with(char::is_lowercase) // Skip built-in types
            && rust_type.chars().next().is_some_and(char::is_alphabetic)
            && !rust_type.contains('<') // Skip generic type names with parameters
        {
            type_names.insert(rust_type.to_string());
        }
    }

    /// Get discovered structs
    pub fn get_discovered_structs(&self) -> &HashMap<String, StructInfo> {
        &self.discovered_structs
    }

    /// Get the dependency graph for visualization
    pub fn get_dependency_graph(&self) -> &TypeDependencyGraph {
        &self.dependency_graph
    }

    /// Sort types topologically to ensure dependencies are declared before being used
    pub fn topological_sort_types(&self, types: &HashSet<String>) -> Vec<String> {
        self.dependency_graph.topological_sort_types(types)
    }

    /// Generate a text-based visualization of the dependency graph
    pub fn visualize_dependencies(&self, commands: &[CommandInfo]) -> String {
        self.dependency_graph.visualize_dependencies(commands)
    }

    /// Generate a DOT graph visualization of the dependency graph
    pub fn generate_dot_graph(&self, commands: &[CommandInfo]) -> String {
        self.dependency_graph.generate_dot_graph(commands)
    }

    /// Map a Rust type to its TypeScript equivalent
    pub fn map_rust_type_to_typescript(&mut self, rust_type: &str) -> String {
        self.type_resolver.map_rust_type_to_typescript(rust_type)
    }
}

impl Default for CommandAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}