import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invokeBrowser } from "./browser-host";

export const isBrowserHost = import.meta.env.VITE_MASTERDATA_WEB === "1";

export async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (isBrowserHost) {
    return invokeBrowser<T>(command, args);
  }
  return tauriInvoke<T>(command, args);
}

export async function closeWindow(): Promise<void> {
  if (isBrowserHost) {
    window.close();
    return;
  }
  await getCurrentWindow().destroy();
}

export function onCloseRequested(handler: (event: { preventDefault: () => void }) => void): Promise<() => void> {
  if (isBrowserHost) return Promise.resolve(() => {});
  return getCurrentWindow().onCloseRequested(handler);
}
