use crate::analysis::CommandAnalyzer;
use crate::generators::generator::BindingsGenerator;
use crate::models::*;
use crate::Result;

// Simple ping function for testing (removed Tauri runtime dependency)
pub fn ping(payload: PingRequest) -> Result<PingResponse> {
    Ok(PingResponse {
        value: Some(format!(
            "Pong: {}",
            payload.value.unwrap_or_else(|| "Unknown".to_string())
        )),
    })
}

pub async fn analyze_commands(payload: AnalyzeCommandsRequest) -> Result<AnalyzeCommandsResponse> {
    let mut analyzer = CommandAnalyzer::new();

    match analyzer.analyze_project(&payload.project_path) {
        Ok(commands) => Ok(AnalyzeCommandsResponse { commands }),
        Err(e) => Err(crate::Error::CommandAnalysis(e.to_string())),
    }
}

pub async fn generate_models(payload: GenerateModelsRequest) -> Result<GenerateModelsResponse> {
    let mut analyzer = CommandAnalyzer::new();

    // Analyze commands first
    let commands = analyzer
        .analyze_project(&payload.project_path)
        .map_err(|e| crate::Error::CommandAnalysis(e.to_string()))?;

    // Generate TypeScript models with struct information
    let mut generator = BindingsGenerator::new(payload.validation_library);
    let output_path = payload
        .output_path
        .unwrap_or_else(|| format!("{}/generated", payload.project_path));

    let generated_files = generator
        .generate_models(&commands, analyzer.get_discovered_structs(), &output_path, &analyzer)
        .map_err(|e| crate::Error::CodeGeneration(e.to_string()))?;

    Ok(GenerateModelsResponse {
        generated_files,
        commands_found: commands.len() as i32,
        types_generated: commands.iter().map(|c| c.parameters.len()).sum::<usize>() as i32,
    })
}
