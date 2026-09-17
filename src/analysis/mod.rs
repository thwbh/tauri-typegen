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
use std::env;
use std::fs;
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
    /// Type names referenced by commands/events but not resolved in-project or
    /// via the external-crate lookup. Populated during `resolve_types_lazily`;
    /// surfaced as warnings so silent false negatives (#77/#84) become
    /// observable instead of producing quietly-incomplete bindings.
    unresolved_types: Vec<String>,
    /// Per-`analyze_project*` memo of external-crate type lookups: maps a type
    /// name to the file that declares it (`Some`) or to a recorded negative
    /// result (`None`). Built lazily by `find_external_type_path` so each type
    /// is walked at most once per analysis pass instead of on every reference
    /// (#87). Reset implicitly because `CommandAnalyzer` is constructed fresh
    /// per `analyze_project*` call.
    external_type_lookup_cache: HashMap<String, Option<PathBuf>>,
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
            unresolved_types: Vec::new(),
            external_type_lookup_cache: HashMap::new(),
        }
    }

    /// Add custom type mappings from configuration
    pub fn add_type_mappings(&mut self, mappings: &HashMap<String, String>) {
        for (rust_type, ts_type) in mappings {
            self.type_resolver
                .add_type_mapping(rust_type.clone(), ts_type.clone());
        }
    }

    /// Type names referenced by commands/events that could not be resolved
    /// in-project or via the external-crate lookup during the last
    /// `analyze_project*` call. Empty when everything resolved (or when nothing
    /// was analyzed yet).
    ///
    /// These are the names whose absence used to be silently dropped (#77/#84);
    /// callers can log them, fail the build, or ignore them. The analyzer itself
    /// only warns and continues, to preserve backward-compatible output.
    pub fn unresolved_types(&self) -> &[String] {
        &self.unresolved_types
    }

    /// Seed the external-crate lookup memo from a previous run's persisted
    /// index (loaded from `.typecache`). Seeded entries short-circuit the
    /// registry walk for types already resolved last time (#87). The cache is
    /// still filled for any type not present in the seed, so a partial/empty
    /// seed is safe.
    pub fn seed_external_type_cache(&mut self, index: HashMap<String, Option<PathBuf>>) {
        self.external_type_lookup_cache = index;
    }

    /// The current external-crate lookup memo (positive and negative results
    /// accumulated during this analysis pass). Persisted into `.typecache` for
    /// the next run via `GenerationCache::new_with_external_index` (#87).
    pub fn external_type_lookup_cache(&self) -> &HashMap<String, Option<PathBuf>> {
        &self.external_type_lookup_cache
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
        let mut file_paths: Vec<PathBuf> = self.ast_cache.keys().cloned().collect();
        file_paths.sort_unstable();
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

        // Surface unresolved referenced types as a warning on stderr so they
        // are visible to CLI and build.rs users regardless of --verbose. These
        // were previously silently dropped, producing quietly-incomplete
        // bindings (see #77/#84). Non-fatal: generation continues.
        if !self.unresolved_types.is_empty() {
            eprintln!(
                "⚠️  tauri-typegen: {} referenced type(s) could not be resolved in the \
                 project or the Cargo registry; their bindings will be missing from the output:",
                self.unresolved_types.len()
            );
            for name in &self.unresolved_types {
                eprintln!("    - {}", name);
            }
            eprintln!(
                "    This is usually caused by a missing/empty/relocated Cargo registry \
                 (CARGO_HOME) or a vendored dependency layout. See issue #84."
            );
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
        self.index_items(&ast.items, file_path);
    }

    /// Recursively index items for type definitions
    fn index_items(&mut self, items: &[syn::Item], file_path: &Path) {
        for item in items {
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
                syn::Item::Mod(item_mod) => {
                    if let Some((_, items)) = &item_mod.content {
                        self.index_items(items, file_path);
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
        // Names that were requested but could not be resolved in-project or via
        // the external-crate lookup. Surfaced as warnings after the pass so the
        // silent false negatives reported in #77/#84 become observable.
        let mut unresolved_types: Vec<String> = Vec::new();

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
                .or_else(|| {
                    // External crate lookup (memoized per pass — #87)
                    if let Some(ext_path) = self.find_external_type_path_cached(&type_name) {
                        // Cache the discovery for future look‑ups
                        self.dependency_graph
                            .add_type_definition(type_name.clone(), ext_path.clone());
                        let _ = self.ast_cache.parse_and_cache_file(&ext_path);
                        Some(ext_path)
                    } else {
                        None
                    }
                })
            {
                if let Some(parsed_file) = self.ast_cache.get_cloned(&file_path) {
                    self.index_items(&parsed_file.ast.items, &file_path);
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

                        // Collect dependencies from enum variants
                        if let Some(variants) = &struct_info.enum_variants {
                            for variant in variants {
                                match &variant.kind {
                                    crate::models::EnumVariantKind::Unit => {}
                                    crate::models::EnumVariantKind::Tuple(types) => {
                                        for type_struct in types {
                                            let mut variant_types = HashSet::new();
                                            crate::generators::TypeCollector::collect_referenced_types_from_structure(
                                                type_struct,
                                                &mut variant_types,
                                            );
                                            type_dependencies.extend(variant_types);
                                        }
                                    }
                                    crate::models::EnumVariantKind::Struct(fields) => {
                                        for field in fields {
                                            self.extract_type_names(
                                                &field.rust_type,
                                                &mut type_dependencies,
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        // Add dependencies to the resolution queue
                        for dep_type in &type_dependencies {
                            if !resolved_types.contains(dep_type)
                                && !self.discovered_structs.contains_key(dep_type)
                                && (self.dependency_graph.has_type_definition(dep_type)
                                    || self.find_external_type_path_cached(dep_type).is_some())
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
            } else {
                // No definition path in-project and the external-crate lookup
                // came up empty (e.g. an empty/missing/relocated Cargo registry
                // — see #84). Record the name so it can be reported rather than
                // silently dropped.
                unresolved_types.push(type_name);
            }
        }

        // De-duplicate in stable (first-seen) order for a tidy warning.
        let mut seen: HashSet<String> = HashSet::new();
        unresolved_types.retain(|name| seen.insert(name.clone()));
        self.unresolved_types = unresolved_types;

        Ok(())
    }

    // Find type paths from external crates.
    //
    // Walks the Cargo registry source tree and, for each `.rs` file, looks for a
    // `struct` or `enum` item whose identifier exactly matches `type_name`. The
    // match is performed on the parsed AST (via `syn`) rather than with a raw
    // substring search, so it correctly handles visibility modifiers
    // (`pub`, `pub(crate)`), attributes (`#[derive(...)]`), generics
    // (`struct X<T>`), and multi-line declarations — all of which the previous
    // `content.contains("struct X")` heuristic missed or matched incorrectly
    // (see issue #82). A cheap `contains` pre-filter keeps the registry walk fast
    // by skipping files that cannot possibly declare the type.
    /// Like `find_external_type_path`, but memoizes the result (positive *or*
    /// negative) in `external_type_lookup_cache` so repeated lookups for the
    /// same name within one analysis pass are O(1) instead of re-walking the
    /// registry (#87). This is the variant the resolver uses.
    fn find_external_type_path_cached(&mut self, type_name: &str) -> Option<PathBuf> {
        if let Some(cached) = self.external_type_lookup_cache.get(type_name) {
            return cached.clone();
        }
        let found = self.find_external_type_path_uncached(type_name);
        self.external_type_lookup_cache
            .insert(type_name.to_string(), found.clone());
        found
    }

    // Walk the Cargo registry source tree looking for a `struct`/`enum` item
    // whose identifier exactly matches `type_name`.
    //
    // The match is performed on the parsed AST (via `syn`) rather than a raw
    // substring search, so it correctly handles visibility modifiers
    // (`pub`, `pub(crate)`), attributes (`#[derive(...)]`), generics
    // (`struct X<T>`), and multi-line declarations — all of which a substring
    // heuristic misses or matches incorrectly (see #82). A cheap `contains`
    // pre-filter keeps the walk fast by skipping files that cannot possibly
    // declare the type. This is the uncached primitive; callers that want
    // per-pass memoization should use `find_external_type_path_cached`.
    fn find_external_type_path_uncached(&self, type_name: &str) -> Option<PathBuf> {
        // Resolve Cargo home. `CARGO_HOME` wins; otherwise fall back to
        // `$HOME/.cargo` using path joins (not string formatting) so the
        // fallback is correct on Windows too (#84).
        let cargo_home: PathBuf = match env::var("CARGO_HOME") {
            Ok(dir) => PathBuf::from(dir),
            Err(_) => {
                let home: String = env::var("HOME").or(env::var("USERPROFILE")).ok()?;
                PathBuf::from(home).join(".cargo")
            }
        };
        let src_dir: PathBuf = cargo_home.join("registry/src");

        // A cheap substring pre-filter: only the identifier, without the
        // `struct`/`enum` keyword, so that `pub struct X`, `pub(crate) enum X`,
        // and `struct\n  X` all pass through to the AST check. This avoids parsing
        // the vast majority of registry files that cannot contain the type.
        let needle: String = type_name.to_string();

        // Walk the registry tree depth‑first.
        let mut dirs: Vec<PathBuf> = vec![src_dir];
        while let Some(dir) = dirs.pop() {
            let entries: fs::ReadDir = fs::read_dir(&dir).ok()?;
            for entry in entries.filter_map(Result::ok) {
                let path: PathBuf = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                    continue;
                }
                if path.extension().and_then(|s| s.to_str()) != Some("rs") {
                    continue;
                }
                // Cheap pre-filter on the raw source before paying for a full parse.
                let content: String = match fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                if !content.contains(&needle) {
                    continue;
                }
                // Authoritative check on the parsed AST.
                if Self::file_declares_type(&content, type_name) {
                    return Some(path);
                }
            }
        }
        None
    }

    /// Returns `true` if `source` declares a `struct` or `enum` item whose
    /// identifier equals `type_name`. Items nested inside `mod` blocks are
    /// considered as well, mirroring how the analyzer indexes local modules.
    /// Files that fail to parse (e.g. macro-heavy or generated sources) yield
    /// `false` so they are simply skipped during the registry walk.
    fn file_declares_type(source: &str, type_name: &str) -> bool {
        let file: syn::File = match syn::parse_file(source) {
            Ok(f) => f,
            Err(_) => return false,
        };
        Self::items_declare_type(&file.items, type_name)
    }

    fn items_declare_type(items: &[syn::Item], type_name: &str) -> bool {
        for item in items {
            let declares = match item {
                syn::Item::Struct(s) => s.ident == type_name,
                syn::Item::Enum(e) => e.ident == type_name,
                // Recurse into inline modules so types declared in `mod x { ... }`
                // blocks within a single file are still discovered.
                syn::Item::Mod(m) => match &m.content {
                    Some((_, inner)) => Self::items_declare_type(inner, type_name),
                    None => false,
                },
                _ => false,
            };
            if declares {
                return true;
            }
        }
        false
    }

    /// Extract a specific type from a cached AST
    fn extract_type_from_ast(
        &mut self,
        ast: &syn::File,
        type_name: &str,
        file_path: &Path,
    ) -> Option<StructInfo> {
        self.find_type_in_items(&ast.items, type_name, file_path)
    }

    /// Recursively find a type in a list of items
    fn find_type_in_items(
        &mut self,
        items: &[syn::Item],
        type_name: &str,
        file_path: &Path,
    ) -> Option<StructInfo> {
        for item in items {
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
                        return self.struct_parser.parse_enum(
                            item_enum,
                            file_path,
                            &mut self.type_resolver,
                        );
                    }
                }
                syn::Item::Mod(item_mod) => {
                    if let Some((_, items)) = &item_mod.content {
                        if let Some(info) = self.find_type_in_items(items, type_name, file_path) {
                            return Some(info);
                        }
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

        // Handle references first
        if rust_type.starts_with('&') {
            let without_ref = rust_type.trim_start_matches('&');
            self.extract_type_names_recursive(without_ref, type_names);
            return;
        }

        // Strip module prefixes like std::, ::std::, ::core::, etc. for generic type detection
        // but keep the original for custom type name detection
        let stripped = Self::strip_module_prefix(rust_type);

        // Handle Result<T, E> - extract both T and E
        if stripped.starts_with("Result<") {
            if let Some(inner) = stripped
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

        // Handle Option<T> - extract T (handles both Option<T> and ::core::option::Option<T>)
        if stripped.starts_with("Option<") {
            if let Some(inner) = stripped
                .strip_prefix("Option<")
                .and_then(|s| s.strip_suffix(">"))
            {
                self.extract_type_names_recursive(inner, type_names);
            }
            return;
        }

        // Handle Vec<T> - extract T (handles both Vec<T> and ::std::vec::Vec<T>)
        if stripped.starts_with("Vec<") {
            if let Some(inner) = stripped
                .strip_prefix("Vec<")
                .and_then(|s| s.strip_suffix(">"))
            {
                self.extract_type_names_recursive(inner, type_names);
            }
            return;
        }

        // Handle HashMap<K, V> and BTreeMap<K, V> - extract K and V
        if stripped.starts_with("HashMap<") || stripped.starts_with("BTreeMap<") {
            let prefix = if stripped.starts_with("HashMap<") {
                "HashMap<"
            } else {
                "BTreeMap<"
            };
            if let Some(inner) = stripped
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
        if stripped.starts_with("HashSet<") || stripped.starts_with("BTreeSet<") {
            let prefix = if stripped.starts_with("HashSet<") {
                "HashSet<"
            } else {
                "BTreeSet<"
            };
            if let Some(inner) = stripped
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

        // Check if this is a custom type name
        if !rust_type.is_empty()
            && !self.type_resolver.get_type_set().contains(rust_type)
            && !rust_type.starts_with(char::is_lowercase) // Skip built-in types
            && rust_type.chars().next().is_some_and(char::is_alphabetic)
            && !rust_type.contains('<')
        // Skip generic type names with parameters
        {
            // Extract just the type name, stripping module prefix if present
            let type_name = Self::extract_simple_type_name(rust_type);
            type_names.insert(type_name);
        }
    }

    /// Strip module prefixes like std::, ::std::, ::core::, crate::, etc.
    /// Used for pattern matching on generic types
    fn strip_module_prefix(rust_type: &str) -> &str {
        // Find the last :: to separate module path from type name
        if let Some(last_double_colon) = rust_type.rfind("::") {
            // Only strip if what follows contains < (it's a generic type)
            let after_colon = &rust_type[last_double_colon + 2..];
            if after_colon.contains('<') {
                return after_colon;
            }
        }
        rust_type
    }

    /// Extract just the type name from a potentially module-qualified name
    /// E.g., "::my_module::MyType" -> "MyType"
    fn extract_simple_type_name(rust_type: &str) -> String {
        // Take everything after the last ::, or the whole thing if no ::
        if let Some(last_double_colon) = rust_type.rfind("::") {
            rust_type[last_double_colon + 2..].to_string()
        } else {
            rust_type.to_string()
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

    /// Find a function by name in an AST (recursive)
    fn find_function_in_ast<'a>(
        &self,
        ast: &'a syn::File,
        function_name: &str,
    ) -> Option<&'a syn::ItemFn> {
        self.find_function_in_items(&ast.items, function_name)
    }

    /// Recursively find a function in a list of items
    fn find_function_in_items<'a>(
        &self,
        items: &'a [syn::Item],
        function_name: &str,
    ) -> Option<&'a syn::ItemFn> {
        for item in items {
            match item {
                syn::Item::Fn(func) => {
                    if func.sig.ident == function_name {
                        return Some(func);
                    }
                }
                syn::Item::Mod(item_mod) => {
                    if let Some((_, items)) = &item_mod.content {
                        if let Some(func) = self.find_function_in_items(items, function_name) {
                            return Some(func);
                        }
                    }
                }
                _ => {}
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

    mod external_type_lookup {
        use super::*;
        use serial_test::serial;
        use std::env;
        use std::fs;
        use std::path::PathBuf;

        /// Build a fake Cargo registry rooted at a temp dir, write the given
        /// source files into `registry/src/dummy-0.1.0/`, point `CARGO_HOME` at
        /// it, and return the analyzer + the path that a declaration in
        /// `lib.rs` should resolve to.
        struct FakeRegistry {
            _root: PathBuf,
            cargo_home: PathBuf,
        }

        impl FakeRegistry {
            fn new(files: &[(&str, &str)]) -> (Self, CommandAnalyzer) {
                let root: PathBuf = std::env::temp_dir().join(format!(
                    "tauri_typegen_external_test_{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos(),
                ));
                let _ = std::fs::remove_dir_all(&root);
                let cargo_home: PathBuf = root.join(".cargo");
                let src_dir: PathBuf = cargo_home.join("registry/src");
                let crate_dir: PathBuf = src_dir.join("dummy-0.1.0");
                fs::create_dir_all(&crate_dir).expect("create temp crate dir");
                for (name, content) in files {
                    fs::write(crate_dir.join(name), content).expect("write file");
                }
                env::set_var("CARGO_HOME", &cargo_home);
                let fake = FakeRegistry {
                    _root: root.clone(),
                    cargo_home,
                };
                (fake, CommandAnalyzer::default())
            }
        }

        impl Drop for FakeRegistry {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self._root);
            }
        }

        #[test]
        #[serial]
        fn test_find_external_type_path_pub_struct() {
            let (reg, mut analyzer) = FakeRegistry::new(&[("lib.rs", "pub struct ExternalFoo;")]);
            let found = analyzer.find_external_type_path_cached("ExternalFoo");
            let expected = reg.cargo_home.join("registry/src/dummy-0.1.0/lib.rs");
            assert_eq!(found.unwrap(), expected, "pub struct should be located");
        }

        /// Regression for issue #82: visibility modifiers such as `pub(crate)`
        /// must not prevent discovery.
        #[test]
        #[serial]
        fn test_find_external_type_path_pub_crate_visibility() {
            let (reg, mut analyzer) =
                FakeRegistry::new(&[("lib.rs", "pub(crate) struct VisCrate;")]);
            let found = analyzer.find_external_type_path_cached("VisCrate");
            let expected = reg.cargo_home.join("registry/src/dummy-0.1.0/lib.rs");
            assert_eq!(
                found.unwrap(),
                expected,
                "pub(crate) struct should be located"
            );
        }

        /// Regression for issue #82: derive attributes preceding the
        /// declaration must not prevent discovery.
        #[test]
        #[serial]
        fn test_find_external_type_path_with_derive_attributes() {
            let (reg, mut analyzer) = FakeRegistry::new(&[(
                "lib.rs",
                "#[derive(Debug, Clone)]\npub struct WithDerives { field: i32 }",
            )]);
            let found = analyzer.find_external_type_path_cached("WithDerives");
            let expected = reg.cargo_home.join("registry/src/dummy-0.1.0/lib.rs");
            assert_eq!(
                found.unwrap(),
                expected,
                "#[derive(...)] pub struct should be located"
            );
        }

        /// Regression for issue #82: generic parameters must not prevent
        /// discovery.
        #[test]
        #[serial]
        fn test_find_external_type_path_with_generics() {
            let (reg, mut analyzer) =
                FakeRegistry::new(&[("lib.rs", "pub struct Generic<T, U> { a: T, b: U }")]);
            let found = analyzer.find_external_type_path_cached("Generic");
            let expected = reg.cargo_home.join("registry/src/dummy-0.1.0/lib.rs");
            assert_eq!(
                found.unwrap(),
                expected,
                "generic pub struct should be located"
            );
        }

        /// Multi-line declarations (keyword and identifier on different lines)
        /// must be discovered — the old `contains("struct X")` heuristic missed
        /// these.
        #[test]
        #[serial]
        fn test_find_external_type_path_multiline() {
            let (reg, mut analyzer) =
                FakeRegistry::new(&[("lib.rs", "pub\n  struct\n  Multiline\n{\n    x: i32,\n  }")]);
            let found = analyzer.find_external_type_path_cached("Multiline");
            let expected = reg.cargo_home.join("registry/src/dummy-0.1.0/lib.rs");
            assert_eq!(
                found.unwrap(),
                expected,
                "multi-line struct should be located"
            );
        }

        /// Enums with attributes and visibility must be discovered too.
        #[test]
        #[serial]
        fn test_find_external_type_path_enum_with_attributes() {
            let (reg, mut analyzer) =
                FakeRegistry::new(&[("lib.rs", "#[derive(Debug)]\npub enum EnumAttr { A, B }")]);
            let found = analyzer.find_external_type_path_cached("EnumAttr");
            let expected = reg.cargo_home.join("registry/src/dummy-0.1.0/lib.rs");
            assert_eq!(
                found.unwrap(),
                expected,
                "pub enum with derive should be located"
            );
        }

        /// Types declared inside an inline `mod` block should still be found.
        #[test]
        #[serial]
        fn test_find_external_type_path_nested_module() {
            let (reg, mut analyzer) =
                FakeRegistry::new(&[("lib.rs", "mod inner {\n  pub struct Nested;\n}\n")]);
            let found = analyzer.find_external_type_path_cached("Nested");
            let expected = reg.cargo_home.join("registry/src/dummy-0.1.0/lib.rs");
            assert_eq!(
                found.unwrap(),
                expected,
                "struct inside an inline mod should be located"
            );
        }

        /// The lookup must not return false positives: a struct whose name only
        /// *starts with* the searched identifier (e.g. `ExternalFooBar` when
        /// searching for `ExternalFoo`) must not match. The old substring
        /// heuristic would incorrectly match this.
        #[test]
        #[serial]
        fn test_find_external_type_path_no_false_positive_prefix() {
            let (_reg, mut analyzer) =
                FakeRegistry::new(&[("lib.rs", "pub struct ExternalFooBar;")]);
            let found = analyzer.find_external_type_path_cached("ExternalFoo");
            assert!(
                found.is_none(),
                "a prefix-named struct must not match the shorter identifier"
            );
        }

        /// An absent type must resolve to `None`.
        #[test]
        #[serial]
        fn test_find_external_type_path_missing() {
            let (_reg, mut analyzer) =
                FakeRegistry::new(&[("lib.rs", "pub struct SomethingElse;")]);
            let found = analyzer.find_external_type_path_cached("ExternalFoo");
            assert!(found.is_none(), "a missing type must resolve to None");
        }

        /// A file that fails to parse must be skipped, not panic.
        #[test]
        #[serial]
        fn test_find_external_type_path_skips_unparseable_file() {
            let (_reg, mut analyzer) =
                FakeRegistry::new(&[("lib.rs", "this is not valid rust !!!")]);
            let found = analyzer.find_external_type_path_cached("ExternalFoo");
            assert!(
                found.is_none(),
                "an unparseable file must be skipped without panicking"
            );
        }
    }

    mod unresolved_type_reporting {
        use super::*;
        use serial_test::serial;
        use std::env;
        use std::fs;
        use std::path::PathBuf;

        /// A freshly constructed analyzer reports no unresolved types.
        #[test]
        fn fresh_analyzer_reports_no_unresolved_types() {
            let analyzer = CommandAnalyzer::default();
            assert!(
                analyzer.unresolved_types().is_empty(),
                "a fresh analyzer must report no unresolved types",
            );
        }

        /// A referenced type that is absent from both the (empty) project and
        /// the (empty/fake) Cargo registry must be recorded as unresolved so the
        /// previously-silent false negative (#77/#84) becomes observable.
        #[test]
        #[serial]
        fn records_unresolved_type_when_registry_is_empty() {
            // Point CARGO_HOME at an empty dir so the registry walk finds nothing.
            let tmp_root: PathBuf = std::env::temp_dir().join(format!(
                "tauri_typegen_unresolved_empty_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
            ));
            let _ = std::fs::remove_dir_all(&tmp_root);
            let cargo_home: PathBuf = tmp_root.join(".cargo");
            fs::create_dir_all(&cargo_home).expect("create empty cargo home");
            env::set_var("CARGO_HOME", &cargo_home);

            let mut analyzer = CommandAnalyzer::default();
            let mut initial: HashSet<String> = HashSet::new();
            initial.insert("DefinitelyMissing".to_string());

            analyzer
                .resolve_types_lazily(&initial)
                .expect("resolve pass");

            let unresolved = analyzer.unresolved_types();
            assert!(
                unresolved.contains(&"DefinitelyMissing".to_string()),
                "an absent type must be recorded as unresolved, got: {:?}",
                unresolved,
            );

            let _ = std::fs::remove_dir_all(&tmp_root);
        }

        /// When the registry is missing entirely (no `registry/src` at all),
        /// the walk must not panic and the type must still be reported.
        #[test]
        #[serial]
        fn records_unresolved_type_when_registry_dir_absent() {
            let tmp_root: PathBuf = std::env::temp_dir().join(format!(
                "tauri_typegen_unresolved_absent_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
            ));
            let _ = std::fs::remove_dir_all(&tmp_root);
            // Create the cargo home but NOT registry/src inside it.
            fs::create_dir_all(&tmp_root).expect("create cargo home root");
            env::set_var("CARGO_HOME", &tmp_root);

            let mut analyzer = CommandAnalyzer::default();
            let mut initial: HashSet<String> = HashSet::new();
            initial.insert("NoRegistryHere".to_string());

            analyzer
                .resolve_types_lazily(&initial)
                .expect("resolve pass");

            assert!(
                analyzer
                    .unresolved_types()
                    .contains(&"NoRegistryHere".to_string()),
                "a missing registry dir must still yield an unresolved report",
            );

            let _ = std::fs::remove_dir_all(&tmp_root);
        }

        /// A type that IS resolvable in-project (present in the dependency
        /// graph with a real on-disk source file) must NOT appear in the
        /// unresolved list — guards against the warning firing for happy-path
        /// types.
        #[test]
        #[serial]
        fn resolved_type_is_not_reported_unresolved() {
            let tmp_root: PathBuf = std::env::temp_dir().join(format!(
                "tauri_typegen_unresolved_resolved_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
            ));
            let _ = std::fs::remove_dir_all(&tmp_root);
            let cargo_home: PathBuf = tmp_root.join(".cargo");
            fs::create_dir_all(&cargo_home).expect("create cargo home");
            env::set_var("CARGO_HOME", &cargo_home);

            // Real on-disk Rust source declaring the type. Must carry a
            // Serialize/Deserialize derive, otherwise should_include_struct
            // filters it out (project policy).
            let src_file: PathBuf = tmp_root.join("types.rs");
            fs::write(
                &src_file,
                "use serde::Serialize;\n#[derive(Serialize)]\npub struct ResolvedType { a: i32 }",
            )
            .expect("write source");

            let mut analyzer = CommandAnalyzer::default();
            // Parse + cache the file so extract_type_from_ast can find it.
            analyzer
                .ast_cache
                .parse_and_cache_file(&src_file)
                .expect("parse source file");
            // Tell the dependency graph where the type lives.
            analyzer
                .dependency_graph
                .add_type_definition("ResolvedType".to_string(), src_file.clone());

            let mut initial: HashSet<String> = HashSet::new();
            initial.insert("ResolvedType".to_string());

            analyzer
                .resolve_types_lazily(&initial)
                .expect("resolve pass");

            assert!(
                analyzer.discovered_structs.contains_key("ResolvedType"),
                "the in-project type should have been resolved",
            );
            assert!(
                !analyzer
                    .unresolved_types()
                    .contains(&"ResolvedType".to_string()),
                "a resolved in-project type must not be reported unresolved",
            );

            let _ = std::fs::remove_dir_all(&tmp_root);
        }

        /// The `CARGO_HOME` fallback must work when only `USERPROFILE` is set
        /// (Windows-style), and `HOME` is unset — regression for the
        /// `format!("{}/.cargo", h)` path that produced wrong paths on Windows.
        #[test]
        #[serial]
        fn cargo_home_fallback_uses_userprofile_when_home_absent() {
            let tmp_root: PathBuf = std::env::temp_dir().join(format!(
                "tauri_typegen_userprofile_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
            ));
            let _ = std::fs::remove_dir_all(&tmp_root);
            // Build the registry under a fake "user profile" home.
            let src_dir: PathBuf = tmp_root.join(".cargo").join("registry/src");
            let crate_dir: PathBuf = src_dir.join("windep-0.1.0");
            fs::create_dir_all(&crate_dir).expect("create registry crate dir");
            let file_path: PathBuf = crate_dir.join("lib.rs");
            fs::write(&file_path, "pub struct WinOnly;").expect("write registry file");

            env::remove_var("CARGO_HOME");
            env::remove_var("HOME");
            env::set_var("USERPROFILE", &tmp_root);

            let mut analyzer = CommandAnalyzer::default();
            let found = analyzer.find_external_type_path_cached("WinOnly");
            assert_eq!(
                found.unwrap(),
                file_path,
                "USERPROFILE fallback must locate the type without HOME/CARGO_HOME",
            );

            let _ = std::fs::remove_dir_all(&tmp_root);
        }
    }

    mod external_type_lookup_caching {
        use super::*;
        use serial_test::serial;
        use std::env;
        use std::fs;
        use std::path::PathBuf;

        /// Build a fake registry rooted at a temp `CARGO_HOME`, write the given
        /// files under `registry/src/dummy-0.1.0/`, set `CARGO_HOME`, and return
        /// the analyzer + a handle on the temp dir for cleanup.
        fn registry_with(files: &[(&str, &str)]) -> (PathBuf, CommandAnalyzer) {
            let root: PathBuf = std::env::temp_dir().join(format!(
                "tauri_typegen_caching_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
            ));
            let _ = std::fs::remove_dir_all(&root);
            let cargo_home: PathBuf = root.join(".cargo");
            let crate_dir: PathBuf = cargo_home.join("registry/src/dummy-0.1.0");
            fs::create_dir_all(&crate_dir).expect("create registry crate dir");
            for (name, content) in files {
                fs::write(crate_dir.join(name), content).expect("write file");
            }
            env::set_var("CARGO_HOME", &cargo_home);
            (root, CommandAnalyzer::default())
        }

        /// A successful lookup populates the cache so the next lookup for the
        /// same name is served without re-walking.
        #[test]
        #[serial]
        fn caches_positive_result() {
            let (root, mut analyzer) = registry_with(&[("lib.rs", "pub struct CachedFoo;")]);

            let first = analyzer.find_external_type_path_cached("CachedFoo");
            assert!(first.is_some(), "first lookup should find the type");
            // The cache must now hold the result.
            assert_eq!(
                analyzer.external_type_lookup_cache.get("CachedFoo"),
                Some(&first.clone()),
                "positive result must be memoized in external_type_lookup_cache",
            );

            let second = analyzer.find_external_type_path_cached("CachedFoo");
            assert_eq!(first, second, "second lookup must return the cached value",);

            let _ = std::fs::remove_dir_all(&root);
        }

        /// A failed lookup records `None` in the cache so a repeated lookup for
        /// the same missing type does not re-walk the registry (#87's core
        /// complaint: no negative-result caching).
        #[test]
        #[serial]
        fn caches_negative_result() {
            let (root, mut analyzer) = registry_with(&[("lib.rs", "pub struct SomethingElse;")]);

            let first = analyzer.find_external_type_path_cached("MissingType");
            assert!(first.is_none(), "first lookup should miss");
            // Negative results are cached as `Some(None)` so repeats are O(1).
            assert_eq!(
                analyzer.external_type_lookup_cache.get("MissingType"),
                Some(&None),
                "negative result must be memoized as Some(None)",
            );

            let second = analyzer.find_external_type_path_cached("MissingType");
            assert!(
                second.is_none(),
                "second lookup for a cached-miss must still return None",
            );

            let _ = std::fs::remove_dir_all(&root);
        }

        /// Distinct type names get independent cache entries.
        #[test]
        #[serial]
        fn caches_distinct_types_independently() {
            let (root, mut analyzer) =
                registry_with(&[("lib.rs", "pub struct Alpha;\npub struct Beta;\n")]);

            let alpha = analyzer.find_external_type_path_cached("Alpha");
            let beta = analyzer.find_external_type_path_cached("Beta");
            // A lookup for a third, absent name should not disturb the others.
            let gamma = analyzer.find_external_type_path_cached("Gamma");

            assert!(alpha.is_some() && beta.is_some());
            assert!(gamma.is_none());
            assert_eq!(analyzer.external_type_lookup_cache.len(), 3);
            assert!(analyzer.external_type_lookup_cache.contains_key("Alpha"));
            assert!(analyzer.external_type_lookup_cache.contains_key("Beta"));
            assert!(analyzer.external_type_lookup_cache.contains_key("Gamma"));

            let _ = std::fs::remove_dir_all(&root);
        }
    }

    mod external_type_cache_seeding {
        use super::*;
        use serial_test::serial;
        use std::env;
        use std::fs;
        use std::path::PathBuf;

        /// Seeding the analyzer from a previous run's index must short-circuit
        /// the registry walk: a seeded positive entry is returned even when the
        /// registry is empty (so the walk would otherwise return None).
        #[test]
        #[serial]
        fn seeded_positive_entry_skips_walk() {
            let tmp_root: PathBuf = std::env::temp_dir().join(format!(
                "tauri_typegen_seeding_pos_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
            ));
            let _ = std::fs::remove_dir_all(&tmp_root);
            // Empty registry — the walk would find nothing.
            let cargo_home: PathBuf = tmp_root.join(".cargo");
            fs::create_dir_all(&cargo_home).expect("create empty cargo home");
            env::set_var("CARGO_HOME", &cargo_home);

            let mut analyzer = CommandAnalyzer::default();
            let mut seed: HashMap<String, Option<PathBuf>> = HashMap::new();
            seed.insert(
                "SeededType".to_string(),
                Some(PathBuf::from("/fake/registry/seeded.rs")),
            );
            analyzer.seed_external_type_cache(seed);

            let found = analyzer.find_external_type_path_cached("SeededType");
            assert_eq!(
                found,
                Some(PathBuf::from("/fake/registry/seeded.rs")),
                "a seeded positive entry must be returned without walking",
            );

            let _ = std::fs::remove_dir_all(&tmp_root);
        }

        /// A seeded negative entry is honored: the walk is skipped and None is
        /// returned even if the type now exists in the registry. (Stale
        /// negatives are an accepted trade-off matching .typecache's existing
        /// source-stability assumption; invalidation rides the command/struct
        /// hashes via needs_regeneration.)
        #[test]
        #[serial]
        fn seeded_negative_entry_skips_walk() {
            let tmp_root: PathBuf = std::env::temp_dir().join(format!(
                "tauri_typegen_seeding_neg_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
            ));
            let _ = std::fs::remove_dir_all(&tmp_root);
            let cargo_home: PathBuf = tmp_root.join(".cargo");
            let crate_dir: PathBuf = cargo_home.join("registry/src/dummy-0.1.0");
            fs::create_dir_all(&crate_dir).expect("create registry crate dir");
            // The type genuinely exists now, but the seed says it's missing.
            fs::write(crate_dir.join("lib.rs"), "pub struct ActuallyHere;").expect("write file");
            env::set_var("CARGO_HOME", &cargo_home);

            let mut analyzer = CommandAnalyzer::default();
            let mut seed: HashMap<String, Option<PathBuf>> = HashMap::new();
            seed.insert("ActuallyHere".to_string(), None);
            analyzer.seed_external_type_cache(seed);

            let found = analyzer.find_external_type_path_cached("ActuallyHere");
            assert!(
                found.is_none(),
                "a seeded negative entry must short-circuit and return None",
            );

            let _ = std::fs::remove_dir_all(&tmp_root);
        }

        /// A type absent from the seed is still resolved via the walk and then
        /// memoized, so a partial/empty seed is safe.
        #[test]
        #[serial]
        fn unseeded_type_falls_back_to_walk() {
            let tmp_root: PathBuf = std::env::temp_dir().join(format!(
                "tauri_typegen_seeding_fallback_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
            ));
            let _ = std::fs::remove_dir_all(&tmp_root);
            let cargo_home: PathBuf = tmp_root.join(".cargo");
            let crate_dir: PathBuf = cargo_home.join("registry/src/dummy-0.1.0");
            fs::create_dir_all(&crate_dir).expect("create registry crate dir");
            let file_path: PathBuf = crate_dir.join("lib.rs");
            fs::write(&file_path, "pub struct WalkResolved;").expect("write file");
            env::set_var("CARGO_HOME", &cargo_home);

            let mut analyzer = CommandAnalyzer::default();
            // Empty seed.
            analyzer.seed_external_type_cache(HashMap::new());

            let found = analyzer.find_external_type_path_cached("WalkResolved");
            assert_eq!(
                found.unwrap(),
                file_path,
                "an unseeded type must be found via the walk",
            );
            assert!(
                analyzer
                    .external_type_lookup_cache()
                    .contains_key("WalkResolved"),
                "the walk result must be memoized after resolving",
            );

            let _ = std::fs::remove_dir_all(&tmp_root);
        }
    }
}
