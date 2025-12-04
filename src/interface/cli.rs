use crate::interface::config::GenerateConfig;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
pub struct CargoCli {
    #[command(subcommand)]
    pub command: CargoSubcommands,
}

#[derive(Subcommand)]
pub enum CargoSubcommands {
    #[command(name = "tauri-typegen")]
    TauriTypegen(TauriTypegenArgs),
}

#[derive(Args)]
pub struct TauriTypegenArgs {
    /// Print version information
    #[arg(short = 'V', long = "version")]
    pub version: bool,

    #[command(subcommand)]
    pub command: Option<TypegenCommands>,
}

#[derive(Subcommand)]
pub enum TypegenCommands {
    /// Generate TypeScript models and bindings from Tauri commands
    Generate {
        /// Path to the Tauri project source directory. Defaults to config file value or "./src-tauri"
        #[arg(short = 'p', long = "project-path")]
        project_path: Option<PathBuf>,

        /// Output path for generated TypeScript files. Defaults to config file value or "./src/generated"
        #[arg(short = 'o', long = "output-path")]
        output_path: Option<PathBuf>,

        /// Validation library to use (zod or none). Defaults to config file value or "none"
        #[arg(short = 'v', long = "validation")]
        validation_library: Option<String>,

        /// Verbose output
        #[arg(long, action = clap::ArgAction::SetTrue)]
        verbose: bool,

        /// Generate dependency graph visualization
        #[arg(long, action = clap::ArgAction::SetTrue)]
        visualize_deps: bool,

        /// Configuration file path
        #[arg(short = 'c', long = "config")]
        config_file: Option<PathBuf>,
    },
    /// Initialize configuration for a Tauri project and run initial generation
    Init {
        /// Path to the Tauri project source directory. Defaults to "./src-tauri"
        #[arg(short = 'p', long = "project-path")]
        project_path: Option<PathBuf>,

        /// Output path for generated TypeScript files. Defaults to "./src/generated"
        #[arg(short = 'g', long = "generated-path")]
        generated_path: Option<PathBuf>,

        /// Output path for configuration file. Defaults to "tauri.conf.json"
        #[arg(short = 'o', long = "output")]
        output_path: Option<PathBuf>,

        /// Validation library to use (zod or none). Defaults to "none"
        #[arg(short = 'v', long = "validation")]
        validation_library: Option<String>,

        /// Verbose output
        #[arg(long, action = clap::ArgAction::SetTrue)]
        verbose: bool,

        /// Generate dependency graph visualization
        #[arg(long, action = clap::ArgAction::SetTrue)]
        visualize_deps: bool,

        /// Force overwrite existing configuration
        #[arg(long, action = clap::ArgAction::SetTrue)]
        force: bool,
    },
}

impl From<&TypegenCommands> for GenerateConfig {
    fn from(cmd: &TypegenCommands) -> Self {
        match cmd {
            TypegenCommands::Generate {
                project_path,
                output_path,
                validation_library,
                verbose,
                visualize_deps,
                ..
            } => {
                let mut config = GenerateConfig::default();
                if let Some(p) = project_path {
                    config.project_path = p.to_string_lossy().to_string();
                }
                if let Some(o) = output_path {
                    config.output_path = o.to_string_lossy().to_string();
                }
                if let Some(v) = validation_library {
                    config.validation_library = v.clone();
                }
                // For boolean flags: only set if true (flag was present)
                if *verbose {
                    config.verbose = Some(true);
                }
                if *visualize_deps {
                    config.visualize_deps = Some(true);
                }
                config
            }
            TypegenCommands::Init {
                project_path,
                generated_path,
                validation_library,
                verbose,
                visualize_deps,
                ..
            } => {
                let mut config = GenerateConfig::default();
                if let Some(p) = project_path {
                    config.project_path = p.to_string_lossy().to_string();
                }
                if let Some(g) = generated_path {
                    config.output_path = g.to_string_lossy().to_string();
                }
                if let Some(v) = validation_library {
                    config.validation_library = v.clone();
                }
                // For boolean flags: only set if true (flag was present)
                if *verbose {
                    config.verbose = Some(true);
                }
                if *visualize_deps {
                    config.visualize_deps = Some(true);
                }
                config
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_generate_config_from_cli() {
        // When no CLI args are provided, should use defaults
        let cmd = TypegenCommands::Generate {
            project_path: None,
            output_path: None,
            validation_library: None,
            verbose: false,
            visualize_deps: false,
            config_file: None,
        };

        let config = GenerateConfig::from(&cmd);
        assert_eq!(config.project_path, "./src-tauri");
        assert_eq!(config.output_path, "./src/generated");
        assert_eq!(config.validation_library, "none");
        assert_eq!(config.verbose, Some(false));
        assert_eq!(config.visualize_deps, Some(false));
    }

    #[test]
    fn test_custom_generate_config_from_cli() {
        let cmd = TypegenCommands::Generate {
            project_path: Some(PathBuf::from("./my-tauri")),
            output_path: Some(PathBuf::from("./types")),
            validation_library: Some("none".to_string()),
            verbose: true,
            visualize_deps: true,
            config_file: None,
        };

        let config = GenerateConfig::from(&cmd);
        assert_eq!(config.project_path, "./my-tauri");
        assert_eq!(config.output_path, "./types");
        assert_eq!(config.validation_library, "none");
        assert_eq!(config.verbose, Some(true));
        assert_eq!(config.visualize_deps, Some(true));
    }

    #[test]
    fn test_partial_generate_config_from_cli() {
        // Test that only specified values override defaults
        let cmd = TypegenCommands::Generate {
            project_path: None,
            output_path: None,
            validation_library: Some("none".to_string()),
            verbose: true,
            visualize_deps: false,
            config_file: None,
        };

        let config = GenerateConfig::from(&cmd);
        assert_eq!(config.project_path, "./src-tauri"); // default
        assert_eq!(config.output_path, "./src/generated"); // default
        assert_eq!(config.validation_library, "none"); // overridden
        assert_eq!(config.verbose, Some(true)); // overridden
        assert_eq!(config.visualize_deps, Some(false)); // default (not set)
    }

    #[test]
    fn test_default_init_config_from_cli() {
        let cmd = TypegenCommands::Init {
            project_path: None,
            generated_path: None,
            output_path: None,
            validation_library: None,
            verbose: false,
            visualize_deps: false,
            force: false,
        };

        let config = GenerateConfig::from(&cmd);
        assert_eq!(config.project_path, "./src-tauri");
        assert_eq!(config.output_path, "./src/generated");
        assert_eq!(config.validation_library, "none");
        assert_eq!(config.verbose, Some(false));
        assert_eq!(config.visualize_deps, Some(false));
    }

    #[test]
    fn test_custom_init_config_from_cli() {
        let cmd = TypegenCommands::Init {
            project_path: Some(PathBuf::from("./my-tauri")),
            generated_path: Some(PathBuf::from("./my-types")),
            output_path: Some(PathBuf::from("custom.conf.json")),
            validation_library: Some("none".to_string()),
            verbose: true,
            visualize_deps: true,
            force: true,
        };

        let config = GenerateConfig::from(&cmd);
        assert_eq!(config.project_path, "./my-tauri");
        assert_eq!(config.output_path, "./my-types");
        assert_eq!(config.validation_library, "none");
        assert_eq!(config.verbose, Some(true));
        assert_eq!(config.visualize_deps, Some(true));
    }
}
