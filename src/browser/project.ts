import { readConfig, writeConfig } from "../core/config";
import { parseSource } from "../core/yaml";
import {
  uid,
  type LoadedProject,
  type SaveRequest,
  type SaveResult,
} from "../core/types";
import {
  type Directory,
  directory,
  parent,
  read,
  write,
  entries,
  exists,
  missing,
  parts,
} from "./filesystem";

const journalRoot = ".masterdata/browser-transactions";
interface Change {
  path: string;
  before: string | null;
  after: string | null;
}
interface FolderChange {
  path: string;
  before: boolean;
  after: boolean;
}
export interface Journal {
  version: 1;
  files: Change[];
  folders: FolderChange[];
}
export class RecoveryRequired extends Error {
  constructor(
    public root: Directory,
    public path: string,
    public raw: string,
    public journal?: Journal,
  ) {
    super(`中断した保存があります: ${path}。復旧情報を確認してください。`);
  }
}
function sourcePath(path: string) {
  sourceParts(path);
  if (!/\.ya?ml$/i.test(path))
    throw new Error(`YAML以外は保存できません: ${path}`);
  return `sources/${path}`;
}
function sourceParts(path: string) {
  const names = parts(path);
  // Native discovery also ignores dot entries. Accepting a new hidden draft at
  // Save would report success and then make it disappear on the next Open.
  if (names.some((name) => name.startsWith(".")))
    throw new Error(
      `再オープンで読み込めない名前です: ${path}。先頭の . を変更してください。`,
    );
  return names;
}
function validJournal(raw: string): Journal {
  const j = JSON.parse(raw);
  if (j.version !== 1 || !Array.isArray(j.files) || !Array.isArray(j.folders))
    throw new Error("復旧情報の形式が不正です。");
  const paths = new Set<string>();
  for (const op of [...j.files, ...j.folders]) {
    if (typeof op.path !== "string") throw new Error("復旧pathが不正です。");
    parts(op.path);
    if (paths.has(op.path)) throw new Error("復旧pathが重複しています。");
    paths.add(op.path);
  }
  for (const op of j.files) {
    if (op.path !== "masterdata.toml") {
      if (!op.path.startsWith("sources/"))
        throw new Error("復旧pathが範囲外です。");
      sourcePath(op.path.slice(8));
    }
    if (
      ![op.before, op.after].every(
        (v) => v === null || typeof v === "string",
      ) ||
      op.before === op.after
    )
      throw new Error("復旧fileの形式が不正です。");
    if (op.path === "masterdata.toml") {
      if (op.before === null || op.after === null)
        throw new Error("設定fileの削除は許可されていません。");
      readConfig(op.before);
      readConfig(op.after);
    }
  }
  for (const op of j.folders) {
    if (
      (op.path !== "sources" && !op.path.startsWith("sources/")) ||
      typeof op.before !== "boolean" ||
      typeof op.after !== "boolean" ||
      op.before === op.after
    )
      throw new Error("復旧folderの形式が不正です。");
  }
  return j;
}
export async function pending(root: Directory) {
  let list;
  try {
    list = await entries(root, journalRoot);
  } catch (error) {
    if (missing(error)) return;
    throw error;
  }
  // Any unexpected/corrupt entry blocks writing rather than being silently ignored.
  for (const entry of list) {
    const path = `${journalRoot}/${entry.name}`;
    const raw = entry.kind === "file" ? ((await read(root, path)) ?? "") : "";
    let journal;
    try {
      journal = validJournal(raw);
    } catch {
      /* Present raw data for manual recovery. */
    }
    throw new RecoveryRequired(root, path, raw, journal);
  }
}
export async function load(root: Directory): Promise<LoadedProject> {
  await pending(root);
  const configText = await read(root, "masterdata.toml");
  if (configText === null)
    throw new Error(
      `${root.name}: masterdata.toml がありません。プロジェクトのルートフォルダを選択してください。`,
    );
  let config;
  try {
    config = readConfig(configText);
  } catch (error) {
    throw new Error(`${root.name}/masterdata.toml: ${String(error)}`);
  }
  const sources: LoadedProject["snapshot"]["sources"] = [];
  const folders: string[] = [];
  const saved: LoadedProject["saved"] = [];
  async function walk(path = "") {
    let list;
    try {
      list = await entries(root, ["sources", path].filter(Boolean).join("/"));
    } catch (error) {
      if (!path && missing(error)) return;
      throw error;
    }
    for (const entry of list) {
      if (entry.name.startsWith(".")) continue;
      const name = [path, entry.name].filter(Boolean).join("/");
      parts(name);
      if (entry.kind === "directory") {
        folders.push(name);
        await walk(name);
      } else if (/\.ya?ml$/i.test(name)) {
        const text = await read(root, sourcePath(name));
        if (text === null)
          throw new Error(`読込中にfileが削除されました: ${name}`);
        const source = parseSource(name, text);
        sources.push(source);
        saved.push({ id: source.id, path: name, text });
      }
    }
  }
  await walk();
  sources.sort((a, b) => a.path.localeCompare(b.path));
  saved.sort((a, b) => a.path.localeCompare(b.path));
  folders.sort();
  return {
    root: root.name,
    token: uid(),
    configText,
    snapshot: { config, sources, folders },
    saved,
  };
}
export async function inspect(recovery: RecoveryRequired) {
  if (!recovery.journal)
    return [
      {
        path: recovery.path,
        state:
          "復旧情報が破損しています。JSONを退避し、手動で確認してください。",
      },
    ];
  return Promise.all(
    [...recovery.journal.files, ...recovery.journal.folders].map(async (op) => {
      try {
        const value =
          typeof op.before === "boolean"
            ? await exists(recovery.root, op.path)
            : await read(recovery.root, op.path);
        return {
          path: op.path,
          state:
            value === op.after
              ? "保存予定"
              : value === op.before
                ? "変更前"
                : "外部変更",
        };
      } catch {
        return { path: op.path, state: "読込不能" };
      }
    }),
  );
}
async function verify(
  root: Directory,
  journal: Journal,
  side: "before" | "after",
) {
  for (const op of journal.files)
    if ((await read(root, op.path)) !== op[side])
      throw new Error(`内容を確認できません: ${op.path}`);
  for (const op of journal.folders)
    if ((await exists(root, op.path)) !== op[side])
      throw new Error(`folderを確認できません: ${op.path}`);
}
async function rollback(root: Directory, journal: Journal) {
  // Check the entire set before restoring anything. A third value belongs to an
  // external writer, even if our previous close threw after replacing a file.
  for (const op of journal.files) {
    const value = await read(root, op.path);
    if (value !== op.before && value !== op.after)
      throw new Error(`外部変更を保護しました: ${op.path}`);
  }
  for (const op of journal.folders
    .filter((f) => f.before)
    .sort((a, b) => a.path.length - b.path.length))
    await directory(root, op.path, true);
  for (const op of [...journal.files].reverse()) {
    const value = await read(root, op.path);
    if (value === op.before) continue;
    if (value !== op.after)
      throw new Error(`復旧中の外部変更を保護しました: ${op.path}`);
    await write(root, op.path, op.before);
  }
  for (const op of journal.folders
    .filter((f) => !f.before)
    .sort((a, b) => b.path.length - a.path.length)) {
    if (await exists(root, op.path)) {
      const { dir, name } = await parent(root, op.path);
      await dir.removeEntry(name);
    }
  }
  await verify(root, journal, "before");
}
export async function recover(recovery: RecoveryRequired, adopt: boolean) {
  if (
    !recovery.journal ||
    (await read(recovery.root, recovery.path)) !== recovery.raw
  )
    throw new Error(
      "復旧情報を再確認できません。フォルダを再選択してください。",
    );
  if (adopt) await verify(recovery.root, recovery.journal, "after");
  else await rollback(recovery.root, recovery.journal);
  await write(recovery.root, recovery.path, null);
}
export class BrowserProject {
  constructor(
    public root: Directory,
    public base: LoadedProject,
  ) {}
  async save(request: SaveRequest): Promise<SaveResult> {
    if (request.token !== this.base.token)
      throw new Error(
        "保存基準を読み直すため、プロジェクトを再オープンしてください。メモリの入力は保持しています。",
      );
    await pending(this.root);
    const before = new Map(
      this.base.saved.map((f) => [sourcePath(f.path), f.text]),
    );
    const after = new Map(
      request.files.map((f) => [sourcePath(f.path), f.text]),
    );
    if (
      after.size !== request.files.length ||
      new Set(request.files.map((f) => f.id)).size !== request.files.length
    )
      throw new Error("保存先のパスまたはfile IDが重複しています。");
    const configText = writeConfig(request.config, this.base.configText);
    before.set("masterdata.toml", this.base.configText);
    after.set("masterdata.toml", configText);
    const journal: Journal = { version: 1, files: [], folders: [] };
    for (const path of new Set([...before.keys(), ...after.keys()])) {
      const old = before.get(path) ?? null,
        next = after.get(path) ?? null;
      if (old !== next) journal.files.push({ path, before: old, after: next });
    }
    const desired = new Set<string>(["sources"]);
    const addParents = (path: string) => {
      const names = parts(path);
      names.forEach((_, i) => desired.add(names.slice(0, i + 1).join("/")));
    };
    request.folders.forEach((f) => {
      sourceParts(f);
      addParents(`sources/${f}`);
    });
    request.files.forEach((f) =>
      addParents(sourcePath(f.path).split("/").slice(0, -1).join("/")),
    );
    const original = new Set(
      this.base.snapshot.folders.map((f) => `sources/${f}`),
    );
    if (await exists(this.root, "sources")) original.add("sources");
    for (const path of new Set([...original, ...desired])) {
      const was = original.has(path),
        next = desired.has(path);
      if (was !== next)
        journal.folders.push({ path, before: was, after: next });
    }
    await verify(this.root, journal, "before");
    const removed = new Set([
      ...journal.files.filter((f) => f.after === null).map((f) => f.path),
      ...journal.folders.filter((f) => !f.after).map((f) => f.path),
    ]);
    for (const folder of journal.folders.filter((f) => !f.after))
      for (const entry of await entries(this.root, folder.path))
        if (!removed.has(`${folder.path}/${entry.name}`))
          throw new Error(
            `保存対象外の項目があるためfolderを削除できません: ${folder.path}/${entry.name}`,
          );
    if (!journal.files.length && !journal.folders.length)
      return { files: request.files, configText };
    await directory(this.root, journalRoot, true);
    const path = `${journalRoot}/${uid()}.json`;
    const raw = JSON.stringify(journal);
    // Durability barrier: no source mutation before close + reread of originals.
    await write(this.root, path, raw);
    if ((await read(this.root, path)) !== raw)
      throw new Error(`復旧情報の検証に失敗しました: ${path}`);
    try {
      await verify(this.root, journal, "before");
      for (const folder of journal.folders
        .filter((f) => f.after)
        .sort((a, b) => a.path.length - b.path.length)) {
        if (await exists(this.root, folder.path))
          throw new Error(`保存中に同名folderが作成されました: ${folder.path}`);
        await directory(this.root, folder.path, true);
      }
      // Copy renamed files before deleting originals. Deletions are never recursive.
      for (const op of [
        ...journal.files.filter((f) => f.after !== null),
        ...journal.files.filter((f) => f.after === null),
      ]) {
        if ((await read(this.root, op.path)) !== op.before)
          throw new Error(`保存中に外部変更を検出しました: ${op.path}`);
        await write(this.root, op.path, op.after);
      }
      for (const folder of journal.folders
        .filter((f) => !f.after)
        .sort((a, b) => b.path.length - a.path.length)) {
        const { dir, name } = await parent(this.root, folder.path);
        await dir.removeEntry(name);
      }
      await verify(this.root, journal, "after");
      await write(this.root, path, null);
    } catch (error) {
      try {
        await rollback(this.root, journal);
        await write(this.root, path, null);
      } catch {
        throw new RecoveryRequired(this.root, path, raw, journal);
      }
      throw new Error(
        `${String(error)}\n今回の書込みは保存前へ復旧しました。入力は保持しています。`,
      );
    }
    this.base = {
      ...this.base,
      saved: request.files,
      configText,
      snapshot: {
        ...this.base.snapshot,
        folders: [...desired]
          .filter((f) => f !== "sources")
          .map((f) => f.slice(8)),
        config: request.config,
      },
    };
    return { files: request.files, configText };
  }
}
