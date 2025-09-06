use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use crate::interface::config::GenerateConfig;

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
    #[command(subcommand)]
    pub command: TypegenCommands,
}

#[derive(Subcommand)]
pub enum TypegenCommands {
    /// Generate TypeScript models and bindings from Tauri commands
    Generate {
        /// Path to the Tauri project source directory (default: ./src-tauri)
        #[arg(short = 'p', long = "project-path", default_value = "./src-tauri")]
        project_path: PathBuf,

        /// Output path for generated TypeScript files (default: ./src/generated)
        #[arg(short = 'o', long = "output-path", default_value = "./src/generated")]
        output_path: PathBuf,

        /// Validation library to use (zod or none)
        #[arg(short = 'v', long = "validation", default_value = "zod")]
        validation_library: String,

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
    /// Initialize configuration for a Tauri project
    Init {
        /// Output path for configuration file (default: tauri.conf.json)
        #[arg(short = 'o', long = "output", default_value = "tauri.conf.json")]
        output_path: PathBuf,

        /// Validation library to use (zod or none)
        #[arg(short = 'v', long = "validation", default_value = "zod")]
        validation_library: String,

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
            } => GenerateConfig {
                project_path: project_path.to_string_lossy().to_string(),
                output_path: output_path.to_string_lossy().to_string(),
                validation_library: validation_library.clone(),
                verbose: Some(*verbose),
                visualize_deps: Some(*visualize_deps),
                ..Default::default()
            },
            TypegenCommands::Init { validation_library, .. } => GenerateConfig {
                validation_library: validation_library.clone(),
                ..Default::default()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_generate_config_from_cli() {
        let cmd = TypegenCommands::Generate {
            project_path: PathBuf::from("./src-tauri"),
            output_path: PathBuf::from("./src/generated"),
            validation_library: "zod".to_string(),
            verbose: false,
            visualize_deps: false,
            config_file: None,
        };

        let config = GenerateConfig::from(&cmd);
        assert_eq!(config.project_path, "./src-tauri");
        assert_eq!(config.output_path, "./src/generated");
        assert_eq!(config.validation_library, "zod");
        assert!(!config.verbose.unwrap_or(false));
    }

    #[test]
    fn test_custom_generate_config_from_cli() {
        let cmd = TypegenCommands::Generate {
            project_path: PathBuf::from("./my-tauri"),
            output_path: PathBuf::from("./types"),
            validation_library: "none".to_string(),
            verbose: true,
            visualize_deps: true,
            config_file: None,
        };

        let config = GenerateConfig::from(&cmd);
        assert_eq!(config.project_path, "./my-tauri");
        assert_eq!(config.output_path, "./types");
        assert_eq!(config.validation_library, "none");
        assert!(config.verbose.unwrap_or(false));
        assert!(config.visualize_deps.unwrap_or(false));
    }
}