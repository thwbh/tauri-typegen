const COMMANDS: &[&str] = &["ping"];

fn main() {
    // Build the Tauri plugin
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .ios_path("ios")
        .build();

    // Run TypeScript generation at build time if configured
    if std::env::var("CARGO_FEATURE_BUILD_TIME_GENERATION").is_ok() {
        if let Err(e) = run_build_time_generation() {
            println!("cargo:warning=Failed to generate TypeScript bindings: {}", e);
        }
    }
}

fn run_build_time_generation() -> Result<(), Box<dyn std::error::Error>> {
    // For build-time generation, we would need to implement a separate
    // standalone generator or use the CLI binary.
    // For now, we'll just set up the rerun directives.
    
    println!("cargo:rerun-if-changed=src-tauri");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    
    // Could call the CLI binary here if needed:
    // std::process::Command::new("cargo-tauri-typegen")...
    
    Ok(())
}
