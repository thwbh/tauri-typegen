# Tauri Plugin TypeGen

A Tauri CLI extension that automatically generates TypeScript models and bindings from your Tauri commands, eliminating the manual process of creating frontend types and validation.

## Features

- 🔍 **Automatic Discovery**: Scans your Rust source files to find all `#[tauri::command]` functions
- 📝 **TypeScript Generation**: Creates TypeScript interfaces for all command parameters and return types
- ✅ **Validation Support**: Generates validation schemas using Zod, Yup, or other libraries
- 🚀 **Command Bindings**: Creates strongly-typed frontend functions that call your Tauri commands
- 🎯 **Type Safety**: Ensures frontend and backend types stay in sync
- 🛠️ **CLI Integration**: Generate types with a simple command: `cargo tauri-typegen generate`

## Quick Start

1. **Install the CLI tool**:
   ```bash
   cargo install tauri-plugin-typegen
   ```

2. **Generate TypeScript bindings** from your Tauri project:
   ```bash
   # In your Tauri project root
   cargo tauri-typegen generate
   ```

3. **Use the generated bindings** in your frontend:
   ```typescript
   import { createUser, getUsers } from './src/generated';
   
   const user = await createUser({ request: { name: "John", email: "john@example.com" } });
   const users = await getUsers({ filter: null });
   ```

### CLI Options

```bash
cargo tauri-typegen generate [OPTIONS]

Options:
  -p, --project-path <PATH>    Path to Tauri source directory [default: ./src-tauri]
  -o, --output-path <PATH>     Output path for TypeScript files [default: ./src/generated]  
  -v, --validation <LIBRARY>   Validation library: zod, yup, or none [default: zod]
      --verbose               Verbose output
```

## Table of Contents

