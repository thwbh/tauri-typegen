# Tauri TypeGen

[![Crates.io](https://img.shields.io/crates/v/tauri-typegen.svg)](https://crates.io/crates/tauri-typegen)
[![Documentation](https://docs.rs/tauri-typegen/badge.svg)](https://docs.rs/tauri-typegen)
[![codecov](https://codecov.io/gh/thwbh/tauri-typegen/branch/main/graph/badge.svg)](https://codecov.io/gh/thwbh/tauri-typegen)
[![CI](https://github.com/thwbh/tauri-typegen/workflows/CI/badge.svg)](https://github.com/thwbh/tauri-typegen/actions)

A command-line tool that automatically generates TypeScript bindings from your Tauri commands, eliminating manual frontend type creation.

## Features

- 🔍 **Automatic Discovery**: Scans Rust source for `#[tauri::command]` functions
- 📝 **TypeScript Generation**: Creates TypeScript interfaces for command parameters and return types
- ✅ **Validation Support**: Optional Zod schema generation with runtime validation
- 🚀 **Command Bindings**: Strongly-typed frontend functions
- 📡 **Event Support**: Discovers and types `app.emit()` events
- 📞 **Channel Support**: Types for streaming `Channel<T>` parameters
- 🏷️ **Serde Support**: Respects `#[serde(rename)]` and `#[serde(rename_all)]` attributes
- 🎯 **Type Safety**: Keeps frontend and backend types in sync
- 🛠️ **Build Integration**: Works as standalone CLI or build dependency

## Table of Contents

- [Installation](#installation)
- [Quick Setup](#quick-setup)
- [Recommended Setup](#recommended-setup)
- [Generated Code](#generated-code)
- [Using Generated Bindings](#using-generated-bindings)
- [TypeScript Compatibility](#typescript-compatibility)
- [API Reference](#api-reference)
- [Configuration](#configuration)

## Installation

Install globally as a CLI tool:

```bash
cargo install tauri-typegen
```

**Or** add as a build dependency to your Tauri project:

```bash
cargo add --build tauri-typegen
```

## Quick Setup

For trying it out or one-time generation:

```bash
# Install CLI
cargo install tauri-typegen

# Generate types once
cargo tauri-typegen generate

# Use generated bindings
```

This generates TypeScript files in `./src/generated/` from your `./src-tauri/` code.

## Recommended Setup

For integrated development workflow:

### 1. Install and Initialize

```bash
# Install CLI
cargo install tauri-typegen

# Initialize configuration (adds to tauri.conf.json)
cargo tauri-typegen init

# Or with custom settings
cargo tauri-typegen init --validation zod --output tauri.conf.json
```

This creates a configuration block in your `tauri.conf.json`:

```json
{
  "plugins": {
    "tauri-typegen": {
      "project_path": "./src-tauri",
      "output_path": "./src/generated",
      "validation_library": "none",
      "verbose": false
    }
  }
}
```

### 2. Add Build Hook

Add to `src-tauri/build.rs`:

```rust
fn main() {
    // Generate TypeScript bindings before build
    tauri_typegen::generate_at_build_time()
        .expect("Failed to generate TypeScript bindings");

    tauri_build::build()
}
```

Now types auto-generate on every Rust build:

```bash
npm run tauri dev   # Types generated automatically
npm run tauri build # Types generated automatically
```

## Generated Code

### Example Rust Code

```rust
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

#[derive(Clone, Serialize)]
pub struct ProgressUpdate {
    pub percentage: f32,
    pub message: String,
}

// Simple command
#[tauri::command]
pub async fn get_user(id: i32) -> Result<User, String> {
    // Implementation
}

// Command with custom types
#[tauri::command]
pub async fn create_user(request: CreateUserRequest) -> Result<User, String> {
    // Implementation
}

// Command with Channel for progress streaming
#[tauri::command]
pub async fn download_file(
    url: String,
    on_progress: Channel<ProgressUpdate>
) -> Result<String, String> {
    // Send progress updates
    on_progress.send(ProgressUpdate {
        percentage: 50.0,
        message: "Halfway done".to_string()
    })?;
    // Implementation
}

// Event emission
pub fn notify_user(app: &AppHandle, message: String) {
    app.emit("user-notification", message).unwrap();
}
```

### Generated Files

```
src/generated/
├── types.ts       # TypeScript interfaces
├── commands.ts    # Typed command functions
└── events.ts      # Event listener functions (if events detected)
```

**Generated `types.ts`:**

```typescript
import type { Channel } from '@tauri-apps/api/core';

export interface User {
  id: number;
  name: string;
  email: string;
}

export interface CreateUserRequest {
  name: string;
  email: string;
}

export interface ProgressUpdate {
  percentage: number;
  message: string;
}

export interface GetUserParams {
  id: number;
}

export interface CreateUserParams {
  request: CreateUserRequest;
}

export interface DownloadFileParams {
  url: string;
  onProgress: Channel<ProgressUpdate>;
}
```

**Generated `commands.ts`:**

```typescript
import { invoke, Channel } from '@tauri-apps/api/core';
import * as types from './types';

export async function getUser(params: types.GetUserParams): Promise<types.User> {
  return invoke('get_user', params);
}

export async function createUser(params: types.CreateUserParams): Promise<types.User> {
  return invoke('create_user', params);
}

export async function downloadFile(params: types.DownloadFileParams): Promise<string> {
  return invoke('download_file', params);
}
```

**Generated `events.ts`:**

```typescript
import { listen } from '@tauri-apps/api/event';

export async function onUserNotification(handler: (event: string) => void) {
  return listen('user-notification', (event) => handler(event.payload as string));
}
```

### With Zod Validation

When using `--validation zod`, generated commands include runtime validation:

```typescript
export async function createUser(
  params: types.CreateUserParams,
  hooks?: CommandHooks<types.User>
): Promise<types.User> {
  try {
    const result = types.CreateUserParamsSchema.safeParse(params);

    if (!result.success) {
      hooks?.onValidationError?.(result.error);
      throw result.error;
    }

    const data = await invoke<types.User>('create_user', result.data);
    hooks?.onSuccess?.(data);
    return data;
  } catch (error) {
    if (!(error instanceof ZodError)) {
      hooks?.onInvokeError?.(error);
    }
    throw error;
  } finally {
    hooks?.onSettled?.();
  }
}
```

## Using Generated Bindings

### Basic Usage

```typescript
import { getUser, createUser, downloadFile } from './generated';
import { Channel } from '@tauri-apps/api/core';

// Simple command
const user = await getUser({ id: 1 });

// With custom types
const newUser = await createUser({
  request: {
    name: "John Doe",
    email: "john@example.com"
  }
});

// With Channel for streaming
const onProgress = new Channel<ProgressUpdate>();
onProgress.onmessage = (progress) => {
  console.log(`${progress.percentage}%: ${progress.message}`);
};

const result = await downloadFile({
  url: "https://example.com/file.zip",
  onProgress
});
```

### With Event Listeners

```typescript
import { onUserNotification } from './generated';

// Listen for events
const unlisten = await onUserNotification((message) => {
  console.log('Notification:', message);
});

// Stop listening
unlisten();
```

### React Example

```tsx
import React, { useState } from 'react';
import { createUser } from './generated';
import type { User } from './generated';

export function CreateUserForm() {
  const [name, setName] = useState('');
  const [email, setEmail] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    const user = await createUser({
      request: { name, email }
    });

    console.log('Created:', user);
  };

  return (
    <form onSubmit={handleSubmit}>
      <input value={name} onChange={e => setName(e.target.value)} />
      <input value={email} onChange={e => setEmail(e.target.value)} />
      <button type="submit">Create User</button>
    </form>
  );
}
```

### With Zod Validation Hooks

```typescript
import { createUser } from './generated';
import { toast } from 'sonner';

await createUser(
  { request: userData },
  {
    onValidationError: (err) => toast.error(err.errors[0].message),
    onInvokeError: (err) => toast.error('Failed to create user'),
    onSuccess: (user) => toast.success(`Created ${user.name}!`),
  }
);
```

## TypeScript Compatibility

### Requirements

- **TypeScript 5.0+**
- **Zod 4.x** (when using Zod validation)
- **ES2018+** target

### tsconfig.json

```json
{
  "compilerOptions": {
    "target": "ES2018",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true
  }
}
```

### Type Mappings

| Rust Type | TypeScript |
|-----------|-----------|
| `String`, `&str` | `string` |
| `i32`, `f64`, etc. | `number` |
| `bool` | `boolean` |
| `Option<T>` | `T \| null` |
| `Vec<T>` | `T[]` |
| `HashMap<K,V>` | `Map<K,V>` |
| `(T,U)` | `[T,U]` |
| `Channel<T>` | `Channel<T>` |
| `Result<T,E>` | `T` (errors via Promise rejection) |

### Serde Attribute Support

Tauri-typegen respects serde serialization attributes to ensure generated TypeScript types match your JSON API:

#### Field Renaming

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "userId")]
    pub user_id: i32,
    pub name: String,
}
```

Generates:

```typescript
export interface User {
  userId: number;  // Field renamed as specified
  name: string;
}
```

#### Struct-Level Naming Convention

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse {
    pub user_id: i32,
    pub user_name: String,
    pub is_active: bool,
}
```

Generates:

```typescript
export interface ApiResponse {
  userId: number;      // snake_case → camelCase
  userName: string;    // snake_case → camelCase
  isActive: boolean;   // snake_case → camelCase
}
```

**Supported naming conventions:**
- `camelCase`
- `PascalCase`
- `snake_case`
- `SCREAMING_SNAKE_CASE`
- `kebab-case`
- `SCREAMING-KEBAB-CASE`

#### Field Precedence

Field-level `rename` takes precedence over struct-level `rename_all`:

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub user_id: i32,              // → userId
    #[serde(rename = "fullName")]
    pub user_name: String,          // → fullName (override)
}
```

#### Skip Fields

```rust
#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    #[serde(skip)]
    pub internal_data: String,  // Not included in TypeScript
}
```

#### Enum Support

Enums also support serde rename attributes:

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MyEnum {
    HelloWorld,  // → HELLO_WORLD
    ByeWorld,    // → BYE_WORLD
}
```

Generates:

```typescript
export type MyEnum = "HELLO_WORLD" | "BYE_WORLD";

// With Zod:
export const MyEnumSchema = z.enum(["HELLO_WORLD", "BYE_WORLD"]);
```

Variant-level rename also works:

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    InProgress,           // → inProgress
    #[serde(rename = "not-started")]
    NotStarted,          // → not-started (override)
}
```

## API Reference

### CLI Commands

```bash
# Generate bindings
cargo tauri-typegen generate [OPTIONS]

Options:
  -p, --project-path <PATH>     Tauri source directory [default: ./src-tauri]
  -o, --output-path <PATH>      Output directory [default: ./src/generated]
  -v, --validation <LIBRARY>    Validation library: zod or none [default: none]
      --verbose                 Verbose output
      --visualize-deps          Generate dependency graph
  -c, --config <FILE>          Config file path
```

```bash
# Initialize configuration
cargo tauri-typegen init [OPTIONS]

Options:
  -p, --project-path <PATH>     Tauri source directory [default: ./src-tauri]
  -g, --generated-path <PATH>   Output directory [default: ./src/generated]
  -o, --output <FILE>          Config file [default: tauri.conf.json]
  -v, --validation <LIBRARY>    Validation library [default: none]
      --force                   Overwrite existing config
```

### Build Script API

```rust
// In src-tauri/build.rs
fn main() {
    tauri_typegen::generate_at_build_time()
        .expect("Failed to generate TypeScript bindings");

    tauri_build::build()
}
```

### Programmatic API

```rust
use tauri_typegen::{GenerateConfig, generate_from_config};

let config = GenerateConfig {
    project_path: "./src-tauri".to_string(),
    output_path: "./src/generated".to_string(),
    validation_library: "none".to_string(),
    verbose: Some(true),
};

let files = generate_from_config(&config)?;
```

## Configuration

### Standalone Config File

```json
{
  "project_path": "./src-tauri",
  "output_path": "./src/generated",
  "validation_library": "none",
  "verbose": false
}
```

### Tauri Config Integration

In `tauri.conf.json`:

```json
{
  "plugins": {
    "tauri-typegen": {
      "project_path": "./src-tauri",
      "output_path": "./src/generated",
      "validation_library": "zod",
      "verbose": true
    }
  }
}
```

### Validation Options

- **`none`** (default): TypeScript types only, no runtime validation
- **`zod`**: Generate Zod schemas with runtime validation and hooks

## Contributing
 
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT license.
