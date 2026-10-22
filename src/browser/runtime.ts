import type { DesktopAPI, LoadedProject } from "../core/types";
import { type Directory, permission } from "./filesystem";
import { BrowserProject, RecoveryRequired, load, recover } from "./project";
import {
  clearRecentDirectory,
  getRecentDirectory,
  saveRecentDirectory,
} from "./storage";

declare global {
  interface Window {
    showDirectoryPicker?: (options: {
      mode: "readwrite";
      id: string;
    }) => Promise<Directory>;
  }
}
export const isBrowser = () => window.desktop?.platform === "browser";
export const browserSupport = () =>
  window.isSecureContext &&
  !!window.showDirectoryPicker &&
  typeof FileSystemFileHandle !== "undefined" &&
  "createWritable" in FileSystemFileHandle.prototype &&
  typeof FileSystemDirectoryHandle !== "undefined" &&
  "queryPermission" in FileSystemDirectoryHandle.prototype &&
  "requestPermission" in FileSystemDirectoryHandle.prototype &&
  "values" in FileSystemDirectoryHandle.prototype &&
  !!navigator.locks;
const unsupported = async (): Promise<never> => {
  throw new Error("ブラウザ版ではこの操作を提供していません。");
};
interface Status {
  busy: boolean;
  recovery?: RecoveryRequired;
  recoveryLive?: boolean;
  notice?: string;
  recent?: {
    name: string;
    handle: Directory;
  };
}
let status: Status = { busy: false };
const listeners = new Set<() => void>();
const update = (value: Partial<Status>) => {
  status = { ...status, ...value };
  listeners.forEach((fn) => fn());
};
export const browserStatus = {
  subscribe(fn: () => void) {
    listeners.add(fn);
    return () => {
      listeners.delete(fn);
    };
  },
  get: () => status,
};
let project: BrowserProject | undefined;
let dirty = false;
// One origin-wide lock deliberately serializes even differently-named handles
// to the same project. Handle names and user-visible root labels are not IDs.
const locked = <T>(fn: () => Promise<T>) =>
  navigator.locks.request("masterdata:filesystem", fn);
