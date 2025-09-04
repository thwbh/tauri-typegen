import { invoke } from '@tauri-apps/api/core';
import * as schemas from './schemas';
import type * as types from './types';

// Auto-generated command bindings

export async function greet(params: types.GreetParams): Promise<string> {
  const validatedParams = schemas.GreetParamsSchema.parse(params);
  return invoke('greet', validatedParams);
}

