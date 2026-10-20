import fs from "node:fs/promises";
import path from "node:path";
import { randomUUID, createHash } from "node:crypto";
import { projectFileName, readConfig, writeConfig } from "./config";
import { parseSource } from "../src/core/yaml";
import {
  type Config,
  type LoadedProject,
  type SaveRequest,
  type SaveResult,
  type SavedFile,
} from "../src/core/types";
export const digest = (text: string) =>
  createHash("sha256").update(text).digest("hex");
export interface Workspace {
  root: string;
  token: string;
  baseline: SavedFile[];
  configText: string;
  folders: string[];
}
const safeName = (name: string) =>
  !path.isAbsolute(name) &&
  name
    .split(/[\\/]/)
    .every((p) => !!p && p !== ".." && p !== "." && !/[\x00-\x1f]/.test(p));
export async function safePath(root: string, relative: string) {
  if (!safeName(relative)) throw new Error("Project外のパスは使用できません。");
  const destination = path.join(root, relative);
  let current = root;
  for (const part of relative.split("/")) {
    current = path.join(current, part);
    try {
      if ((await fs.lstat(current)).isSymbolicLink())
        throw new Error("シンボリックリンクは変更しません。");
    } catch (error: any) {
      if (error.code !== "ENOENT") throw error;
    }
  }
  return destination;
}
export async function optionalRead(file: string) {
  try {
    return await fs.readFile(file, "utf8");
  } catch (error: any) {
    if (error.code === "ENOENT") return null;
    throw error;
  }
}
export async function loadWorkspace(
  input: string,
): Promise<{ workspace: Workspace; project: LoadedProject }> {
  let root: string;
  let configText: string;
  let config: Config;
  try {
    if (
      path.basename(input) !== projectFileName ||
      !(await fs.lstat(input)).isFile()
    )
      throw new Error(`${projectFileName} ファイルを選択してください。`);
    root = await fs.realpath(path.dirname(input));
    configText = await fs.readFile(
      await safePath(root, projectFileName),
      "utf8",
    );
    config = readConfig(configText);
  } catch (error) {
    throw new Error(
      `プロジェクトを開けません: ${input}\n${error instanceof Error ? error.message : String(error)}`,
    );
  }
  const sources: LoadedProject["snapshot"]["sources"] = [];
  const saved: SavedFile[] = [];
  const folders: string[] = [];
  let reading = 0;
  const readers: (() => void)[] = [];
  async function readSource(file: string) {
    if (reading >= 16)
      await new Promise<void>((resolve) => readers.push(resolve));
    else reading++;
    try {
      return await fs.readFile(file, "utf8");
    } finally {
      const next = readers.shift();
      if (next) next();
      else reading--;
    }
  }
  async function walk(relative = "") {
    const directory = path.join(root, "sources", relative);
    let entries;
    try {
      entries = await fs.readdir(directory, { withFileTypes: true });
    } catch (error: any) {
      if (error.code === "ENOENT") return;
      throw error;
    }
    await Promise.all(
      entries.map(async (entry) => {
        if (entry.isSymbolicLink() || entry.name.startsWith(".")) return;
        const name = [relative, entry.name].filter(Boolean).join("/");
        if (entry.isDirectory()) {
          folders.push(name);
          await walk(name);
        } else if (/\.ya?ml$/i.test(entry.name)) {
          const text = await readSource(path.join(directory, entry.name));
          const source = parseSource(name, text);
          sources.push(source);
          saved.push({ id: source.id, path: name, text });
        }
      }),
    );
  }
  await walk();
  sources.sort((a, b) => a.path.localeCompare(b.path));
  folders.sort();
  const token = randomUUID();
  const workspace: Workspace = {
    root,
    token,
    baseline: saved,
    configText,
    folders,
  };
  return {
    workspace,
    project: {
      root,
      token,
      snapshot: { config, sources, folders },
      saved,
      configText,
    },
  };
}
export async function saveWorkspace(
  workspace: Workspace,
  request: SaveRequest,
): Promise<SaveResult> {
  const desired = new Map(request.files.map((f) => [f.path, f]));
  if (
    desired.size !== request.files.length ||
    new Set(request.files.map((f) => f.id)).size !== request.files.length
  )
    throw new Error("保存先のパスが重複しています。");
  const old = new Map(workspace.baseline.map((f) => [f.path, f]));
  const names = new Set([...old.keys(), ...desired.keys()]);
  const changes: {
    file: string;
    before: string | null;
    after: string | null;
  }[] = [];
  for (const name of names) {
    const file = await safePath(workspace.root, `sources/${name}`);
    const before = old.get(name)?.text ?? null;
    const after = desired.get(name)?.text ?? null;
    if (before === after) continue;
    const current = await optionalRead(file);
    if (current !== before)
      throw new Error(
        `外部で変更されています: ${name}。保存を中止しました。現在の入力は保持しています。`,
      );
    changes.push({ file, before, after });
  }
  const configText = writeConfig(request.config, workspace.configText);
  const configFile = await safePath(workspace.root, projectFileName);
  if (configText !== workspace.configText) {
    if ((await optionalRead(configFile)) !== workspace.configText)
      throw new Error("プロジェクト設定が外部で変更されています。");
    changes.push({
      file: configFile,
      before: workspace.configText,
      after: configText,
    });
  }
  const directories = await Promise.all(
    request.folders.map((f) => safePath(workspace.root, `sources/${f}`)),
  );
  const transaction = await safePath(
    workspace.root,
    `.masterdata/transactions/${randomUUID()}`,
  );
  await fs.mkdir(transaction, { recursive: true });
  await fs.writeFile(
    path.join(transaction, "manifest.json"),
    JSON.stringify(
      changes.map((c, i) => ({
        path: path.relative(workspace.root, c.file),
        before: c.before === null ? null : `${i}.before`,
        after: c.after === null ? null : `${i}.after`,
      })),
      null,
      2,
    ),
  );
  for (const [i, change] of changes.entries()) {
    if (change.before !== null)
      await fs.writeFile(path.join(transaction, `${i}.before`), change.before);
    if (change.after !== null)
      await fs.writeFile(path.join(transaction, `${i}.after`), change.after);
  }
  const applied: typeof changes = [];
  try {
    for (const dir of directories) await fs.mkdir(dir, { recursive: true });
    for (const change of changes) {
      // Check again immediately before replacement; cached discovery is never write authorization.
      if ((await optionalRead(change.file)) !== change.before)
        throw new Error(`保存中に外部変更を検出しました: ${change.file}`);
      await fs.mkdir(path.dirname(change.file), { recursive: true });
      if (change.after === null) await fs.unlink(change.file);
      else {
        const temporary = `${change.file}.${randomUUID()}.tmp`;
        await fs.writeFile(temporary, change.after, { flag: "wx" });
        await fs.rename(temporary, change.file);
      }
      applied.push(change);
    }
  } catch (error) {
    let recoveryFailed = false;
    for (const change of applied.reverse()) {
      try {
        if ((await optionalRead(change.file)) !== change.after) {
          recoveryFailed = true;
          continue;
        }
        if (change.before === null) await fs.unlink(change.file);
        else await fs.writeFile(change.file, change.before);
      } catch {
        recoveryFailed = true;
      }
    }
    throw new Error(
      `${String(error)}${recoveryFailed ? `\n復旧情報を保持しました: ${transaction}` : "\n今回の書込みは元に戻しました。"}`,
    );
  }
  for (const name of [...workspace.folders].sort(
    (a, b) => b.length - a.length,
  )) {
    if (!request.folders.includes(name)) {
      try {
        await fs.rmdir(await safePath(workspace.root, `sources/${name}`));
      } catch {
        /* Non-empty folders include user files and are intentionally preserved. */
      }
    }
  }
  await fs.writeFile(
    path.join(transaction, "complete"),
    new Date().toISOString(),
  );
  workspace.baseline = request.files;
  workspace.configText = configText;
  workspace.folders = request.folders;
  return { files: request.files, configText };
}
