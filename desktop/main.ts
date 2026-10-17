import { app, BrowserWindow, Menu, dialog, ipcMain, shell } from "electron";
import path from "node:path";
import fs from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { stringify } from "yaml";
import { loadWorkspace, saveWorkspace, type Workspace } from "./files";
import { buildWorkspace, publishBuild } from "./build";
import {
  defaultConfig,
  type BuildResult,
  type Snapshot,
  type SaveRequest,
  type Config,
} from "../src/core/types";
let window: BrowserWindow;
let workspace: Workspace | null = null;
let build: BuildResult | null = null;
let dirty = false;
let writing = false;
let building = false;
const current = (token: string) => {
  if (!workspace || token !== workspace.token)
    throw new Error("Projectが切り替わっています。");
  return workspace;
};
function handle(name: string, callback: (...args: any[]) => any) {
  ipcMain.handle(name, (event, ...args) => {
    if (
      event.sender !== window.webContents ||
      event.senderFrame !== window.webContents.mainFrame
    )
      throw new Error("許可されていない呼出しです。");
    return callback(...args);
  });
}
async function openRoot(root: string) {
  if (writing || building)
    throw new Error("保存・生成処理が完了するまでお待ちください。");
  const loaded = await loadWorkspace(root);
  workspace = loaded.workspace;
  build = null;
  dirty = false;
  return loaded.project;
}
app.whenReady().then(async () => {
  window = new BrowserWindow({
    width: 1480,
    height: 960,
    minWidth: 1000,
    minHeight: 680,
    title: "MasterData",
    backgroundColor: "#f6f8fc",
    titleBarStyle: "hiddenInset",
    trafficLightPosition: { x: 18, y: 20 },
    webPreferences: {
      preload: path.join(__dirname, "preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });
  window.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  window.webContents.on("will-navigate", (event) => event.preventDefault());
  window.on("close", (event) => {
    if (dirty || writing || building) {
      event.preventDefault();
      window.webContents.send(
        "command",
        writing || building ? "busy-close" : "close",
      );
    }
  });
  const send = (command: string) => window.webContents.send("command", command);
  const template: Electron.MenuItemConstructorOptions[] = [
    ...(process.platform === "darwin"
      ? [
          {
            label: "MasterData",
            submenu: [
              { role: "about" },
              { type: "separator" },
              {
                label: "終了",
                accelerator: "Cmd+Q",
                click: () => window.close(),
              },
            ],
          } as Electron.MenuItemConstructorOptions,
        ]
      : []),
    {
      label: "ファイル",
      submenu: [
        {
          label: "新しいプロジェクト",
          accelerator: "CmdOrCtrl+Shift+N",
          click: () => send("new"),
        },
        {
          label: "開く…",
          accelerator: "CmdOrCtrl+O",
          click: () => send("open"),
        },
        {
          label: "保存",
          accelerator: "CmdOrCtrl+S",
          click: () => send("save"),
        },
        { type: "separator" },
        {
          label: "Build",
          accelerator: "CmdOrCtrl+B",
          click: () => send("build"),
        },
        { label: "閉じる", click: () => window.close() },
      ],
    },
    {
      label: "編集",
      submenu: [
        {
          label: "元に戻す",
          accelerator: "CmdOrCtrl+Z",
          click: () => send("undo"),
        },
        {
          label: "やり直す",
          accelerator: "CmdOrCtrl+Shift+Z",
          click: () => send("redo"),
        },
        { type: "separator" },
        { role: "cut" },
        { role: "copy" },
        { role: "paste" },
        { role: "selectAll" },
      ],
    },
    {
      label: "表示",
      submenu: [
        {
          label: "検索",
          accelerator: "CmdOrCtrl+F",
          click: () => send("find"),
        },
        {
          label: "問題",
          accelerator: "CmdOrCtrl+Shift+M",
          click: () => send("problems"),
        },
        { type: "separator" },
        { role: "resetZoom" },
        { role: "zoomIn" },
        { role: "zoomOut" },
        ...(app.isPackaged ? [] : [{ role: "toggleDevTools" as const }]),
      ],
    },
  ];
  Menu.setApplicationMenu(Menu.buildFromTemplate(template));
  handle("project:initial", async () => {
    const index = process.argv.indexOf("--project");
    return index >= 0 && process.argv[index + 1]
      ? openRoot(process.argv[index + 1])
      : null;
  });
  handle("project:open", async () => {
    const result = await dialog.showOpenDialog(window, {
      title: "MasterDataプロジェクトを開く",
      properties: ["openDirectory"],
    });
    return result.canceled ? null : openRoot(result.filePaths[0]);
  });
  handle("project:create", async (name: string) => {
    const result = await dialog.showSaveDialog(window, {
      title: "新しいプロジェクトの保存先",
      defaultPath: path.join(app.getPath("documents"), name || "MasterData"),
      buttonLabel: "プロジェクトを作成",
    });
    if (result.canceled || !result.filePath) return null;
    await fs.mkdir(result.filePath);
    await fs.mkdir(path.join(result.filePath, "sources"));
    await fs.writeFile(
      path.join(result.filePath, "masterdata.yaml"),
      stringify(defaultConfig(name)),
    );
    return openRoot(result.filePath);
  });
  handle("project:sample", async () => {
    const source = app.isPackaged
      ? path.join(process.resourcesPath, "sample")
      : path.join(__dirname, "..", "samples", "Adventure");
    const destination = path.join(
      app.getPath("userData"),
      "workspaces",
      `Adventure-${randomUUID().slice(0, 8)}`,
    );
    await fs.cp(source, destination, { recursive: true });
    return openRoot(destination);
  });
  handle("project:save", async (request: SaveRequest) => {
    if (writing) throw new Error("保存処理中です。");
    writing = true;
    try {
      return await saveWorkspace(current(request.token), request);
    } finally {
      writing = false;
    }
  });
  handle(
    "project:build",
    async (
      token: string,
      snapshot: Snapshot,
      profile: string,
      revision: number,
    ) => {
      if (building) throw new Error("Build処理中です。");
      building = true;
      try {
        const owner = current(token);
        const result = await buildWorkspace(owner, snapshot, profile, revision);
        if (workspace?.token === owner.token) build = result;
        return result;
      } finally {
        building = false;
      }
    },
  );
  handle(
    "project:publish",
    async (token: string, targets: Config["publish"]) => {
      const ws = current(token);
      if (!build) throw new Error("先にBuildを実行してください。");
      if (writing || building) throw new Error("処理中です。");
      writing = true;
      try {
        return await publishBuild(build, ws.root, targets);
      } finally {
        writing = false;
      }
    },
  );
  handle("project:reveal", async (token: string, output = false) => {
    const ws = current(token);
    const error = await shell.openPath(
      output && build ? build.directory : ws.root,
    );
    if (error) throw new Error(error);
  });
  handle("project:close", () => {
    dirty = false;
    window.close();
  });
  ipcMain.on("project:dirty", (event, value) => {
    if (event.sender === window.webContents) {
      dirty = !!value;
      window.setDocumentEdited(dirty);
    }
  });
  if (process.env.MASTERDATA_DEV_URL)
    await window.loadURL(process.env.MASTERDATA_DEV_URL);
  else await window.loadFile(path.join(__dirname, "..", "dist", "index.html"));
});
app.on("window-all-closed", () => app.quit());
