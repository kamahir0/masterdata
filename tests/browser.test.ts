import { test } from "node:test";
import assert from "node:assert/strict";
import { Session } from "../src/core/session";
import {
  BrowserProject,
  RecoveryRequired,
  load,
  pending,
  recover,
  inspect,
} from "../src/browser/project";
import { permission, type Directory } from "../src/browser/filesystem";
import { parse } from "smol-toml";

// Implements the File System Access contract (including create-before-stream,
// close visibility, nonrecursive remove, and persistent state across page loss).
class Disk {
  files = new Map<string, string | Uint8Array>();
  dirs = new Set(["", "sources", "sources/types"]);
  log: string[] = [];
  fault?: (operation: string, path: string) => void | Promise<void>;
  grant: PermissionState = "granted";
  constructor() {
    this.files.set(
      "masterdata.toml",
      '# project\r\nname = "Test"\r\n[profiles.prod]\r\ninclude = ["common"]\r\nexclude = []\r\n[publish]\r\ncsharp = "../Unity"\r\nbinary = "../data.bytes"\r\n[extension]\r\nkeep = 42\r\n',
    );
    this.files.set(
      "sources/table.yaml",
      '# comment\r\nkind: schema\r\ntable: Item\r\nfields: [{key: 0, name: id, type: ulong}, {key: 1, name: title, type: string}]\r\nrecords: [{id: 18446744073709551615, title: "first", $tags: [common]}]\r\n',
    );
    this.files.set(
      "sources/types/type.yml",
      "kind: type\nname: Stats\ncustom: {fields: []}\n",
    );
  }
  async op(operation: string, path: string) {
    this.log.push(`${operation}:${path}`);
    await this.fault?.(operation, path);
  }
  error(name: string): never {
    throw new DOMException(name, name);
  }
  dir(path = ""): Directory {
    const disk = this;
    const child = (name: string) => [path, name].filter(Boolean).join("/");
    return {
      kind: "directory",
      name: path.split("/").at(-1) || "Project",
      queryPermission: async () => disk.grant,
      requestPermission: async () => disk.grant,
      isSameEntry: async (other: any) => other === disk.root,
      async getDirectoryHandle(name: string, options?: { create?: boolean }) {
        const target = child(name);
        await disk.op("directory", target);
        if (disk.files.has(target)) disk.error("TypeMismatchError");
        if (!disk.dirs.has(target)) {
          if (!options?.create) disk.error("NotFoundError");
          await disk.op("mkdir", target);
          disk.dirs.add(target);
        }
        return disk.dir(target);
      },
      async getFileHandle(name: string, options?: { create?: boolean }) {
        const target = child(name);
        await disk.op("handle", target);
        if (disk.dirs.has(target)) disk.error("TypeMismatchError");
        if (!disk.files.has(target)) {
          if (!options?.create) disk.error("NotFoundError");
          await disk.op("create", target);
          disk.files.set(target, "");
        }
        return {
          kind: "file",
          name,
          async getFile() {
            await disk.op("read", target);
            if (!disk.files.has(target)) disk.error("NotFoundError");
            return new File([disk.files.get(target)! as BlobPart], name);
          },
          async createWritable() {
            await disk.op("stream", target);
            let draft = "";
            return {
              async write(value: string) {
                await disk.op("write", target);
                draft = value;
              },
              async close() {
                await disk.op("close", target);
                disk.files.set(target, draft);
                await disk.op("closed", target);
              },
              async abort() {
                await disk.op("abort", target);
              },
            };
          },
        };
      },
      async *values() {
        await disk.op("list", path);
        for (const name of disk.dirs)
          if (
            name &&
            name.slice(
              0,
              name.lastIndexOf("/") < 0 ? 0 : name.lastIndexOf("/"),
            ) === path
          )
            yield disk.dir(name);
        for (const name of disk.files.keys())
          if (
            name.slice(
              0,
              name.lastIndexOf("/") < 0 ? 0 : name.lastIndexOf("/"),
            ) === path
          )
            yield await disk.dir(path).getFileHandle(name.split("/").at(-1)!);
      },
      async removeEntry(name: string) {
        const target = child(name);
        await disk.op("remove", target);
        if (disk.dirs.has(target)) {
          if (
            [...disk.dirs, ...disk.files.keys()].some((p) =>
              p.startsWith(`${target}/`),
            )
          )
            disk.error("InvalidModificationError");
          disk.dirs.delete(target);
        } else if (!disk.files.delete(target)) disk.error("NotFoundError");
      },
    } as unknown as Directory;
  }
  root = this.dir();
  content() {
    return [...this.files]
      .filter(([path]) => !path.startsWith(".masterdata/"))
      .sort();
  }
}
async function fixture() {
  const disk = new Disk(),
    project = await load(disk.root),
    session = new Session();
  session.load(project);
  const adapter = new BrowserProject(disk.root, project);
  const request = (ids?: string[]) => ({
    token: project.token,
    files: session.files(ids),
    folders: session.saveFolders(session.state.snapshot, ids),
    config: session.saveConfig(session.state.snapshot, ids),
  });
  return { disk, project, session, adapter, request };
}
test("browser shares editing session without any disk IO; scoped save roundtrips comments / CRLF / ulong / TOML", async () => {
  const { disk, project, session, adapter, request } = await fixture();
  const table = project.snapshot.sources.find(
    (s) => s.data?.kind === "schema",
  )!;
  const untouched = disk.files.get("sources/types/type.yml");
  const config = disk.files.get("masterdata.toml");
  const calls = disk.log.length;
  session.update(table.id, ["records", 0, "title"], "next");
  session.addFolder();
  session.undo();
  session.redo();
  session.change("name", (s) => ({
    ...s,
    config: { ...s.config, name: "Changed" },
  }));
  assert.equal(disk.log.length, calls);
  const captured = session.state.snapshot;
  const result = await adapter.save(request([table.id]));
  session.acceptSave(result, captured, [table.id]);
  assert.equal(disk.files.get("masterdata.toml"), config);
  assert.equal(disk.files.get("sources/types/type.yml"), untouched);
  assert(!disk.dirs.has("sources/新しいフォルダ1"));
  const yaml = String(disk.files.get("sources/table.yaml"));
  assert.match(yaml, /# comment\r\n/);
  assert.match(yaml, /18446744073709551615/);
  assert.match(yaml, /title: "next"/);
  assert.match(yaml, /\$tags:/);
  assert.equal(yaml.replaceAll("\r\n", "").includes("\n"), false);
  const result2 = await adapter.save(request());
  session.acceptSave(result2, session.state.snapshot);
  assert.equal(session.dirty, false);
  const toml = parse(String(disk.files.get("masterdata.toml")));
  assert.equal((toml.extension as any).keep, 42);
  assert.deepEqual(toml.profiles, parse(String(config)).profiles);
  assert.deepEqual(toml.publish, parse(String(config)).publish);
  assert(disk.dirs.has("sources/新しいフォルダ1"));
  session.undo();
  assert(session.dirty);
  assert.equal(
    (await load(disk.root)).snapshot.sources.find(
      (s) => s.id !== table.id && s.data?.kind === "schema",
    )!.data.records[0].id,
    18446744073709551615n,
  );
});
test("browser rename / folder move / delete are virtual until Save and preserve untouched files", async () => {
  const { disk, session, adapter, request } = await fixture();
  const before = disk.content();
  session.move("types", "deep/types");
  session.move("table.yaml", "deep/items.yaml");
  session.addFolder("deep");
  assert.deepEqual(disk.content(), before);
  await adapter.save(request());
  assert(!disk.files.has("sources/table.yaml"));
  assert(!disk.dirs.has("sources/types"));
  assert(disk.files.has("sources/deep/items.yaml"));
  assert(disk.files.has("sources/deep/types/type.yml"));
  session.remove(["deep/types"]);
  await adapter.save(request());
  assert(!disk.dirs.has("sources/deep/types"));
});
test("browser all-target preflight rejects external edits, deleted files, collisions, unknown contents, unsafe paths and invalid UTF8", async () => {
  for (const conflict of [
    "external",
    "deleted",
    "collision",
    "unknown",
    "path",
    "utf8",
    "config",
  ]) {
    const { disk, project, session, adapter, request } = await fixture();
    const table = project.snapshot.sources.find(
      (s) => s.data?.kind === "schema",
    )!;
    session.update(table.id, ["records", 0, "title"], "next");
    if (conflict === "external")
      disk.files.set("sources/table.yaml", "external");
    if (conflict === "deleted") disk.files.delete("sources/table.yaml");
    if (conflict === "collision") {
      session.move("table.yaml", "occupied.yaml");
      disk.files.set("sources/occupied.yaml", "external");
    }
    if (conflict === "unknown") {
      session.remove(["types"]);
      disk.files.set("sources/types/readme.txt", "keep");
    }
    if (conflict === "path")
      session.state.snapshot.sources[0].path = "../outside.yaml";
    if (conflict === "utf8")
      disk.files.set("sources/table.yaml", new Uint8Array([0xff]));
    if (conflict === "config") {
      session.change("name", (s) => ({
        ...s,
        config: { ...s.config, name: "Next" },
      }));
      disk.files.set("masterdata.toml", 'name = "External"');
    }
    const before = disk.content();
    await assert.rejects(adapter.save(request()));
    assert.deepEqual(disk.content(), before, conflict);
    assert(!disk.log.some((op) => /^(write|remove|mkdir):/.test(op)));
  }
});
test("browser save keeps newer edits and Undo after captured snapshot acknowledgement", async () => {
  const { disk, project, session, adapter, request } = await fixture();
  const source = project.snapshot.sources[0];
  session.update(source.id, ["records", 0, "title"], "captured");
  const captured = session.state.snapshot;
  disk.fault = (op, path) => {
    if (op === "write" && path.startsWith("sources/"))
      session.update(source.id, ["records", 0, "title"], "newer");
  };
  const result = await adapter.save(request());
  session.acceptSave(result, captured);
  assert(session.dirty);
  assert.equal(
    session.state.snapshot.sources[0].data.records[0].title,
    "newer",
  );
  session.undo();
  assert.equal(session.dirty, false);
});
test("browser rolls back every failure point of multi-file replacement / deletion / folder changes", async () => {
  // Discover mutation sequence, then independently fail each operation, once.
  const first = await fixture();
  first.session.move("types", "deep/types");
  first.session.remove(["table.yaml"]);
  const start = first.disk.log.length;
  await first.adapter.save(first.request());
  const mutations = first.disk.log
    .slice(start)
    .filter((s) => /^(create|stream|write|close|closed|remove|mkdir):/.test(s));
  for (const [index, target] of mutations.entries()) {
    const { disk, session, adapter, request } = await fixture();
    const before = disk.content();
    session.move("types", "deep/types");
    session.remove(["table.yaml"]);
    let step = 0,
      hit = false;
    disk.fault = (op) => {
      if (
        /^(create|stream|write|close|closed|remove|mkdir)$/.test(op) &&
        step++ === index
      ) {
        hit = true;
        throw new Error(`fault:${target}`);
      }
    };
    await assert.rejects(adapter.save(request()), target);
    assert(hit, target);
    // New file creation may leave an empty file if a stream never closes.
    // That ambiguous state is kept behind a journal, never acknowledged.
    const same = JSON.stringify(disk.content()) === JSON.stringify(before);
    if (!same) {
      await assert.rejects(pending(disk.root), RecoveryRequired);
      await assert.rejects(adapter.save(request()), RecoveryRequired);
    }
    for (const [path, text] of before)
      assert.equal(disk.files.get(path), text, `${target}: original ${path}`);
  }
});
test("browser durable recovery survives reload, refuses third-party edits, and can restore or adopt verified complete content", async () => {
  for (const adopt of [false, true]) {
    const { disk, project, session, adapter, request } = await fixture();
    const source = project.snapshot.sources[0];
    const original = disk.files.get(`sources/${source.path}`);
    session.update(source.id, ["records", 0, "title"], "next");
    // Simulated process loss: all subsequent filesystem calls fail, including rollback.
    let crashed = false;
    disk.fault = (op, path) => {
      if (op === "closed" && path === `sources/${source.path}`) crashed = true;
      if (crashed) throw new Error("process lost");
    };
    await assert.rejects(adapter.save(request()), RecoveryRequired);
    disk.fault = undefined;
    let recovery!: RecoveryRequired;
    try {
      await load(disk.root);
    } catch (error) {
      assert(error instanceof RecoveryRequired);
      recovery = error;
    }
    assert(recovery.journal);
    assert((await inspect(recovery)).every((row) => row.state === "保存予定"));
    const intended = disk.files.get(`sources/${source.path}`);
    disk.files.set(`sources/${source.path}`, "third-party");
    await assert.rejects(recover(recovery, adopt));
    assert.equal(disk.files.get(`sources/${source.path}`), "third-party");
    assert(disk.files.has(recovery.path));
    disk.files.set(`sources/${source.path}`, intended!);
    await recover(recovery, adopt);
    assert.equal(
      disk.files.get(`sources/${source.path}`),
      adopt ? intended : original,
    );
    await pending(disk.root);
  }
});
test("browser corrupt / malicious journal and permission loss fail closed", async () => {
  for (const raw of [
    "",
    "{",
    JSON.stringify({
      version: 1,
      files: [{ path: "../outside", before: "x", after: "y" }],
      folders: [],
    }),
  ]) {
    const { disk } = await fixture();
    disk.dirs.add(".masterdata");
    disk.dirs.add(".masterdata/browser-transactions");
    disk.files.set(".masterdata/browser-transactions/pending.json", raw);
    await assert.rejects(
      load(disk.root),
      (error: any) => error instanceof RecoveryRequired && !error.journal,
    );
  }
  const disk = new Disk();
  disk.grant = "denied";
  await assert.rejects(permission(disk.root), /許可/);
  disk.grant = "granted";
  await permission(disk.root);
});
test("browser stale project tokens cannot authorize writes", async () => {
  const { disk, adapter, request } = await fixture();
  const before = disk.log.length;
  await assert.rejects(
    adapter.save({ ...request(), token: "other" }),
    /読み直/,
  );
  assert.equal(disk.log.length, before);
});

test("browser refuses hidden draft paths that native discovery would ignore after reopening", async () => {
  for (const folder of [false, true]) {
    const { disk, session, adapter, request } = await fixture();
    const before = disk.content();
    if (folder) session.move("types", ".hidden");
    else session.move("table.yaml", ".hidden.yaml");
    await assert.rejects(adapter.save(request()), /再オープンで読み込めない/);
    assert.deepEqual(disk.content(), before);
    assert(!disk.log.some((op) => /^(write|remove|mkdir):/.test(op)));
  }
});
