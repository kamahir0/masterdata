import fs from "node:fs/promises";
import path from "node:path";
import { randomUUID } from "node:crypto";
import { projectFileName, writeConfig } from "./config";
import { loadWorkspace, saveWorkspace, type Workspace } from "./files";
import { buildWorkspace, publishBuild } from "./build";
import {
  defaultConfig,
  type BuildResult,
  type SaveRequest,
  type Snapshot,
  type Config,
} from "../src/core/types";

// Only the native host chooses open/create/sample paths. The webview has no generic filesystem API.
export class ProjectService {
  private workspace: Workspace | null = null;
  private build: BuildResult | null = null;
  private writing = false;
  private building = false;
  private current(token: string) {
    if (!this.workspace || token !== this.workspace.token)
      throw new Error("Projectが切り替わっています。");
    return this.workspace;
  }
  private idle() {
    if (this.writing || this.building)
      throw new Error("保存・生成処理が完了するまでお待ちください。");
  }
  private async open(root: string) {
    this.idle();
    const loaded = await loadWorkspace(root);
    this.workspace = loaded.workspace;
    this.build = null;
    return loaded.project;
  }
  async request(method: string, args: any[]): Promise<unknown> {
    switch (method) {
      case "initial":
        return args[0] ? this.open(args[0]) : null;
      case "open":
        return this.open(args[0]);
      case "sample": {
        this.idle();
        const [source, userData] = args;
        const destination = path.join(
          userData,
          "workspaces",
          `Adventure-${randomUUID().slice(0, 8)}`,
        );
        await fs.cp(source, destination, { recursive: true });
        return this.open(path.join(destination, projectFileName));
      }
      case "create": {
        this.idle();
        const [name, parent] = args as [string, string];
        const folderName = name.trim();
        if (
          !folderName ||
          folderName === "." ||
          folderName === ".." ||
          /[<>:"/\\|?*\x00-\x1f]/.test(folderName)
        )
          throw new Error(
            "プロジェクト名にはフォルダ名に使える文字を入力してください。",
          );
        const destination = path.join(parent, folderName);
        await fs.mkdir(destination);
        await fs.mkdir(path.join(destination, "sources"));
        await fs.writeFile(
          path.join(destination, projectFileName),
          writeConfig(defaultConfig(folderName)),
        );
        return this.open(path.join(destination, projectFileName));
      }
      case "save": {
        if (this.writing) throw new Error("保存処理中です。");
        this.writing = true;
        try {
          const request = args[0] as SaveRequest;
          return await saveWorkspace(this.current(request.token), request);
        } finally {
          this.writing = false;
        }
      }
      case "build": {
        if (this.building) throw new Error("Build処理中です。");
        this.building = true;
        try {
          const [token, snapshot, profile, revision] = args as [
            string,
            Snapshot,
            string,
            number,
          ];
          const owner = this.current(token);
          const result = await buildWorkspace(
            owner,
            snapshot,
            profile,
            revision,
          );
          if (this.workspace?.token === owner.token) this.build = result;
          return result;
        } finally {
          this.building = false;
        }
      }
      case "publish": {
        const [token, targets] = args as [string, Config["publish"]];
        const ws = this.current(token);
        if (!this.build) throw new Error("先にBuildを実行してください。");
        this.idle();
        this.writing = true;
        try {
          return await publishBuild(this.build, ws.root, targets);
        } finally {
          this.writing = false;
        }
      }
      case "reveal": {
        const ws = this.current(args[0]);
        return args[1] && this.build ? this.build.directory : ws.root;
      }
      default:
        throw new Error("許可されていない操作です。");
    }
  }
}