- [Quick Start](#quick-start)
- [Installation](#installation)
- [Usage Examples](#usage-examples)
  - [E-commerce App Example](#example-e-commerce-app)
  - [Generate TypeScript Bindings](#generate-typescript-bindings)
  - [Generated Files Structure](#generated-files-structure)
  - [Using Generated Bindings](#using-generated-bindings-in-frontend)
    - [React Example](#react-example)
    - [Vue Example](#vue-example)
    - [Svelte Example](#svelte-example)
  - [Benefits](#benefits-of-using-generated-bindings)
- [Generated Files](#generated-files)
- [API Reference](#api-reference)
- [Configuration Options](#configuration-options)
- [Development](#development)

## Installation

### CLI Tool Installation

Install the CLI tool globally:

```bash
cargo install tauri-plugin-typegen
```

### Build Integration (Optional)

For automatic generation during builds, add to your project's `build.rs`:

```rust
// build.rs
use tauri_plugin_typegen::build::generate_at_build_time;

fn main() {
    // Generate TypeScript bindings during build
    if let Err(e) = generate_at_build_time() {
        println!("cargo:warning=Failed to generate TypeScript bindings: {}", e);
    }
    
    tauri_build::build()
}
```

Add the build dependency to your `Cargo.toml`:

```toml
[build-dependencies]
tauri-plugin-typegen = { git = "https://github.com/yourusername/tauri-plugin-typegen" }
tauri-build = { version = "2.0", features = [] }
```

### Package.json Integration

Add generation to your build scripts:

```json
{
  "scripts": {
    "generate-types": "cargo tauri-typegen generate",
    "tauri:dev": "npm run generate-types && tauri dev", 
    "tauri:build": "npm run generate-types && tauri build"
  }
}

## Usage Examples

### Example: E-commerce App

Let's say you have these Tauri commands in your Rust backend:

**`src-tauri/src/commands/products.rs`:**
```rust
use serde::{Deserialize, Serialize};
use tauri::command;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub price: f64,
    pub in_stock: bool,
    pub category_id: i32,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateProductRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(length(max = 500))]
    pub description: String,
    #[validate(range(min = 0.01, max = 10000.0))]
    pub price: f64,
    pub category_id: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductFilter {
    pub search: Option<String>,
    pub category_id: Option<i32>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub in_stock_only: Option<bool>,
}

#[command]
pub async fn create_product(request: CreateProductRequest) -> Result<Product, String> {
    // Implementation here
    Ok(Product {
        id: 1,
        name: request.name,
        description: request.description,
        price: request.price,
        in_stock: true,
        category_id: request.category_id,
    })
}

#[command]
pub async fn get_products(filter: Option<ProductFilter>) -> Result<Vec<Product>, String> {
    // Implementation here
    Ok(vec![])
}

#[command]
pub async fn delete_product(id: i32) -> Result<(), String> {
    // Implementation here
    Ok(())
}
```

### Generate TypeScript Bindings

#### Command Line Generation

Generate bindings with the CLI tool:

```bash
# Basic generation with defaults
cargo tauri-typegen generate

# Custom paths and validation
cargo tauri-typegen generate \
  --project-path ./src-tauri \
  --output-path ./src/lib/generated \
  --validation zod \
  --verbose

# Quick examples for different setups
cargo tauri-typegen generate -p ./backend -o ./frontend/types -v yup
cargo tauri-typegen generate --validation none  # No validation schemas
```

#### Build Integration

Add automatic generation to your build process:

**`package.json`:**
```json
{
  "scripts": {
    "generate-types": "cargo tauri-typegen generate",
    "dev": "npm run generate-types && npm run tauri dev",
    "build": "npm run generate-types && npm run tauri build",
    "tauri": "tauri"
  }
}
```

**`Makefile` (optional):**
```makefile
.PHONY: generate-types dev build

generate-types:
	cargo tauri-typegen generate --verbose

dev: generate-types
	npm run tauri dev

build: generate-types  
	npm run tauri build
```

### Generated Files Structure

After running the generator:

```
src/lib/generated/
├── types.ts      # TypeScript interfaces
├── schemas.ts    # Zod validation schemas
├── commands.ts   # Typed command functions
└── index.ts      # Barrel exports
```

**Generated `types.ts`:**
```typescript
export interface Product {
  id: number;
  name: string;
  description: string;
  price: number;
  inStock: boolean;
  categoryId: number;
}

export interface CreateProductRequest {
  name: string;
  description: string;
  price: number;
  categoryId: number;
}

export interface CreateProductParams {
  request: CreateProductRequest;
}

export interface GetProductsParams {
  filter?: ProductFilter | null;
}

export interface DeleteProductParams {
  id: number;
}
```

**Generated `commands.ts`:**
```typescript
import { invoke } from '@tauri-apps/api/core';
import * as schemas from './schemas';
import type * as types from './types';

export async function createProduct(params: types.CreateProductParams): Promise<types.Product> {
  const validatedParams = schemas.CreateProductParamsSchema.parse(params);
  return invoke('create_product', validatedParams);
}

export async function getProducts(params: types.GetProductsParams): Promise<types.Product[]> {
  const validatedParams = schemas.GetProductsParamsSchema.parse(params);
  return invoke('get_products', validatedParams);
}

export async function deleteProduct(params: types.DeleteProductParams): Promise<void> {
  const validatedParams = schemas.DeleteProductParamsSchema.parse(params);
  return invoke('delete_product', validatedParams);
}
```

### Using Generated Bindings in Frontend

#### React Example

```tsx
import React, { useEffect, useState } from 'react';
import { getProducts, createProduct, deleteProduct } from '../lib/generated';
import type { Product, ProductFilter } from '../lib/generated';

export function ProductList() {
  const [products, setProducts] = useState<Product[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadProducts();
  }, []);

  const loadProducts = async () => {
    try {
      setLoading(true);
      const result = await getProducts({ filter: null });
      setProducts(result);
    } catch (error) {
      console.error('Failed to load products:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleCreateProduct = async () => {
    try {
      const newProduct = await createProduct({
        request: {
          name: 'New Product',
          description: 'A new product',
          price: 19.99,
          categoryId: 1,
        }
      });
      
      setProducts([...products, newProduct]);
    } catch (error) {
      console.error('Failed to create product:', error);
    }
  };

  const handleDeleteProduct = async (productId: number) => {
    try {
      await deleteProduct({ id: productId });
      setProducts(products.filter(p => p.id !== productId));
    } catch (error) {
      console.error('Failed to delete product:', error);
    }
  };

  if (loading) return <div>Loading...</div>;

  return (
    <div>
      <h2>Products</h2>
      <button onClick={handleCreateProduct}>Create Product</button>

      <div className="products">
        {products.map((product) => (
          <div key={product.id} className="product-card">
            <h3>{product.name}</h3>
            <p>{product.description}</p>
            <p>${product.price}</p>
            <p>Stock: {product.inStock ? '✅' : '❌'}</p>
            <button onClick={() => handleDeleteProduct(product.id)}>
              Delete
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
```

#### Vue Example

```vue
<template>
  <div class="product-manager">
    <h2>Product Manager</h2>
    
    <form @submit.prevent="createProduct" class="create-form">
      <input v-model="newProduct.name" placeholder="Product name" required />
      <textarea v-model="newProduct.description" placeholder="Description"></textarea>
      <input v-model.number="newProduct.price" type="number" step="0.01" placeholder="Price" required />
      <button type="submit">Create Product</button>
    </form>

    <div class="products-list">
      <div v-for="product in products" :key="product.id" class="product-item">
        <h4>{{ product.name }}</h4>
        <p>{{ product.description }}</p>
        <p>${{ product.price }}</p>
        <button @click="deleteProduct(product.id)">Delete</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { getProducts, createProduct as createProductCmd, deleteProduct as deleteProductCmd } from '../lib/generated';
import type { Product, CreateProductRequest } from '../lib/generated';

const products = ref<Product[]>([]);
const newProduct = ref<CreateProductRequest>({
  name: '',
  description: '',
  price: 0,
  categoryId: 1,
});

onMounted(async () => {
  await loadProducts();
});

const loadProducts = async () => {
  try {
    const result = await getProducts({ filter: null });
    products.value = result;
  } catch (error) {
    console.error('Failed to load products:', error);
  }
};

const createProduct = async () => {
  try {
    const product = await createProductCmd({ request: { ...newProduct.value } });
    products.value.push(product);
    newProduct.value = { name: '', description: '', price: 0, categoryId: 1 };
  } catch (error) {
    console.error('Failed to create product:', error);
  }
};

const deleteProduct = async (id: number) => {
  try {
    await deleteProductCmd({ id });
    products.value = products.value.filter(p => p.id !== id);
  } catch (error) {
    console.error('Failed to delete product:', error);
  }
};
</script>
```

#### Svelte Example

**`src/lib/ProductStore.ts`:**
```typescript
import { writable } from 'svelte/store';
import { getProducts, createProduct, deleteProduct } from './generated';
import type { Product, ProductFilter } from './generated';

export const products = writable<Product[]>([]);
export const loading = writable(false);

export const productStore = {
  async loadProducts(filter: ProductFilter = {}) {
    loading.set(true);
    try {
      const result = await getProducts({ filter });
      products.set(result);
    } catch (error) {
      console.error('Failed to load products:', error);
    } finally {
      loading.set(false);
    }
  },

  async createProduct(request: CreateProductRequest) {
    try {
      const newProduct = await createProduct({ request });
      products.update(items => [...items, newProduct]);
      return newProduct;
    } catch (error) {
      console.error('Failed to create product:', error);
      throw error;
    }
  },

  async deleteProduct(id: number) {
    try {
      await deleteProduct({ id });
      products.update(items => items.filter(p => p.id !== id));
    } catch (error) {
      console.error('Failed to delete product:', error);
      throw error;
    }
  }
};
```

### Benefits of Using Generated Bindings

#### ✅ Type Safety
```typescript
// ❌ Before: Manual typing, prone to errors
const result = await invoke('create_product', {
  name: 'Product',
  price: '19.99', // Oops! Should be number
  category_id: 1   // Oops! Should be camelCase
});

// ✅ After: Generated bindings with validation
const result = await createProduct({
  request: {
    name: 'Product',
    price: 19.99,      // Correct type
    categoryId: 1      // Correct naming
  }
});
```

#### ✅ Runtime Validation
```typescript
// Automatically validates input at runtime
try {
  await createProduct({
    request: {
      name: '', // Will throw validation error
      price: -5 // Will throw validation error
    }
  });
} catch (error) {
  console.error('Validation failed:', error);
}
```

#### ✅ IntelliSense & Autocomplete
Your IDE will provide full autocomplete and type hints for all generated functions and types.

#### ✅ Automatic Updates
When you change your Rust commands, just re-run the generator to get updated TypeScript bindings.

## Generated Files

The plugin generates several files in your output directory:

- **`types.ts`** - TypeScript interfaces for all command parameters and return types
- **`schemas.ts`** - Validation schemas (if validation library is specified)
- **`commands.ts`** - Strongly-typed command binding functions
- **`index.ts`** - Barrel export file

## API Reference

### CLI Commands

```bash
cargo tauri-typegen generate [OPTIONS]

OPTIONS:
    -p, --project-path <PATH>     Path to the Tauri project source directory
                                 [default: ./src-tauri]
    -o, --output-path <PATH>      Output path for generated TypeScript files  
                                 [default: ./src/generated]
    -v, --validation <LIBRARY>    Validation library to use
                                 [default: zod] [possible values: zod, yup, none]
        --verbose                Enable verbose output
    -h, --help                   Print help information
```

### Library Usage (Advanced)

For programmatic usage in build scripts:

```rust
use tauri_plugin_typegen::cli::{GenerateConfig, generate_from_config};

let config = GenerateConfig {
    project_path: "./src-tauri".to_string(),
    output_path: "./src/generated".to_string(),
    validation_library: "zod".to_string(),
};

let files = generate_from_config(&config)?;
```

## Configuration Options

### Validation Libraries

- **`zod`** - Generates Zod schemas with validation
- **`yup`** - Generates Yup schemas with validation  
- **`none`** - No validation schemas generated

### Type Mapping

The plugin automatically maps Rust types to TypeScript:

| Rust Type | TypeScript Type |
|-----------|----------------|
| `String`, `&str` | `string` |
| `i32`, `i64`, `f32`, `f64` | `number` |
| `bool` | `boolean` |
| `Option<T>` | `T \| null` |
| `Vec<T>` | `T[]` |
| `Result<T, E>` | `T` (error handling via Tauri) |

Custom structs are generated as TypeScript interfaces.

## Example Project Structure

```
my-tauri-app/
├── src-tauri/
│   ├── src/
│   │   ├── commands/
│   │   │   ├── user.rs      # Contains #[command] functions
│   │   │   └── mod.rs
│   │   └── lib.rs
│   └── Cargo.toml
├── src/
│   ├── generated/           # Generated by this plugin
│   │   ├── types.ts
│   │   ├── schemas.ts
│   │   ├── commands.ts
│   │   └── index.ts
│   └── App.tsx
└── package.json
```

## Development

### Testing with LibreFit Example

The plugin includes an example that demonstrates generating models for the LibreFit project:

```bash
cd examples/tauri-app
npm install
npm run tauri dev
```

Click "Analyze LibreFit Commands" to scan the LibreFit project and "Generate TypeScript Models" to create the bindings.

### Building the Plugin

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT License.