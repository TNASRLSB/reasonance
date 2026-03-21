// Mock for @tauri-apps/api/event, @tauri-apps/api/core, @tauri-apps/api/window
export function listen(_event: string, _handler: Function) {
  return Promise.resolve(() => {});
}

export function emit(_event: string, _payload?: unknown) {
  return Promise.resolve();
}

export function invoke(_cmd: string, _args?: Record<string, unknown>) {
  return Promise.resolve(null);
}

// Mock window object for @tauri-apps/api/window
export function getCurrentWindow() {
  return {
    minimize: () => Promise.resolve(),
    maximize: () => Promise.resolve(),
    toggleMaximize: () => Promise.resolve(),
    close: () => Promise.resolve(),
    show: () => Promise.resolve(),
    hide: () => Promise.resolve(),
    setTitle: (_title: string) => Promise.resolve(),
    innerSize: () => Promise.resolve({ width: 800, height: 600 }),
    outerSize: () => Promise.resolve({ width: 800, height: 600 }),
    isMaximized: () => Promise.resolve(false),
    isMinimized: () => Promise.resolve(false),
    isFocused: () => Promise.resolve(true),
    isVisible: () => Promise.resolve(true),
    onCloseRequested: (_handler: Function) => Promise.resolve(() => {}),
    onResized: (_handler: Function) => Promise.resolve(() => {}),
    onMoved: (_handler: Function) => Promise.resolve(() => {}),
    onFocusChanged: (_handler: Function) => Promise.resolve(() => {}),
  };
}

export const event = { listen, emit };
export const core = { invoke };
export default { event, core, listen, emit, invoke, getCurrentWindow };
