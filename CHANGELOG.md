# Changelog

All notable changes to this project will be documented in this file.


The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.3] - 2025-12-04

### Added
- Generated files now contain a timestamp and tauri-typegen version info
- Added `--version` flag as CLI argument


### Changed
- Generated zod code now includes type coercion for numbers

## [0.3.2] - 2025-11-23

### Fixed
- **Nested Collection Type Discovery**: Fixed bug where nested collection types (e.g., `Vec<HashMap<String, Vec<T>>>`) were not properly discovered when the type was not referenced standalone
  - Type analyzer now recursively traverses nested generic types to discover all referenced custom types
  - Ensures all nested types are included in the generated TypeScript bindings

### Changed
- Cleaned up unused permissions directory from project structure

## [0.3.1] - 2025-11-19

### Fixed
- **Zod Command Generation with Channels**: Fixed bug where commands with both regular parameters and channels generated invalid TypeScript code
  - Previously referenced non-existent `extractChannels` function
  - Now generates explicit channel parameter references (e.g., `{ ...result.data, onProgress: params.onProgress }`)

## [0.3.0] - 2025-11-13

### Added
- **Tauri Channel Support**: Automatically generate TypeScript bindings for Tauri IPC Channels
  - New `ChannelParser` detects `Channel<T>` parameters in commands
  - Generates TypeScript channel listener boilerplate with proper typing
  - Supports both bare `Channel<T>` and qualified `tauri::ipc::Channel<T>` syntax

- **Tauri Event Support**: Automatically discover and generate TypeScript event listeners
  - New `EventParser` detects `app.emit()`, `window.emit()`, and `app.emit_to()` calls
  - Generates TypeScript event listener boilerplate with proper payload types
  - Infers payload types from emitted expressions (structs, primitives, variables)
  - Discovers events in conditionals, loops, match arms, and nested blocks
  - Auto-imports event payload types in generated `events.ts`

- **Serde Attribute Support**: Respect Serde serialization attributes
  - `#[serde(rename = "customName")]` - field-level renaming
  - `#[serde(rename_all = "camelCase")]` - struct-level naming convention (camelCase, snake_case, PascalCase, kebab-case, SCREAMING_SNAKE_CASE, SCREAMING-KEBAB-CASE)
  - `#[serde(skip)]` - exclude fields from generated types
  - Field renames properly override struct-level conventions
  - Applies to both structs and enums with variant-level rename support

- **Improved Tauri Parameter Filtering**: More robust detection of Tauri-specific parameters
  - Properly filters `AppHandle`, `Window`, `WebviewWindow`, `State<T>`, `Manager`, `Channel<T>`, `Request`
  - Handles both fully-qualified (`tauri::AppHandle`) and imported (`AppHandle`) types
  - Uses AST-based type checking instead of string matching
  - Prevents false positives for user types with similar names

- **GitHub Actions CI/CD**: Automated testing and code coverage
  - Runs tests, formatting checks, and Clippy linting on all PRs and pushes
  - Generates code coverage reports with `cargo-tarpaulin`
  - Uploads coverage to Codecov automatically

- **Git Hooks with cargo-husky**: Automatic code formatting on commit
  - Pre-commit hook runs `cargo fmt` automatically
  - Auto-stages formatted files for commit

### Changed
- **BREAKING**: Default validation library changed from `"zod"` to `"none"`
  - Running `generate` without `--validation` flag now generates vanilla TypeScript
  - Explicit `--validation zod` required for Zod schema generation

- **BREAKING**: Removed all Tauri runtime dependencies
  - Removed `tauri` dependency from `Cargo.toml` (was only needed for plugin feature)
  - Removed `tauri-plugin` build dependency
  - Removed plugin-specific code from `build.rs`
  - Removed mobile-specific error variant (`PluginInvokeError`)
  - Project is now a pure code generation library with no runtime requirements
  - Eliminates glib-sys dependency issues in CI environments

- **Code Quality Improvements**:
  - Converted recursive methods to associated functions (static methods) to eliminate clippy warnings
  - Changed methods like `type_to_string(&self, ty)` to `type_to_string(ty)` where `self` was only used for recursion
  - Cleaner, more honest API that accurately reflects function dependencies

### Removed
- **JavaScript/TypeScript Build Setup**: Removed all Node.js-based build infrastructure
  - Removed `package.json`, `rollup.config.js`, `tsconfig.json`
  - Removed `guest-js/` directory (was for plugin guest code)
  - Tool is now purely a Rust CLI with no JavaScript build dependencies

- **Old Example App**: Removed outdated `examples/tauri-app/` directory
  - Example apps have been reorganized but not committed to repository
  - Cleaner project structure focused on the core library

### Fixed
- Fixed vanilla TypeScript generator not using camelCase for function names
  - Command functions now properly convert from snake_case to camelCase (e.g., `get_user_count` → `getUserCount()`)

- Fixed Channel message types not being included in generated type files
  - Channel generic types are now properly extracted and collected
  - `Channel<T>` now ensures `T` is exported in `types.ts`
  - Event payload types are now auto-imported in `events.ts`

- Fixed TypeScript compilation errors for command parameter interfaces
  - Parameter interfaces now include `[key: string]: unknown;` index signature
  - Satisfies Tauri's `InvokeArgs` type requirement
  - Only applies to parameter interfaces, not response types

- Fixed verbose mode propagation through AST parsing steps
  - Verbose logging now works consistently across all analysis phases

## [0.2.1] - 2025-11-02

### Fixed
- **CRITICAL**: Fixed CLI arguments being ignored when configuration file exists
  - CLI arguments (`--validation`, `--project-path`, `--output-path`) now properly override config file settings
  - Previously, when a config file was present, CLI flags were replaced by their default values, causing user-specified options to be ignored
  - Changed CLI argument types to `Option<T>` to distinguish between "not specified" and "specified with default value"
  - Boolean flags (`--verbose`, `--visualize-deps`) now only override config when explicitly provided
  - Fixes issue where running without `--validation` flag would ignore config file's validation library setting

### Changed
- CLI argument precedence is now: CLI Arguments > Config File > Hardcoded Defaults
- Improved help text for CLI arguments to clarify default behavior and config file integration

## [0.2.0] - 2025-11-01

### Added
- Added optional hook feature for customizing generated TypeScript code
  - Allows users to inject custom code transformations into the generation process
  - Comprehensive test coverage for hook functionality
- Added build timestamp and crate version information to generated files
  - Generated files now include metadata about when and with which version they were created

### Changed
- **BREAKING**: Renamed crate from `tauri-plugin-typegen` to `tauri-typegen`
  - Updated all references, imports, and documentation
  - Updated package names in both Cargo.toml and package.json
  - Updated capability permissions and schema references

## [0.1.5] - 2025-10-31 

### Fixed
- Fixed regression where `Optional` custom types were not exported properly
- Fixed regression where primitve types were not imported properly

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
- Initial release of tauri-typegen
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

