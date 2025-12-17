# Test Organization

This directory contains the test suite for tauri-typegen.

## Structure

```
tests/
├── common/                       # Shared test utilities
│   └── mod.rs                   # TestProject, TestGenerator helpers
│
├── fixtures/                     # Rust code examples for testing
│   ├── mod.rs
│   ├── basic_commands.rs        # Simple commands
│   ├── serde_attributes.rs      # Serde rename examples
│   ├── channels.rs              # IPC channel examples
│   └── complex_types.rs         # Nested types, collections, tuples
│
├── integration/                  # End-to-end integration tests
│   ├── test_basic_generation.rs
│   ├── test_serde_support.rs
│   ├── test_channels.rs
│   └── test_validation_libraries.rs
│
├── regression/                   # Backward compatibility tests
│   └── test_backward_compat.rs
│
└── [legacy test files]          # Old test files (to be migrated/deleted)
```

## Test Types

### Integration Tests (`integration/`)

End-to-end tests that verify the complete pipeline:
1. Parse Rust code
2. Analyze commands/types
3. Generate TypeScript/Zod output
4. Verify generated code

These tests use real Rust code fixtures and test the full workflow.

### Regression Tests (`regression/`)

Tests that ensure backward compatibility is maintained across versions.

### Unit Tests (in source files)

Focused tests for individual functions/traits. Located in `#[cfg(test)]` modules within source files (Phase 1 - not yet implemented).

## Writing New Integration Tests

```rust
mod common;
mod fixtures;

use common::{TestGenerator, TestProject};

#[test]
fn test_example() {
    // Create project with Rust code
    let project = TestProject::new();
    project.write_file("main.rs", fixtures::basic_commands::SIMPLE_COMMAND);

    // Analyze and generate
    let (analyzer, commands) = project.analyze();
    let generator = TestGenerator::new();
    generator.generate(
        &commands,
        analyzer.get_discovered_structs(),
        &analyzer,
        Some("none"), // or "zod"
        None,
    );

    // Assert on output
    let types = generator.read_file("types.ts");
    assert!(types.contains("expected"));
}
```

## Running Tests

```bash
# Run all tests
cargo test

# Run specific integration test file
cargo test --test test_basic_generation

# Run specific test
cargo test test_simple_command_generates_typescript

# Run with output
cargo test -- --nocapture
```

## Migration Status

See `MIGRATION.md` for the detailed migration plan from old test structure to new.

Current status: **Phase 2 in progress** - Integration test reorganization partially complete.

## Next Steps

1. ✅ Create common utilities and fixtures
2. ✅ Create initial integration test files
3. 🔄 Complete remaining integration tests
4. 🔄 Update old tests with new config parameter
5. ⏳ Delete old test files after verification
6. ⏳ Phase 1: Add unit tests to source files
