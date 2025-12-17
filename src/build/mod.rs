pub mod dependency_resolver;
pub mod output_manager;
pub mod project_scanner;

use crate::analysis::CommandAnalyzer;
use crate::generators::create_generator;
use crate::interface::config::{ConfigError, GenerateConfig};
use crate::interface::output::{Logger, ProgressReporter};
use std::path::Path;

pub use dependency_resolver::*;
pub use output_manager::*;
pub use project_scanner::*;

/// Build-time code generation orchestrator
pub struct BuildSystem {
    logger: Logger,
}

impl BuildSystem {
    pub fn new(verbose: bool, debug: bool) -> Self {
        Self {
            logger: Logger::new(verbose, debug),
        }
    }

    /// Generate TypeScript bindings at build time
    pub fn generate_at_build_time() -> Result<(), Box<dyn std::error::Error>> {
        let build_system = Self::new(false, false);
        build_system.run_generation()
    }

    /// Run the complete generation process
    pub fn run_generation(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut reporter = ProgressReporter::new(self.logger.clone(), 5);

        reporter.start_step("Detecting Tauri project");
        let project_scanner = ProjectScanner::new();
        let project_info = match project_scanner.detect_project()? {
            Some(info) => {
                reporter.complete_step(Some(&format!(
                    "Found project at {}",
                    info.root_path.display()
                )));
                info
            }
            None => {
                reporter.complete_step(Some("No Tauri project detected, skipping generation"));
                return Ok(());
            }
        };

        reporter.start_step("Loading configuration");
        let config = self.load_configuration(&project_info)?;
        reporter.complete_step(Some(&format!(
            "Using {} validation with output to {}",
            config.validation_library, config.output_path
        )));

        reporter.start_step("Setting up build dependencies");
        self.setup_build_dependencies(&config)?;
        reporter.complete_step(None);

        reporter.start_step("Analyzing and generating bindings");
        let generated_files = self.generate_bindings(&config)?;
        reporter.complete_step(Some(&format!("Generated {} files", generated_files.len())));

        reporter.start_step("Managing output");
        let mut output_manager = OutputManager::new(&config.output_path);
        output_manager.finalize_generation(&generated_files)?;
        reporter.complete_step(None);

        reporter.finish(&format!(
            "Successfully generated TypeScript bindings for {} commands",
            generated_files.len()
        ));

        Ok(())
    }

    fn load_configuration(
        &self,
        project_info: &ProjectInfo,
    ) -> Result<GenerateConfig, ConfigError> {
        // Try to load from tauri.conf.json first
        if let Some(tauri_config_path) = &project_info.tauri_config_path {
            if tauri_config_path.exists() {
                match GenerateConfig::from_tauri_config(tauri_config_path) {
                    Ok(Some(config)) => {
                        self.logger
                            .debug("Loaded configuration from tauri.conf.json");
                        return Ok(config);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        self.logger.warning(&format!(
                            "Failed to load config from tauri.conf.json: {}. Using defaults.",
                            e
                        ));
                    }
                }
            }
        }

        // Try standalone config file
        let standalone_config = project_info.root_path.join("typegen.json");
        if standalone_config.exists() {
            match GenerateConfig::from_file(&standalone_config) {
                Ok(config) => {
                    self.logger.debug("Loaded configuration from typegen.json");
                    return Ok(config);
                }
                Err(e) => {
                    self.logger.warning(&format!(
                        "Failed to load config from typegen.json: {}. Using defaults.",
                        e
                    ));
                }
            }
        }

        // Use defaults
        self.logger.debug("Using default configuration");
        Ok(GenerateConfig::default())
    }

    fn setup_build_dependencies(
        &self,
        config: &GenerateConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Set up cargo rerun directives
        println!("cargo:rerun-if-changed={}", config.project_path);

        // Watch for changes in configuration files
        if Path::new("tauri.conf.json").exists() {
            println!("cargo:rerun-if-changed=tauri.conf.json");
        }
        if Path::new("typegen.json").exists() {
            println!("cargo:rerun-if-changed=typegen.json");
        }

        // Watch output directory for cleanup detection
        if Path::new(&config.output_path).exists() {
            println!("cargo:rerun-if-changed={}", config.output_path);
        }

        Ok(())
    }

    fn generate_bindings(
        &self,
        config: &GenerateConfig,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut analyzer = CommandAnalyzer::new();
        let commands = analyzer.analyze_project(&config.project_path)?;

        if commands.is_empty() {
            self.logger
                .info("No Tauri commands found. Skipping generation.");
            return Ok(vec![]);
        }

        let validation = match config.validation_library.as_str() {
            "zod" | "none" => Some(config.validation_library.clone()),
            _ => return Err("Invalid validation library. Use 'zod' or 'none'".into()),
        };

        let mut generator = create_generator(validation);
        let generated_files = generator.generate_models(
            &commands,
            analyzer.get_discovered_structs(),
            &config.output_path,
            &analyzer,
            &config,
        )?;

        // Generate dependency visualization if requested
        if config.should_visualize_deps() {
            self.generate_dependency_visualization(&analyzer, &commands, &config.output_path)?;
        }

        Ok(generated_files)
    }

    fn generate_dependency_visualization(
        &self,
        analyzer: &CommandAnalyzer,
        commands: &[crate::models::CommandInfo],
        output_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;

        self.logger.debug("Generating dependency visualization");

        let text_viz = analyzer.visualize_dependencies(commands);
        let viz_file_path = Path::new(output_path).join("dependency-graph.txt");
        fs::write(&viz_file_path, text_viz)?;

        let dot_viz = analyzer.generate_dot_graph(commands);
        let dot_file_path = Path::new(output_path).join("dependency-graph.dot");
        fs::write(&dot_file_path, dot_viz)?;

        self.logger.verbose(&format!(
            "Generated dependency graphs: {} and {}",
            viz_file_path.display(),
            dot_file_path.display()
        ));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_build_system_creation() {
        let build_system = BuildSystem::new(true, false);
        assert!(build_system
            .logger
            .should_log(crate::interface::output::LogLevel::Verbose));
    }

    #[test]
    fn test_load_default_configuration() {
        let temp_dir = TempDir::new().unwrap();
        let project_info = ProjectInfo {
            root_path: temp_dir.path().to_path_buf(),
            src_tauri_path: temp_dir.path().join("src-tauri"),
            tauri_config_path: None,
        };

        let build_system = BuildSystem::new(false, false);
        let config = build_system.load_configuration(&project_info).unwrap();

        assert_eq!(config.validation_library, "none");
        assert_eq!(config.project_path, "./src-tauri");
    }
}
