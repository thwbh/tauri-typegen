# Project: tauri-plugin-typegen

## Project Description
Tauri currently does not provide an option to generate Javascript or Typescript bindings based on the tauri commands 
and associated models used in the Rust code. This project fits the gap and additionally ensures consistent vaidation 
if the validator crate is being used.

## Tech Stack
- Rust
- Testing: cargo test

## Code Conventions
- Use rust fmt for formatting

## Project Structure
- /src - Main source code
  - /bin - CLI main function
- /tests - Test files

## Important Notes
- This plugin supports two validation flags: 'zod' and 'none' 
  - zod: Typescript-First schema validation library with static type inference: https://zod.dev/
  - none: No validation happens and only vanilla Typescript types are generated.
- Changes need to be validated against existing tests
- New features should be validated against new tests
- Project uses AST cache and attempts single pass traversal
- Project provides an optional dependency graph


# Future plans
- When using zod, the schema validation should match the parameters provided in the validator macro
  - Example: `#[validate(range(min = 1, max = 10, message = "Must be between 1 and 10"))]` (rust) translates to `z.number().min(1).max(100)` and takes over the error message as defined here: https://zod.dev/api

## Modularization Plan

### 3. Type System Module (src/types/)

  Create a dedicated module for type handling:

  src/types/
  ├── mod.rs                   # Public API
  ├── rust_types.rs           # Rust type definitions and parsing
  ├── typescript_types.rs     # TypeScript type generation
  ├── mappings.rs            # Type mapping tables
  ├── complex_types.rs       # Generics, tuples, collections
  └── validation_types.rs    # Validator attribute handling

  Benefits:
  - Centralizes type system logic
  - Supports CLAUDE.md goal of enhanced type system
  - Easier to extend for generics and complex types

###  5. Core Models (src/core/)

  Keep models but add domain-specific groupings:

  src/core/
  ├── mod.rs                 # Re-exports
  ├── ast_models.rs          # AST-related data structures
  ├── command_models.rs      # Command and parameter info
  ├── type_models.rs         # Type and struct definitions
  └── validation_models.rs   # Validator attributes and constraints

2. Type System Enhancements

  - Issue: Limited support for complex Rust types (generics, associated types, trait bounds)
  - Suggestion:
    - Add support for generic structs and enums
    - Handle impl types and trait objects
    - Support for serde rename attributes
  - Implementation: Expand type mapping system and add generic type parameter tracking

## Nice to Have

### 3. Macro-Based Command Discovery

**Concept**: Instead of parsing all Rust files for `#[tauri::command]` functions, extract commands directly from `tauri::generate_handler!` macro invocations.

**Benefits**:
- Higher accuracy - only analyzes actually registered commands
- Better performance - no need to parse entire codebase
- Eliminates false positives from unused commands
- Single source of truth with the application's actual command registration

**Implementation Approaches**:
1. **Macro Parsing**: Parse `main.rs`/`lib.rs` for `tauri::generate_handler![cmd1, cmd2, cmd3]` patterns
2. **Build Script Integration**: Hook into Rust compilation process to access macro expansion
3. **Proc Macro Hook**: Create custom proc macro wrapper around `generate_handler!`
4. **Hybrid Approach**: Try macro parsing first, fall back to file parsing, cross-reference for validation

**Challenges**:
- Multiple registration points (plugins, conditional registration)
- Complex proc macro analysis required
- Integration with Rust compilation process
- Dynamic command registration scenarios

**Resources**:
- Tauri generate_handler macro docs: https://docs.rs/tauri/latest/tauri/macro.generate_handler.html
- tauri-macros crate: https://docs.rs/tauri-macros/latest/tauri_macros/
- Tauri v2 calling rust guide: https://v2.tauri.app/develop/calling-rust/

**Priority**: Low (current file-parsing approach works well for most use cases)

