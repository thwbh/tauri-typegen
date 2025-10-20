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
    /// Initialize configuration for a Tauri project and run initial generation
    Init {
        /// Path to the Tauri project source directory (default: ./src-tauri)
        #[arg(short = 'p', long = "project-path", default_value = "./src-tauri")]
        project_path: PathBuf,

        /// Output path for generated TypeScript files (default: ./src/generated)
        #[arg(short = 'g', long = "generated-path", default_value = "./src/generated")]
        generated_path: PathBuf,

        /// Output path for configuration file (default: tauri.conf.json)
        #[arg(short = 'o', long = "output", default_value = "tauri.conf.json")]
        output_path: PathBuf,

        /// Validation library to use (zod or none)
        #[arg(short = 'v', long = "validation", default_value = "zod")]
        validation_library: String,

        /// Tauri app identifier (e.g., "com.company.app")
        #[arg(short = 'i', long = "identifier")]
        tauri_identifier: Option<String>,

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
            } => GenerateConfig {
                project_path: project_path.to_string_lossy().to_string(),
                output_path: output_path.to_string_lossy().to_string(),
                validation_library: validation_library.clone(),
                verbose: Some(*verbose),
                visualize_deps: Some(*visualize_deps),
                ..Default::default()
            },
            TypegenCommands::Init {
                project_path,
                generated_path,
                validation_library,
                tauri_identifier,
                verbose,
                visualize_deps,
                ..
            } => GenerateConfig {
                project_path: project_path.to_string_lossy().to_string(),
                output_path: generated_path.to_string_lossy().to_string(),
                validation_library: validation_library.clone(),
                tauri_identifier: tauri_identifier.clone(),
                verbose: Some(*verbose),
                visualize_deps: Some(*visualize_deps),
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

    #[test]
    fn test_default_init_config_from_cli() {
        let cmd = TypegenCommands::Init {
            project_path: PathBuf::from("./src-tauri"),
            generated_path: PathBuf::from("./src/generated"),
            output_path: PathBuf::from("tauri.conf.json"),
            validation_library: "zod".to_string(),
            tauri_identifier: None,
            verbose: false,
            visualize_deps: false,
            force: false,
        };

        let config = GenerateConfig::from(&cmd);
        assert_eq!(config.project_path, "./src-tauri");
        assert_eq!(config.output_path, "./src/generated");
        assert_eq!(config.validation_library, "zod");
        assert_eq!(config.tauri_identifier, None);
        assert!(!config.verbose.unwrap_or(true));
        assert!(!config.visualize_deps.unwrap_or(true));
    }

    #[test]
    fn test_custom_init_config_from_cli() {
        let cmd = TypegenCommands::Init {
            project_path: PathBuf::from("./my-tauri"),
            generated_path: PathBuf::from("./my-types"),
            output_path: PathBuf::from("custom.conf.json"),
            validation_library: "none".to_string(),
            tauri_identifier: Some("com.example.myapp".to_string()),
            verbose: true,
            visualize_deps: true,
            force: true,
        };

        let config = GenerateConfig::from(&cmd);
        assert_eq!(config.project_path, "./my-tauri");
        assert_eq!(config.output_path, "./my-types");
        assert_eq!(config.validation_library, "none");
        assert_eq!(config.tauri_identifier, Some("com.example.myapp".to_string()));
        assert!(config.verbose.unwrap_or(false));
        assert!(config.visualize_deps.unwrap_or(false));
    }
}
