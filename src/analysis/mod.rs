pub mod ast_cache;
pub mod channel_parser;
pub mod command_parser;
pub mod dependency_graph;
pub mod event_parser;
pub mod serde_parser;
pub mod struct_parser;
pub mod type_resolver;
pub mod validator_parser;

use crate::models::{ChannelInfo, CommandInfo, EventInfo, StructInfo};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use ast_cache::AstCache;
use channel_parser::ChannelParser;
use command_parser::CommandParser;
use dependency_graph::TypeDependencyGraph;
use event_parser::EventParser;
use struct_parser::StructParser;
use type_resolver::TypeResolver;

/// Analyzer that orchestrates all analysis sub-modules
pub struct CommandAnalyzer {
    /// AST cache for parsed files
    ast_cache: AstCache,
    /// Command parser for extracting Tauri commands
    command_parser: CommandParser,
    /// Channel parser for extracting channel parameters
    channel_parser: ChannelParser,
    /// Event parser for extracting event emissions
    event_parser: EventParser,
    /// Struct parser for extracting type definitions
    struct_parser: StructParser,
    /// Type resolver for Rust to TypeScript type mappings
    type_resolver: TypeResolver,
    /// Dependency graph for type resolution
    dependency_graph: TypeDependencyGraph,
    /// Discovered struct definitions
    discovered_structs: HashMap<String, StructInfo>,
    /// Discovered event emissions
    discovered_events: Vec<EventInfo>,
}

impl CommandAnalyzer {
    pub fn new() -> Self {
        Self {
            ast_cache: AstCache::new(),
            command_parser: CommandParser::new(),
            channel_parser: ChannelParser::new(),
            event_parser: EventParser::new(),
            struct_parser: StructParser::new(),
            type_resolver: TypeResolver::new(),
            dependency_graph: TypeDependencyGraph::new(),
            discovered_structs: HashMap::new(),
            discovered_events: Vec::new(),
        }
    }

    /// Add custom type mappings from configuration
    pub fn add_type_mappings(&mut self, mappings: &HashMap<String, String>) {
        for (rust_type, ts_type) in mappings {
            self.type_resolver
                .add_type_mapping(rust_type.clone(), ts_type.clone());
        }
    }

    /// Analyze a complete project for Tauri commands and types
    pub fn analyze_project(
        &mut self,
        project_path: &str,
    ) -> Result<Vec<CommandInfo>, Box<dyn std::error::Error>> {
        self.analyze_project_with_verbose(project_path, false)
    }

