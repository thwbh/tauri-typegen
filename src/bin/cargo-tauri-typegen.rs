
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use tauri_plugin_typegen::analyzer::CommandAnalyzer;
use tauri_plugin_typegen::generator::TypeScriptGenerator;

#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
struct CargoCli {
    #[command(subcommand)]
    command: CargoSubcommands,
}

#[derive(Subcommand)]
enum CargoSubcommands {
    #[command(name = "tauri-typegen")]
    TauriTypegen(TauriTypegenArgs),
}

#[derive(Args)]
struct TauriTypegenArgs {
    #[command(subcommand)]
    command: TypegenCommands,
}

#[derive(Subcommand)]
enum TypegenCommands {
    /// Generate TypeScript models and bindings from Tauri commands
    Generate {
        /// Path to the Tauri project source directory (default: ./src-tauri)
        #[arg(short = 'p', long = "project-path", default_value = "./src-tauri")]
        project_path: PathBuf,

        /// Output path for generated TypeScript files (default: ./src/generated)
        #[arg(short = 'o', long = "output-path", default_value = "./src/generated")]
        output_path: PathBuf,

        /// Validation library to use (zod, yup, or none)
        #[arg(short = 'v', long = "validation", default_value = "zod")]
        validation_library: String,

        /// Verbose output
        #[arg(long, action = clap::ArgAction::SetTrue)]
        verbose: bool,
    },
}

fn main() {
    let args = CargoCli::parse();

    match args.command {
        CargoSubcommands::TauriTypegen(typegen_args) => {
            match typegen_args.command {
                TypegenCommands::Generate {
                    project_path,
                    output_path,
                    validation_library,
                    verbose,
                } => {
                    if let Err(e) = run_generate(project_path, output_path, validation_library, verbose) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
    }
}

fn run_generate(
    project_path: PathBuf,
    output_path: PathBuf,
    validation_library: String,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        println!("🔍 Analyzing Tauri commands in: {}", project_path.display());
    }

    // Validate paths
    if !project_path.exists() {
        return Err(format!("Project path does not exist: {}", project_path.display()).into());
    }

    // Analyze commands with struct discovery
    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer.analyze_project(&project_path.to_string_lossy())?;

    if verbose {
        println!("📋 Found {} Tauri commands:", commands.len());
        for cmd in &commands {
            println!("  - {} ({})", cmd.name, cmd.file_path);
        }

        let discovered_structs = analyzer.get_discovered_structs();
        println!("🏗️  Found {} struct definitions:", discovered_structs.len());
        for (name, struct_info) in discovered_structs {
            let struct_type = if struct_info.is_enum { "enum" } else { "struct" };
            println!("  - {} ({}) with {} fields", name, struct_type, struct_info.fields.len());
            if verbose {
                for field in &struct_info.fields {
                    let visibility = if field.is_public { "pub" } else { "private" };
                    let optional = if field.is_optional { "?" } else { "" };
                    println!("    • {}{}: {} ({})", field.name, optional, field.typescript_type, visibility);
                }
            }
        }

        if discovered_structs.is_empty() {
            println!("  ℹ️  No custom struct definitions found in the project");
        }
    }

    if commands.is_empty() {
        println!("⚠️  No Tauri commands found. Make sure your project contains functions with #[tauri::command] attributes.");
        return Ok(());
    }

    // Validate validation library
    let validation = match validation_library.as_str() {
        "zod" | "yup" | "none" => Some(validation_library),
        _ => {
            return Err("Invalid validation library. Use 'zod', 'yup', or 'none'".into());
        }
    };

    // Generate TypeScript models
    if verbose {
        println!("🚀 Generating TypeScript models with {} validation...", validation.as_ref().unwrap());
    }

    let mut generator = TypeScriptGenerator::new(validation);
    let generated_files = generator.generate_models(&commands, analyzer.get_discovered_structs(), &output_path.to_string_lossy())?;

    println!("✅ Successfully generated {} files for {} commands:", generated_files.len(), commands.len());
    for file in &generated_files {
        println!("  📄 {}/{}", output_path.display(), file);
    }

    println!("\n💡 Usage in your frontend:");
    println!("  import {{ createUser, getUsers }} from '{}/index'", output_path.display());

    Ok(())
}