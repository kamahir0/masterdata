import { invoke as nativeInvoke } from "@tauri-apps/api/core";

type TraceEvent = { phase: string; path?: string; command?: string; at: number; ms?: number };
declare global { interface Window { __navigationTrace?: TraceEvent[] } }
// Only the Desktop evidence harness opts in. No production timer or observer runs.
export function navigationMark(phase: string, path?: string, ms?: number, command?: string) {
  window.__navigationTrace?.push({ phase, path, command, ms, at: performance.now() });
}
export async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!window.__navigationTrace || !["authoring_workspace", "open_data_file", "open_table_context", "open_type", "select_source"].includes(command))
    return nativeInvoke<T>(command, args);
  const started = performance.now(); const path = args?.relativePath as string | undefined;
  navigationMark("request-start", path, undefined, command);
  try { return await nativeInvoke<T>(command, args); }
  finally { navigationMark("ipc-return", path, performance.now() - started, command); }
}
export function navigationCommit(path: string | null) {
  if (!window.__navigationTrace || !path) return;
  navigationMark("react-commit", path);
  requestAnimationFrame(() => {
    navigationMark("paint-opportunity", path);
    if (document.querySelector(`.editor-area[data-active-source="${CSS.escape(path)}"] [role="gridcell"]`))
      navigationMark("grid-paint", path);
  });
}
