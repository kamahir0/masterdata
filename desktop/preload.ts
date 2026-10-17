import { contextBridge, ipcRenderer } from "electron";
contextBridge.exposeInMainWorld("desktop", {
  initial: () => ipcRenderer.invoke("project:initial"),
  close: () => ipcRenderer.invoke("project:close"),
  open: () => ipcRenderer.invoke("project:open"),
  create: (name: string) => ipcRenderer.invoke("project:create", name),
  sample: () => ipcRenderer.invoke("project:sample"),
  save: (request: unknown) => ipcRenderer.invoke("project:save", request),
  build: (...args: unknown[]) => ipcRenderer.invoke("project:build", ...args),
  publish: (...args: unknown[]) =>
    ipcRenderer.invoke("project:publish", ...args),
  reveal: (...args: unknown[]) => ipcRenderer.invoke("project:reveal", ...args),
  dirty: (value: boolean) => ipcRenderer.send("project:dirty", value),
  onCommand: (callback: (command: string) => void) => {
    const listener = (_: unknown, command: string) => callback(command);
    ipcRenderer.on("command", listener);
    return () => ipcRenderer.removeListener("command", listener);
  },
});
