# Tauri TypeGen Example App

This example demonstrates the Tauri TypeGen CLI tool capabilities using a Svelte + Vite frontend.

## Features Demonstrated

- Basic Tauri command (`greet`)
- TypeScript type generation
- Integration with Svelte frontend
- CLI tool testing environment

## Quick Start

1. **Install dependencies**:
   ```bash
   npm install
   ```

2. **Generate TypeScript bindings**:
   ```bash
   # From the project root
   ../../target/debug/cargo-tauri-typegen tauri-typegen generate --project-path ./src-tauri --output-path ./src/generated --validation zod --verbose
   ```

3. **Run the development server**:
   ```bash
   npm run tauri dev
   ```

## Testing the CLI Tool

This example serves as a test environment for the tauri-typegen CLI. The `greet` command in `src-tauri/src/lib.rs` provides a simple function to test type generation.

### Generated Files

When you run the CLI tool, it will generate:
- `src/generated/types.ts` - TypeScript interfaces
- `src/generated/schemas.ts` - Zod validation schemas (if using zod)
- `src/generated/commands.ts` - Typed command functions
- `src/generated/index.ts` - Barrel exports

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

