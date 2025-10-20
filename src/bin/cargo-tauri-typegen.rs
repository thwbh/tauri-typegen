use clap::Parser;
use std::fs;
use std::path::PathBuf;
use tauri_plugin_typegen::analysis::CommandAnalyzer;
use tauri_plugin_typegen::generators::generator::BindingsGenerator;
use tauri_plugin_typegen::interface::{
    print_dependency_visualization_info, print_usage_info, CargoCli, CargoSubcommands,
    GenerateConfig, Logger, ProgressReporter, TypegenCommands,
};

fn main() {
    let args = CargoCli::parse();

    match args.command {
        CargoSubcommands::TauriTypegen(typegen_args) => match typegen_args.command {
            TypegenCommands::Generate {
                project_path,
                output_path,
                validation_library,
                verbose,
                visualize_deps,
                config_file,
            } => {
                if let Err(e) = run_generate(
                    project_path,
                    output_path,
                    validation_library,
                    verbose,
                    visualize_deps,
                    config_file,
                ) {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
            TypegenCommands::Init {
                project_path,
                generated_path,
                output_path,
                validation_library,
                verbose,
                visualize_deps,
                force,
            } => {
                if let Err(e) = run_init(
                    project_path,
                    generated_path,
                    output_path,
                    validation_library,
                    verbose,
                    visualize_deps,
                    force,
                ) {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        },
    }
}

fn run_generate(
    project_path: PathBuf,
    output_path: PathBuf,
    validation_library: String,
    verbose: bool,
    visualize_deps: bool,
    config_file: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::new(verbose, false);
    let mut reporter = ProgressReporter::new(logger, 4);

    // Load configuration
    reporter.start_step("Loading configuration");
    let mut config = if let Some(config_path) = config_file {
        if config_path.exists() {
            GenerateConfig::from_file(config_path)?
        } else {
            return Err(format!("Configuration file not found: {}", config_path.display()).into());
        }
    } else {
        // Try to load from tauri.conf.json if it exists
        if std::path::Path::new("tauri.conf.json").exists() {
            GenerateConfig::from_tauri_config("tauri.conf.json").unwrap_or_default()
        } else {
            GenerateConfig::default()
        }
    };

    // Override with CLI arguments
    let cli_config = GenerateConfig {
        project_path: project_path.to_string_lossy().to_string(),
        output_path: output_path.to_string_lossy().to_string(),
        validation_library,
        verbose: Some(verbose),
        visualize_deps: Some(visualize_deps),
        ..Default::default()
    };
    config.merge(&cli_config);
    reporter.complete_step(Some(&format!(
        "Using {} validation",
        config.validation_library
    )));

    // Validate paths and configuration
    reporter.start_step("Validating project structure");
    config.validate()?;
    reporter.complete_step(None);

    // Analyze and generate
    reporter.start_step("Analyzing Tauri commands");
    let mut analyzer = CommandAnalyzer::new();
    let commands =
        analyzer.analyze_project_with_verbose(&config.project_path, config.is_verbose())?;

    if config.is_verbose() {
        reporter.update_progress(&format!("Found {} Tauri commands", commands.len()));
        commands.iter().for_each(|cmd| {
            reporter.update_progress(&format!("  - {} ({})", cmd.name, cmd.file_path));
        });

        let discovered_structs = analyzer.get_discovered_structs();
        reporter.update_progress(&format!(
            "Found {} struct definitions",
            discovered_structs.len()
        ));
        discovered_structs.iter().for_each(|(name, struct_info)| {
            let struct_type = if struct_info.is_enum {
                "enum"
            } else {
                "struct"
            };
            reporter.update_progress(&format!(
                "  - {} ({}) with {} fields",
                name,
                struct_type,
                struct_info.fields.len()
            ));
        });
    }
    reporter.complete_step(Some(&format!("Found {} commands", commands.len())));

    if commands.is_empty() {
        println!("⚠️  No Tauri commands found. Make sure your project contains functions with #[tauri::command] attributes.");
        return Ok(());
    }

    // Generate bindings
    reporter.start_step("Generating TypeScript bindings");
    let validation = match config.validation_library.as_str() {
        "zod" | "none" => Some(config.validation_library.clone()),
        _ => return Err("Invalid validation library. Use 'zod' or 'none'".into()),
    };

    let mut generator = BindingsGenerator::new(validation);
    let generated_files = generator.generate_models(
        &commands,
        analyzer.get_discovered_structs(),
        &config.output_path,
        &analyzer,
    )?;
    reporter.complete_step(Some(&format!("Generated {} files", generated_files.len())));

    // Generate dependency visualization if requested
    if config.should_visualize_deps() {
        let text_viz = analyzer.visualize_dependencies(&commands);
        let viz_file_path = PathBuf::from(&config.output_path).join("dependency-graph.txt");
        fs::write(&viz_file_path, text_viz)?;

        let dot_viz = analyzer.generate_dot_graph(&commands);
        let dot_file_path = PathBuf::from(&config.output_path).join("dependency-graph.dot");
        fs::write(&dot_file_path, dot_viz)?;

        print_dependency_visualization_info(&config.output_path);
    }

    // Print summary
    reporter.finish("Generation complete");
    print_usage_info(&config.output_path, &generated_files, commands.len());

    Ok(())
}

fn run_init(
    project_path: PathBuf,
    generated_path: PathBuf,
    mut output_path: PathBuf,
    validation_library: String,
    verbose: bool,
    visualize_deps: bool,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::new(verbose, false);

    logger.info("🚀 Initializing Tauri TypeScript generation configuration");

    // If output path is just "tauri.conf.json" (default), place it in the project path
    let has_no_meaningful_parent = output_path
        .parent()
        .map(|p| p.as_os_str().is_empty())
        .unwrap_or(true);

    if output_path.file_name().and_then(|n| n.to_str()) == Some("tauri.conf.json")
        && has_no_meaningful_parent
    {
        output_path = project_path.join("tauri.conf.json");
    }

    let is_tauri_config =
        output_path.file_name().and_then(|n| n.to_str()) == Some("tauri.conf.json");

    // For tauri.conf.json, we always update/merge (no force required)
    // For custom config files, require force if they exist
    if !is_tauri_config && output_path.exists() && !force {
        return Err(format!(
            "Configuration file already exists at {}. Use --force to overwrite.",
            output_path.display()
        )
        .into());
    }

    // Create configuration
    let config = GenerateConfig {
        project_path: project_path.to_string_lossy().to_string(),
        output_path: generated_path.to_string_lossy().to_string(),
        validation_library,
        verbose: Some(verbose),
        visualize_deps: Some(visualize_deps),
        ..Default::default()
    };

    // Determine file format and save
    if is_tauri_config {
        // For tauri.conf.json, require it to exist
        if !output_path.exists() {
            return Err(format!(
                "tauri.conf.json not found at {}.\n\
                 Please ensure you have a Tauri project initialized.\n\
                 Run 'cargo tauri init' or use --output to specify a different config file.",
                output_path.display()
            )
            .into());
        }

        config.save_to_tauri_config(&output_path)?;
        logger.info(&format!(
            "✅ Updated typegen configuration in {}",
            output_path.display()
        ));
    } else {
        config.save_to_file(&output_path)?;
        logger.info(&format!(
            "✅ Created configuration file: {}",
            output_path.display()
        ));
    }

    // Print configuration summary
    logger.info("📋 Configuration summary:");
    logger.info(&format!("  • Project path: {}", config.project_path));
    logger.info(&format!(
        "  • Generated files output path: {}",
        config.output_path
    ));
    logger.info(&format!(
        "  • Validation library: {}",
        config.validation_library
    ));

    // Now run initial generation
    logger.info("");
    logger.info("🔄 Running initial generation...");

    run_generate(
        project_path,
        generated_path,
        config.validation_library.clone(),
        verbose,
        visualize_deps,
        None, // No config file since we just created one
    )?;

    logger.info("");
    logger.info(
        "✨ Initialization complete! Your Tauri project is now set up for TypeScript generation.",
    );
    logger.info("💡 You can run 'cargo tauri-typegen generate' anytime to regenerate bindings.");

    Ok(())
}