async function perform<T>(fn: () => Promise<T>): Promise<T> {
  if (status.busy)
    throw new Error("保存・読込・復旧処理の完了をお待ちください。");
  update({ busy: true });
  try {
    return await fn();
  } catch (error) {
    if (error instanceof RecoveryRequired) {
      const live = project
        ? await project.root.isSameEntry(error.root).catch(() => true)
        : false;
      update({ recovery: error, recoveryLive: live });
    }
    throw error;
  } finally {
    update({ busy: false });
  }
}
export async function resolveRecovery(adopt: boolean) {
  const recovery = status.recovery;
  if (!recovery) return;
  if (status.busy) throw new Error("復旧処理中です。");
  if (adopt && status.recoveryLive)
    throw new Error(
      "入力を保持するため、保存前へ復旧してから再保存してください。",
    );
  // requestPermission starts in the click handler, before lock / disk awaits.
  const allowed = permission(recovery.root);
  await perform(async () => {
    await allowed;
    await locked(() => recover(recovery, adopt));
    // Rollback verified every journal path. Keep the original memory baseline:
    // the next Save still compares each changed path against disk. Replacing it
    // with the current disk state would authorize overwriting external changes.
    update({
      recovery: undefined,
      notice: status.recoveryLive
        ? "保存前へ復旧しました。入力とUndoは保持しています。内容を確認して再保存できます。"
        : "復旧を確認しました。プロジェクトを開いて内容を確認してください。",
    });
  });
}
export async function reauthorizeAccess() {
  if (status.busy) throw new Error("保存・復旧処理中です。");
  const recovery = status.recovery;
  const owner = recovery?.root ?? project?.root;
  if (!owner) throw new Error("プロジェクトが開かれていません。");
  // A denied permission may stay denied without another prompt. A new picker
  // grant is explicit, and handle identity prevents a same-name folder from
  // redirecting an existing draft or its recovery writes somewhere else.
  const picked = window.showDirectoryPicker!({
    mode: "readwrite",
    id: "masterdata-project",
  });
  return perform(async () => {
    let selected: Directory;
    try {
      selected = await picked;
    } catch (error) {
      if (error instanceof Error && error.name === "AbortError") return false;
      throw error;
    }
    if (!(await owner.isSameEntry(selected)))
      throw new Error(
        `元と同じフォルダ「${owner.name}」を選択してください。入力は保持しています。`,
      );
    await permission(selected);
    if (recovery) recovery.root = selected;
    if (project && (await project.root.isSameEntry(selected))) {
      project.root = selected;
      await saveRecentDirectory(selected);
    }
    return true;
  });
}
export async function openRecent(): Promise<LoadedProject | null> {
  if (status.busy) throw new Error("保存・復旧処理中です。");
  const recent = status.recent;
  if (!recent) return null;
  const { handle } = recent;
  const allowed = await handle
    .requestPermission({ mode: "readwrite" })
    .catch(() => "denied" as PermissionState);
  if (allowed !== "granted") return null;

  return perform(async (): Promise<LoadedProject | null> => {
    try {
      const value = await locked(() => load(handle));
      project = new BrowserProject(handle, value);
      await saveRecentDirectory(handle);
      update({ recovery: undefined, notice: undefined, recent: undefined });
      return value;
    } catch (error) {
      if (
        error instanceof Error &&
        (error.name === "NotFoundError" ||
          error.message.includes("masterdata.toml がありません"))
      ) {
        await clearRecentDirectory();
        update({ recent: undefined });
      }
      throw error;
    }
  });
}
export const browser: DesktopAPI = {
  platform: "browser",
  initial: async () => {
    if (!browserSupport()) return null;
    const handle = await getRecentDirectory();
    if (!handle) return null;
    try {
      const state = await handle.queryPermission({ mode: "readwrite" });
      if (state === "granted") {
        return await perform(async () => {
          try {
            const value = await locked(() => load(handle));
            project = new BrowserProject(handle, value);
            update({
              recovery: undefined,
              notice: undefined,
              recent: undefined,
            });
            return value;
          } catch (error) {
            if (error instanceof RecoveryRequired) {
              throw error;
            }
            if (
              error instanceof Error &&
              (error.name === "NotFoundError" ||
                error.message.includes("masterdata.toml がありません"))
            ) {
              await clearRecentDirectory();
            }
            throw error;
          }
        });
      } else {
        update({ recent: { name: handle.name, handle } });
        return null;
      }
    } catch (error) {
      if (error instanceof RecoveryRequired) {
        return null;
      }
      await clearRecentDirectory();
      return null;
    }
  },
  open: async () => {
    if (!browserSupport())
      throw new Error(
        "この環境ではローカルフォルダを編集できません。デスクトップ版ChromeとHTTPS（またはlocalhost）で開いてください。",
      );
    if (status.busy)
      throw new Error("保存・読込・復旧処理の完了をお待ちください。");
    // Picker must be invoked in the user's activation, before any filesystem IO.
    const picked = window.showDirectoryPicker!({
      mode: "readwrite",
      id: "masterdata-project",
    });
    return perform(async (): Promise<LoadedProject | null> => {
      let root: Directory;
      try {
        root = await picked;
      } catch (error) {
        if (error instanceof Error && error.name === "AbortError") return null;
        throw error;
      }
      const value = await locked(() => load(root));
      project = new BrowserProject(root, value);
      await saveRecentDirectory(root);
      update({ recovery: undefined, notice: undefined, recent: undefined });
      return value;
    });
  },
  save: (request) => {
    if (status.busy) return Promise.reject(new Error("保存・復旧処理中です。"));
    if (!project)
      return Promise.reject(new Error("プロジェクトが開かれていません。"));
    const current = project;
    const allowed = permission(current.root);
    return perform(async () => {
      await allowed;
      const result = await locked(() => current.save(request));
      update({ notice: undefined });
      return result;
    });
  },
  create: unsupported,
  sample: unsupported,
  build: unsupported,
  publish: unsupported,
  reveal: unsupported,
  close: unsupported,
  dirty(value) {
    dirty = value;
  },
  onCommand(callback) {
    const handler = (event: KeyboardEvent) => {
      if (
        event.defaultPrevented ||
        event.isComposing ||
        event.altKey ||
        !(event.metaKey || event.ctrlKey)
      )
        return;
      const key = event.key.toLowerCase();
      const input =
        event.target instanceof Element &&
        event.target.closest("input, textarea, [contenteditable=true]");
      const command =
        key === "s"
          ? event.shiftKey
            ? "save-all"
            : "save"
          : key === "o"
            ? "open"
            : !input && key === "z"
              ? event.shiftKey
                ? "redo"
                : "undo"
              : !input && key === "f"
                ? "find"
                : "";
      if (command) {
        event.preventDefault();
        callback(command);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  },
};
export function installBrowser() {
  if (window.desktop || "__TAURI_INTERNALS__" in window) return;
  window.desktop = browser;
  window.addEventListener("beforeunload", (event) => {
    if (dirty || status.busy || status.recovery) {
      event.preventDefault();
      event.returnValue = "";
    }
  });
}
