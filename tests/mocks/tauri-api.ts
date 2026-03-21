// Mock for @tauri-apps/api/event and @tauri-apps/api/core
export function listen(_event: string, _handler: Function) {
  return Promise.resolve(() => {});
}

export function emit(_event: string, _payload?: unknown) {
  return Promise.resolve();
}

export function invoke(_cmd: string, _args?: Record<string, unknown>) {
  return Promise.resolve(null);
}

export const event = { listen, emit };
export const core = { invoke };
export default { event, core, listen, emit, invoke };
