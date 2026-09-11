import { vi } from 'vitest';
Object.defineProperty(window, 'matchMedia', { value: () => ({ matches: false, addListener() {}, removeListener() {}, addEventListener() {}, removeEventListener() {} }) });
globalThis.ResizeObserver = class { observe() {} unobserve() {} disconnect() {} };
Object.defineProperty(globalThis, 'CSS', { value: { escape: (value: string) => value.replace(/[^a-zA-Z0-9_-]/g, '\\$&') } });
const original = window.getComputedStyle;
window.getComputedStyle = (element) => original(element);
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => ({ onCloseRequested: async () => () => {}, destroy: vi.fn() }) }));
