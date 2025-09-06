use crate::models::{CommandInfo, StructInfo};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Dependency graph for lazy type resolution
#[derive(Debug, Default)]
pub struct TypeDependencyGraph {
    /// Maps type name to the files where it's defined
    pub type_definitions: HashMap<String, PathBuf>,
    /// Maps type name to types it depends on
    pub dependencies: HashMap<String, HashSet<String>>,
    /// Maps type name to its resolved StructInfo
    pub resolved_types: HashMap<String, StructInfo>,
}

impl TypeDependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a type definition to the graph
    pub fn add_type_definition(&mut self, type_name: String, file_path: PathBuf) {
        self.type_definitions.insert(type_name, file_path);
    }

    /// Add a dependency relationship between types
    pub fn add_dependency(&mut self, dependent: String, dependency: String) {
        self.dependencies
            .entry(dependent)
            .or_default()
            .insert(dependency);
    }

    /// Add multiple dependencies for a type
    pub fn add_dependencies(&mut self, dependent: String, dependencies: HashSet<String>) {
        self.dependencies.insert(dependent, dependencies);
    }

    /// Add a resolved type to the graph
    pub fn add_resolved_type(&mut self, type_name: String, struct_info: StructInfo) {
        self.resolved_types.insert(type_name, struct_info);
    }

    /// Get all resolved types
    pub fn get_resolved_types(&self) -> &HashMap<String, StructInfo> {
        &self.resolved_types
    }

    /// Get dependencies for a type
    pub fn get_dependencies(&self, type_name: &str) -> Option<&HashSet<String>> {
        self.dependencies.get(type_name)
    }

    /// Check if a type is defined in the graph
    pub fn has_type_definition(&self, type_name: &str) -> bool {
        self.type_definitions.contains_key(type_name)
    }

    /// Get the file path where a type is defined
    pub fn get_type_definition_path(&self, type_name: &str) -> Option<&PathBuf> {
        self.type_definitions.get(type_name)
    }

    /// Perform topological sort on the given types using the dependency graph
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

    /// Recursive helper for topological sorting with cycle detection
    fn topological_visit(
        &self,
        type_name: &str,
        sorted: &mut Vec<String>,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
    ) {
        // Check for cycles
        if visiting.contains(type_name) {
            eprintln!("Warning: Circular dependency detected involving type: {}", type_name);
            return;
        }
        
        if visited.contains(type_name) {
            return;
        }
        
        visiting.insert(type_name.to_string());
        
        // Visit dependencies first
        if let Some(deps) = self.dependencies.get(type_name) {
            for dep in deps {
                self.topological_visit(dep, sorted, visited, visiting);
            }
        }
        
        visiting.remove(type_name);
        visited.insert(type_name.to_string());
        sorted.push(type_name.to_string());
    }

    /// Build visualization of the dependency graph
    pub fn visualize_dependencies(&self, entry_commands: &[crate::models::CommandInfo]) -> String {
        let mut output = String::new();
        output.push_str("🌐 Type Dependency Graph\n");
        output.push_str("======================\n\n");
        
        // Show command entry points
        output.push_str("📋 Command Entry Points:\n");
        for cmd in entry_commands {
            output.push_str(&format!("• {} ({}:{})\n", cmd.name, cmd.file_path, cmd.line_number));
            
            // Show parameters
            for param in &cmd.parameters {
                output.push_str(&format!("  ├─ {}: {} → {}\n", param.name, param.rust_type, param.typescript_type));
            }
            
            // Show return type
            output.push_str(&format!("  └─ returns: {} → {}\n", cmd.return_type, cmd.return_type));
        }
        
        output.push_str("\n🏗️  Discovered Types:\n");
        for (type_name, struct_info) in &self.resolved_types {
            let type_kind = if struct_info.is_enum { "enum" } else { "struct" };
            output.push_str(&format!(
                "• {} ({}) - {} fields - defined in {}\n",
                type_name,
                type_kind,
                struct_info.fields.len(),
                struct_info.file_path
            ));
            
            // Show dependencies
            if let Some(deps) = self.dependencies.get(type_name) {
                if !deps.is_empty() {
                    let deps_list: Vec<String> = deps.iter().cloned().collect();
                    output.push_str(&format!("  └─ depends on: {}\n", deps_list.join(", ")));
                }
            }
        }
        
        // Show dependency chains
        output.push_str("\n🔗 Dependency Chains:\n");
        for type_name in self.resolved_types.keys() {
            self.show_dependency_chain(type_name, &mut output, 0);
        }
        
        output.push_str(&format!(
            "\n📊 Summary:\n• {} commands analyzed\n• {} types discovered\n• {} files with type definitions\n",
            entry_commands.len(),
            self.resolved_types.len(),
            self.type_definitions.len()
        ));
        
        output
    }

    /// Recursively show dependency chain for a type
    fn show_dependency_chain(&self, type_name: &str, output: &mut String, indent: usize) {
        let indent_str = "  ".repeat(indent);
        output.push_str(&format!("{}├─ {}\n", indent_str, type_name));
        
        if let Some(deps) = self.dependencies.get(type_name) {
            for dep in deps {
                if indent < 3 { // Prevent too deep recursion in visualization
                    self.show_dependency_chain(dep, output, indent + 1);
                }
            }
        }
    }

    /// Generate a DOT graph representation of the dependency graph
    pub fn generate_dot_graph(&self, commands: &[CommandInfo]) -> String {
        let mut output = String::new();
        output.push_str("digraph Dependencies {\n");
        output.push_str("  rankdir=LR;\n");
        output.push_str("  node [shape=box];\n");
        output.push('\n');

        // Add command nodes
        for command in commands {
            output.push_str(&format!("  \"{}\" [color=blue, style=filled, fillcolor=lightblue];\n", command.name));
        }

        // Add type nodes
        for type_name in self.resolved_types.keys() {
            output.push_str(&format!("  \"{}\" [color=green];\n", type_name));
        }

        // Add edges from commands to their parameter/return types
        for command in commands {
            for param in &command.parameters {
                if self.resolved_types.contains_key(&param.rust_type) {
                    output.push_str(&format!("  \"{}\" -> \"{}\" [label=\"param\"];\n", command.name, param.rust_type));
                }
            }
            if self.resolved_types.contains_key(&command.return_type) {
                output.push_str(&format!("  \"{}\" -> \"{}\" [label=\"return\"];\n", command.name, command.return_type));
            }
        }

        // Add type dependency edges
        for (type_name, deps) in &self.dependencies {
            for dep in deps {
                output.push_str(&format!("  \"{}\" -> \"{}\";\n", type_name, dep));
            }
        }

        output.push_str("}\n");
        output
    }
}