import { invoke } from '@tauri-apps/api/core';

export interface PingRequest {
  value?: string;
  [key: string]: unknown;
}

export interface PingResponse {
  value?: string;
}

export interface GenerateModelsRequest {
  projectPath: string;
  outputPath?: string;
  validationLibrary?: string;
  [key: string]: unknown;
}

export interface GenerateModelsResponse {
  generatedFiles: string[];
  commandsFound: number;
  typesGenerated: number;
}

export interface AnalyzeCommandsRequest {
  projectPath: string;
  [key: string]: unknown;
}

export interface AnalyzeCommandsResponse {
  commands: CommandInfo[];
}

export interface CommandInfo {
  name: string;
  filePath: string;
  lineNumber: number;
  parameters: ParameterInfo[];
  returnType: string;
  isAsync: boolean;
}

export interface ParameterInfo {
  name: string;
  rustType: string;
  typescriptType: string;
  isOptional: boolean;
}

/**
 * Ping the plugin to test connectivity
 */
export async function ping(request: PingRequest): Promise<PingResponse> {
  return invoke('plugin:typegen|ping', request);
}

/**
 * Analyze a Tauri project to find all commands and their types
 */
export async function analyzeCommands(request: AnalyzeCommandsRequest): Promise<AnalyzeCommandsResponse> {
  return invoke('plugin:typegen|analyze_commands', request);
}

/**
 * Generate TypeScript models and bindings for a Tauri project
 */
export async function generateModels(request: GenerateModelsRequest): Promise<GenerateModelsResponse> {
  return invoke('plugin:typegen|generate_models', request);
}

/**
 * Generate models with sensible defaults
 */
export async function generateModelsSimple(
  projectPath: string,
  options?: {
    outputPath?: string;
    validationLibrary?: 'zod' | 'none';
  }
): Promise<GenerateModelsResponse> {
  return generateModels({
    projectPath,
    outputPath: options?.outputPath,
    validationLibrary: options?.validationLibrary,
  });
}
