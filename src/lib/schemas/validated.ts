import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { z } from 'zod';

export async function validatedInvoke<T>(
  command: string,
  args: Record<string, unknown>,
  schema: z.ZodType<T>,
): Promise<T> {
  const raw = await invoke(command, args);
  const result = schema.safeParse(raw);
  if (!result.success) {
    console.error(`[validatedInvoke] ${command} validation failed:`, result.error.format());
    throw new Error(`Invalid response from ${command}: ${result.error.message}`);
  }
  return result.data;
}

export function validatedListen<T>(
  event: string,
  schema: z.ZodType<T>,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  return listen(event, (e) => {
    const result = schema.safeParse(e.payload);
    if (!result.success) {
      console.warn(`[validatedListen] ${event} validation failed:`, result.error.format());
      return;
    }
    handler(result.data);
  });
}