    /// Analyze a complete project for Tauri commands and types with verbose output
    pub fn analyze_project_with_verbose(
        &mut self,
        project_path: &str,
        verbose: bool,
    ) -> Result<Vec<CommandInfo>, Box<dyn std::error::Error>> {
        // Single pass: Parse all Rust files and cache ASTs
        self.ast_cache
            .parse_and_cache_all_files(project_path, verbose)?;

        // Extract commands from cached ASTs
        let file_paths: Vec<PathBuf> = self.ast_cache.keys().cloned().collect();
        let mut commands = Vec::new();
        let mut type_names_to_discover = HashSet::new();

        // Process each file - using functional style where possible
        for file_path in file_paths {
            if let Some(parsed_file) = self.ast_cache.get_cloned(&file_path) {
                if verbose {
                    println!("🔍 Analyzing file: {}", parsed_file.path.display());
                }

                // Extract commands from this file's AST
                let mut file_commands = self.command_parser.extract_commands_from_ast(
                    &parsed_file.ast,
                    parsed_file.path.as_path(),
                    &mut self.type_resolver,
                )?;

                // Extract channels for each command
                for command in &mut file_commands {
                    if let Some(func) = self.find_function_in_ast(&parsed_file.ast, &command.name) {
                        let channels = self.channel_parser.extract_channels_from_command(
                            func,
                            &command.name,
                            parsed_file.path.as_path(),
                            &mut self.type_resolver,
                        )?;

                        // Collect type names from channel message types
                        channels.iter().for_each(|ch| {
                            self.extract_type_names(&ch.message_type, &mut type_names_to_discover);
                        });

                        command.channels = channels;
                    }
                }

                // Extract events from this file's AST
                let file_events = self.event_parser.extract_events_from_ast(
                    &parsed_file.ast,
                    parsed_file.path.as_path(),
                    &mut self.type_resolver,
                )?;

                // Collect type names from command parameters and return types using functional style
                file_commands.iter().for_each(|cmd| {
                    cmd.parameters.iter().for_each(|param| {
                        self.extract_type_names(&param.rust_type, &mut type_names_to_discover);
                    });
                    // Use the Rust return type (not TypeScript) to properly extract nested type names
                    self.extract_type_names(&cmd.return_type, &mut type_names_to_discover);
                });

                // Collect type names from event payloads
                file_events.iter().for_each(|event| {
                    self.extract_type_names(&event.payload_type, &mut type_names_to_discover);
                });

                commands.extend(file_commands);
                self.discovered_events.extend(file_events);

                // Build type definition index from this file
                self.index_type_definitions(&parsed_file.ast, parsed_file.path.as_path());
            }
        }

        if verbose {
            println!("🔍 Type names to discover: {:?}", type_names_to_discover);
        }

        // Lazy type resolution: Resolve types on demand using dependency graph
        self.resolve_types_lazily(&type_names_to_discover)?;

        if verbose {
            println!(
                "🏗️  Discovered {} structs total",
                self.discovered_structs.len()
            );
            for (name, info) in &self.discovered_structs {
                println!("  - {}: {} fields", name, info.fields.len());
            }
            println!(
                "📡 Discovered {} events total",
                self.discovered_events.len()
            );
            for event in &self.discovered_events {
                println!("  - '{}': {}", event.event_name, event.payload_type);
            }
            let all_channels = self.get_all_discovered_channels(&commands);
            println!("📞 Discovered {} channels total", all_channels.len());
            for channel in &all_channels {
                println!(
                    "  - '{}' in {}: {}",
                    channel.parameter_name, channel.command_name, channel.message_type
                );
            }
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
                // Extract commands and events from the cached AST
                if let Some(parsed_file) = self.ast_cache.get_cloned(&path_buf) {
                    // Extract events
                    let file_events = self.event_parser.extract_events_from_ast(
                        &parsed_file.ast,
                        path_buf.as_path(),
                        &mut self.type_resolver,
                    )?;
                    self.discovered_events.extend(file_events);

                    // Extract commands
                    let mut commands = self.command_parser.extract_commands_from_ast(
                        &parsed_file.ast,
                        path_buf.as_path(),
                        &mut self.type_resolver,
                    )?;

                    // Extract channels for each command
                    for command in &mut commands {
                        if let Some(func) =
                            self.find_function_in_ast(&parsed_file.ast, &command.name)
                        {
                            let channels = self.channel_parser.extract_channels_from_command(
                                func,
                                &command.name,
                                path_buf.as_path(),
                                &mut self.type_resolver,
                            )?;

                            command.channels = channels;
                        }
                    }

                    Ok(commands)
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
                    if let Some(struct_info) = self.extract_type_from_ast(
                        &parsed_file.ast,
                        &type_name,
                        file_path.as_path(),
                    ) {
                        // Collect dependencies of this type
                        let mut type_dependencies = HashSet::new();
                        for field in &struct_info.fields {
                            self.extract_type_names(&field.rust_type, &mut type_dependencies);
                        }

                        // Add dependencies to the resolution queue
                        for dep_type in &type_dependencies {
                            if !resolved_types.contains(dep_type)
                                && !self.discovered_structs.contains_key(dep_type)
                                && self.dependency_graph.has_type_definition(dep_type)
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
                        return self.struct_parser.parse_struct(
                            item_struct,
                            file_path,
                            &mut self.type_resolver,
                        );
                    }
                }
                syn::Item::Enum(item_enum) => {
                    if item_enum.ident == type_name
                        && self.struct_parser.should_include_enum(item_enum)
                    {
                        return self.struct_parser.parse_enum(item_enum, file_path);
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
            && !self.type_resolver.get_type_set().contains(rust_type)
            && !rust_type.starts_with(char::is_lowercase) // Skip built-in types
            && rust_type.chars().next().is_some_and(char::is_alphabetic)
            && !rust_type.contains('<')
        // Skip generic type names with parameters
        {
            type_names.insert(rust_type.to_string());
        }
    }

    /// Get discovered structs
    pub fn get_discovered_structs(&self) -> &HashMap<String, StructInfo> {
        &self.discovered_structs
    }

    /// Get discovered events
    pub fn get_discovered_events(&self) -> &[EventInfo] {
        &self.discovered_events
    }

    /// Get reference to the type resolver
    pub fn get_type_resolver(&self) -> std::cell::RefCell<&TypeResolver> {
        std::cell::RefCell::new(&self.type_resolver)
    }

    /// Get all discovered channels from all commands
    pub fn get_all_discovered_channels(&self, commands: &[CommandInfo]) -> Vec<ChannelInfo> {
        commands
            .iter()
            .flat_map(|cmd| cmd.channels.clone())
            .collect()
    }

    /// Find a function by name in an AST
    fn find_function_in_ast<'a>(
        &self,
        ast: &'a syn::File,
        function_name: &str,
    ) -> Option<&'a syn::ItemFn> {
        for item in &ast.items {
            if let syn::Item::Fn(func) = item {
                if func.sig.ident == function_name {
                    return Some(func);
                }
            }
        }
        None
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
}

impl Default for CommandAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn analyzer() -> CommandAnalyzer {
        CommandAnalyzer::new()
    }

    mod initialization {
        use super::*;

        #[test]
        fn test_new_creates_analyzer() {
            let analyzer = CommandAnalyzer::new();
            assert!(analyzer.get_discovered_structs().is_empty());
            assert!(analyzer.get_discovered_events().is_empty());
        }

        #[test]
        fn test_default_creates_analyzer() {
            let analyzer = CommandAnalyzer::default();
            assert!(analyzer.get_discovered_structs().is_empty());
            assert!(analyzer.get_discovered_events().is_empty());
        }
    }

    mod type_name_extraction {
        use super::*;

        #[test]
        fn test_extract_simple_type() {
            let analyzer = analyzer();
            let mut types = HashSet::new();
            analyzer.extract_type_names("User", &mut types);
            assert_eq!(types.len(), 1);
            assert!(types.contains("User"));
        }

        #[test]
        fn test_extract_option_type() {
            let analyzer = analyzer();
            let mut types = HashSet::new();
            analyzer.extract_type_names("Option<User>", &mut types);
            assert_eq!(types.len(), 1);
            assert!(types.contains("User"));
        }

        #[test]
        fn test_extract_vec_type() {
            let analyzer = analyzer();
            let mut types = HashSet::new();
            analyzer.extract_type_names("Vec<Product>", &mut types);
            assert_eq!(types.len(), 1);
            assert!(types.contains("Product"));
        }

        #[test]
        fn test_extract_result_type() {
            let analyzer = analyzer();
            let mut types = HashSet::new();
            analyzer.extract_type_names("Result<User, AppError>", &mut types);
            assert_eq!(types.len(), 2);
            assert!(types.contains("User"));
            assert!(types.contains("AppError"));
        }

        #[test]
        fn test_extract_hashmap_type() {
            let analyzer = analyzer();
            let mut types = HashSet::new();
            analyzer.extract_type_names("HashMap<String, User>", &mut types);
            // String is a primitive, should only extract User
            assert_eq!(types.len(), 1);
            assert!(types.contains("User"));
        }

        #[test]
        fn test_extract_btreemap_type() {
            let analyzer = analyzer();
            let mut types = HashSet::new();
            analyzer.extract_type_names("BTreeMap<UserId, Profile>", &mut types);
            assert_eq!(types.len(), 2);
            assert!(types.contains("UserId"));
            assert!(types.contains("Profile"));
        }

        #[test]
        fn test_extract_hashset_type() {
            let analyzer = analyzer();
            let mut types = HashSet::new();
            analyzer.extract_type_names("HashSet<User>", &mut types);
            assert_eq!(types.len(), 1);
            assert!(types.contains("User"));
        }

        #[test]
        fn test_extract_btreeset_type() {
            let analyzer = analyzer();
            let mut types = HashSet::new();
            analyzer.extract_type_names("BTreeSet<Tag>", &mut types);
            assert_eq!(types.len(), 1);
            assert!(types.contains("Tag"));
        }

        #[test]
        fn test_extract_tuple_type() {
            let analyzer = analyzer();
            let mut types = HashSet::new();
            analyzer.extract_type_names("(User, Product, Order)", &mut types);
            assert_eq!(types.len(), 3);
            assert!(types.contains("User"));
            assert!(types.contains("Product"));
            assert!(types.contains("Order"));
        }

        #[test]
        fn test_extract_reference_type() {
            let analyzer = analyzer();
            let mut types = HashSet::new();
            analyzer.extract_type_names("&User", &mut types);
            assert_eq!(types.len(), 1);
            assert!(types.contains("User"));
        }

        #[test]
        fn test_extract_nested_types() {
            let analyzer = analyzer();
            let mut types = HashSet::new();
            analyzer.extract_type_names("Vec<Option<User>>", &mut types);
            assert_eq!(types.len(), 1);
            assert!(types.contains("User"));
        }

        #[test]
        fn test_extract_deeply_nested_types() {
            let analyzer = analyzer();
            let mut types = HashSet::new();
            analyzer.extract_type_names("HashMap<String, Vec<Option<Product>>>", &mut types);
            assert_eq!(types.len(), 1);
            assert!(types.contains("Product"));
        }

        #[test]
        fn test_skips_primitive_types() {
            let analyzer = analyzer();
            let mut types = HashSet::new();
            analyzer.extract_type_names("String", &mut types);
            assert_eq!(types.len(), 0);
        }

        #[test]
        fn test_skips_built_in_types() {
            let analyzer = analyzer();
            let mut types = HashSet::new();
            analyzer.extract_type_names("i32", &mut types);
            assert_eq!(types.len(), 0);
        }

        #[test]
        fn test_skips_empty_type() {
            let analyzer = analyzer();
            let mut types = HashSet::new();
            analyzer.extract_type_names("", &mut types);
            assert_eq!(types.len(), 0);
        }

        #[test]
        fn test_skips_unit_type() {
            let analyzer = analyzer();
            let mut types = HashSet::new();
            analyzer.extract_type_names("()", &mut types);
            assert_eq!(types.len(), 0);
        }

        #[test]
        fn test_multiple_calls_accumulate() {
            let analyzer = analyzer();
            let mut types = HashSet::new();
            analyzer.extract_type_names("User", &mut types);
            analyzer.extract_type_names("Product", &mut types);
            assert_eq!(types.len(), 2);
            assert!(types.contains("User"));
            assert!(types.contains("Product"));
        }

        #[test]
        fn test_duplicate_types_deduped() {
            let analyzer = analyzer();
            let mut types = HashSet::new();
            analyzer.extract_type_names("User", &mut types);
            analyzer.extract_type_names("User", &mut types);
            assert_eq!(types.len(), 1);
        }
    }

    mod getters {
        use super::*;

        #[test]
        fn test_get_discovered_structs_empty() {
            let analyzer = analyzer();
            let structs = analyzer.get_discovered_structs();
            assert!(structs.is_empty());
        }

        #[test]
        fn test_get_discovered_events_empty() {
            let analyzer = analyzer();
            let events = analyzer.get_discovered_events();
            assert!(events.is_empty());
        }

        #[test]
        fn test_get_type_resolver() {
            let analyzer = analyzer();
            let resolver = analyzer.get_type_resolver();
            // Just verify it returns a RefCell
            assert!(!resolver.borrow().get_type_set().is_empty());
        }

        #[test]
        fn test_get_dependency_graph() {
            let analyzer = analyzer();
            let graph = analyzer.get_dependency_graph();
            // Verify graph exists (check resolved types)
            assert!(graph.get_resolved_types().is_empty());
        }

        #[test]
        fn test_get_all_discovered_channels_empty() {
            let analyzer = analyzer();
            let commands = vec![];
            let channels = analyzer.get_all_discovered_channels(&commands);
            assert!(channels.is_empty());
        }

        #[test]
        fn test_get_all_discovered_channels_with_commands() {
            let analyzer = analyzer();
            let command = CommandInfo::new_for_test(
                "test_cmd",
                "test.rs",
                1,
                vec![],
                "void",
                false,
                vec![
                    ChannelInfo::new_for_test("ch1", "Message1", "test_cmd", "test.rs", 10),
                    ChannelInfo::new_for_test("ch2", "Message2", "test_cmd", "test.rs", 20),
                ],
            );

            let commands = vec![command];
            let channels = analyzer.get_all_discovered_channels(&commands);
            assert_eq!(channels.len(), 2);
        }
    }

    mod topological_sort {
        use super::*;

        #[test]
        fn test_topological_sort_empty() {
            let analyzer = analyzer();
            let types = HashSet::new();
            let sorted = analyzer.topological_sort_types(&types);
            assert!(sorted.is_empty());
        }

        #[test]
        fn test_topological_sort_single_type() {
            let mut analyzer = analyzer();
            let path = PathBuf::from("test.rs");
            analyzer
                .dependency_graph
                .add_type_definition("User".to_string(), path);

            let mut types = HashSet::new();
            types.insert("User".to_string());

            let sorted = analyzer.topological_sort_types(&types);
            assert_eq!(sorted.len(), 1);
            assert_eq!(sorted[0], "User");
        }
    }

    mod ast_helpers {
        use super::*;
        use syn::{parse_quote, File as SynFile};

        #[test]
        fn test_find_function_in_ast() {
            let analyzer = analyzer();
            let ast: SynFile = parse_quote! {
                #[tauri::command]
                fn my_command() -> String {
                    "test".to_string()
                }

                fn other_function() {}
            };

            let result = analyzer.find_function_in_ast(&ast, "my_command");
            assert!(result.is_some());
            assert_eq!(result.unwrap().sig.ident, "my_command");
        }

        #[test]
        fn test_find_function_in_ast_not_found() {
            let analyzer = analyzer();
            let ast: SynFile = parse_quote! {
                fn my_command() {}
            };

            let result = analyzer.find_function_in_ast(&ast, "non_existent");
            assert!(result.is_none());
        }

        #[test]
        fn test_find_function_in_ast_empty() {
            let analyzer = analyzer();
            let ast: SynFile = parse_quote! {};

            let result = analyzer.find_function_in_ast(&ast, "any_function");
            assert!(result.is_none());
        }
    }

    mod index_type_definitions {
        use super::*;
        use syn::{parse_quote, File as SynFile};

        #[test]
        fn test_index_struct() {
            let mut analyzer = analyzer();
            let ast: SynFile = parse_quote! {
                #[derive(Serialize)]
                pub struct User {
                    name: String,
                }
            };
            let path = Path::new("test.rs");

            analyzer.index_type_definitions(&ast, path);

            assert!(analyzer.dependency_graph.has_type_definition("User"));
        }

        #[test]
        fn test_index_enum() {
            let mut analyzer = analyzer();
            let ast: SynFile = parse_quote! {
                #[derive(Serialize)]
                pub enum Status {
                    Active,
                    Inactive,
                }
            };
            let path = Path::new("test.rs");

            analyzer.index_type_definitions(&ast, path);

            assert!(analyzer.dependency_graph.has_type_definition("Status"));
        }

        #[test]
        fn test_skips_non_serde_types() {
            let mut analyzer = analyzer();
            let ast: SynFile = parse_quote! {
                #[derive(Debug, Clone)]
                pub struct User {
                    name: String,
                }
            };
            let path = Path::new("test.rs");

            analyzer.index_type_definitions(&ast, path);

            assert!(!analyzer.dependency_graph.has_type_definition("User"));
        }
    }

    mod extract_type_from_ast {
        use super::*;
        use syn::{parse_quote, File as SynFile};

        #[test]
        fn test_extract_struct_from_ast() {
            let mut analyzer = analyzer();
            let ast: SynFile = parse_quote! {
                #[derive(Serialize)]
                pub struct User {
                    pub name: String,
                }
            };
            let path = Path::new("test.rs");

            let result = analyzer.extract_type_from_ast(&ast, "User", path);
            assert!(result.is_some());
            let struct_info = result.unwrap();
            assert_eq!(struct_info.name, "User");
            assert_eq!(struct_info.fields.len(), 1);
        }

        #[test]
        fn test_extract_enum_from_ast() {
            let mut analyzer = analyzer();
            let ast: SynFile = parse_quote! {
                #[derive(Serialize)]
                pub enum Status {
                    Active,
                    Inactive,
                }
            };
            let path = Path::new("test.rs");

            let result = analyzer.extract_type_from_ast(&ast, "Status", path);
            assert!(result.is_some());
            let enum_info = result.unwrap();
            assert_eq!(enum_info.name, "Status");
            assert!(enum_info.is_enum);
        }

        #[test]
        fn test_extract_type_not_found() {
            let mut analyzer = analyzer();
            let ast: SynFile = parse_quote! {
                #[derive(Serialize)]
                pub struct User {
                    name: String,
                }
            };
            let path = Path::new("test.rs");

            let result = analyzer.extract_type_from_ast(&ast, "Product", path);
            assert!(result.is_none());
        }

        #[test]
        fn test_extract_type_without_serde() {
            let mut analyzer = analyzer();
            let ast: SynFile = parse_quote! {
                #[derive(Debug)]
                pub struct User {
                    name: String,
                }
            };
            let path = Path::new("test.rs");

            let result = analyzer.extract_type_from_ast(&ast, "User", path);
            assert!(result.is_none());
        }
    }

    mod visualization {
        use super::*;

        #[test]
        fn test_visualize_dependencies() {
            let analyzer = analyzer();
            let commands = vec![];
            let viz = analyzer.visualize_dependencies(&commands);
            // Just verify it returns a string
            assert!(viz.contains("Dependency Graph"));
        }

        #[test]
        fn test_generate_dot_graph() {
            let analyzer = analyzer();
            let commands = vec![];
            let dot = analyzer.generate_dot_graph(&commands);
            // Verify basic DOT format
            assert!(dot.contains("digraph"));
        }
    }
}
