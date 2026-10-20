import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { encode, decode } from "./core/wire";
import type { DesktopAPI } from "./core/types";
const request = async (method: string, args: unknown[] = []) =>
  decode(
    await invoke<string>("project_request", { method, payload: encode(args) }),
  );
// Suppress only the WebView default action; custom menus still receive the event.
// Install for every renderer surface (including portals), not individual screens.
document.addEventListener(
  "contextmenu",
  (event) => event.preventDefault(),
  true,
);
let zoom = 1;
export const desktop: DesktopAPI = {
  initial: () => request("initial"),
  close: () => invoke("close_project"),
  open: () => request("open"),
  create: (name) => request("create", [name]),
  sample: () => request("sample"),
  save: (value) => request("save", [value]),
  build: (...args) => request("build", args),
  publish: (...args) => request("publish", args),
  reveal: (...args) => request("reveal", args),
  dirty: (value) => {
    void invoke("set_dirty", { value });
  },
  onCommand: (callback) => {
    let disposed = false;
    const pending = listen<string>("command", ({ payload }) => {
      if (disposed) return;
      if (payload.startsWith("zoom-")) {
        zoom =
          payload === "zoom-reset"
            ? 1
            : Math.max(
                0.5,
                Math.min(2, zoom + (payload === "zoom-in" ? 0.1 : -0.1)),
              );
        void getCurrentWebview().setZoom(zoom);
      } else callback(payload);
    });
    return () => {
      disposed = true;
      void pending.then((stop) => stop());
    };
  },
};
// Browser regression injects the same API through a test-only bridge before loading this module.
if ("__TAURI_INTERNALS__" in window) {
  window.desktop = desktop;
  document.addEventListener("mousedown", (event) => {
    const target = event.target as HTMLElement;
    if (
      event.button === 0 &&
      target.closest(".titlebar") &&
      !target.closest("button, input, .titlebar-controls")
    ) {
      if (event.detail === 2) void getCurrentWindow().toggleMaximize();
      else void getCurrentWindow().startDragging();
    }
  });
}
