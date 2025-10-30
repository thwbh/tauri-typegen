# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.4] - 2025-10-30

### Added
- Added support for custom validation error messages in Zod schemas
  - Validator `message` parameters are now extracted from Rust `#[validate(...)]` attributes
  - Error messages are properly escaped and included in Zod schema validations
  - Supports messages for `length`, `range`, `email`, and `url` constraints
  - Example: `#[validate(length(min = 5, max = 50, message = "Must be 5-50 chars"))]` generates `z.string().min(5, { message: "Must be 5-50 chars" })`

### Fixed
- Fixed vanilla TypeScript interface properties not using camelCase naming convention
  - Interface properties now consistently use camelCase (e.g., `stringMap` instead of `string_map`)
  - Matches the naming convention already used in Zod schemas for consistency

### Changed
- Updated `z.record()` generation to use explicit two-parameter syntax for better type safety
  - Now generates `z.record(z.string(), z.number())` instead of `z.record(z.number())`
  - More explicit about both key and value types

## [0.1.3] - 2025-10-20

### Fixed
- Fixed camelCase naming convention not being consistently applied to generated TypeScript types
- Fixed configuration file not being properly loaded from `tauri.conf.json`
- Fixed topological sorting and forward references in type dependency resolution after refactoring
- Fixed CLI config loading to properly detect and use `tauri.conf.json` from project directory

### Changed
- Changed enum generation to use Zod native enums (`z.enum()`) instead of union types 
## [0.1.2] - 2025-10-20

### Fixed
- Fixed invalid `tauri.conf.json` generation when using standalone configuration files
- Fixed `tauri.conf.json` path resolution to correctly place file in Rust project directory (`{project-path}/tauri.conf.json`)
- Improved path detection for default configuration file location using proper parent directory checks

### Changed
- Reduced CLI verbosity with cleaner progress output using animated spinners (indicatif library)
- Updated `init` command to require existing `tauri.conf.json` and only update the `plugins.typegen` section
- Configuration file now defaults to `{project-path}/tauri.conf.json` instead of project root
- Enhanced verbose mode to properly propagate through AST parsing and analysis steps

### Added
- Added `indicatif` dependency for progress bar animations
- Added comprehensive tests for CLI configuration conversion
- Added tests for tauri.conf.json preservation and plugin section management

## [0.1.1] - 2025-09-06

### Fixed
- Fixed invalid `tauri.conf.json` generation when using standalone configuration files
- Fixed `tauri.conf.json` path resolution to correctly place file in project directory
- Improved path detection for default configuration file location

### Changed
- Reduced CLI verbosity with cleaner progress output using animated spinners
- Improved user experience with single-line progress indication during generation
- Removed unnecessary info emojis from logging output
- Updated `init` command to require existing `tauri.conf.json` and only update the `plugins.typegen` section
- Configuration file now defaults to `{project-path}/tauri.conf.json` instead of project root

## [0.1.0] - 2025-09-05

### Added
- Initial release of tauri-plugin-typegen
- TypeScript bindings generation for Tauri commands
- Support for Zod validation library integration
- Support for vanilla TypeScript types (no validation)
- AST caching for improved performance
- Single-pass traversal for efficient code analysis
- Optional dependency graph visualization (text and DOT formats)
- CLI with `generate` and `init` commands
- Configuration file support (standalone JSON or integrated with `tauri.conf.json`)
- Comprehensive type mapping for Rust to TypeScript conversion
- Support for complex types including:
  - Structs and enums
  - Generic types
  - Collections (Vec, HashMap, HashSet, BTreeMap, BTreeSet)
  - Option and Result types
  - Tuples
- Topological sorting for proper type dependency ordering
- Command discovery via `#[tauri::command]` attribute parsing
- Automatic generation of barrel exports (`index.ts`)

### Features
- **Zod Generation Mode**: Generates schemas, inferred types, commands, and exports
- **Vanilla TypeScript Mode**: Generates types, commands, and exports without validation
- **Verbose Mode**: Detailed logging for debugging and understanding the generation process
- **Modular Architecture**:
  - Separate analyzer submodules for commands, types, and dependencies
  - Strategy pattern for generator implementations
  - Clean separation between AST parsing and code generation

### Developer Experience
- Progress reporting with step-by-step feedback
- Helpful error messages with actionable guidance
- Usage examples printed after successful generation
- Dependency visualization tools for understanding type relationships
- Extensive test coverage

