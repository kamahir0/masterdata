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
import {
  browser,
  browserStatus,
  openRecent,
  projectNameError,
} from "../src/browser/runtime";
import {
  saveRecentDirectory,
  getRecentDirectory,
  clearRecentDirectory,
} from "../src/browser/storage";
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
    assert.throws(
      () =>
        session.move(
          folder ? "types" : "table.yaml",
          folder ? ".hidden" : ".hidden.yaml",
        ),
      /読込対象外/,
    );
    const malicious = request();
    if (folder) malicious.folders = [".hidden"];
    else malicious.files[0] = { ...malicious.files[0], path: ".hidden.yaml" };
    await assert.rejects(adapter.save(malicious), /再オープンで読み込めない/);
    assert.deepEqual(disk.content(), before);
    assert(!disk.log.some((op) => /^(write|remove|mkdir):/.test(op)));
  }
});

function setupMockBrowser() {
  const stores = new Map<string, Map<string, any>>();
  const origIndexedDB = (globalThis as any).indexedDB;
  const origWindow = (globalThis as any).window;
  const origFSFile = (globalThis as any).FileSystemFileHandle;
  const origFSDir = (globalThis as any).FileSystemDirectoryHandle;

  (globalThis as any).indexedDB = {
    open: () => {
      const req: any = {
        result: {
          close: () => {},
          objectStoreNames: { contains: (s: string) => stores.has(s) },
          createObjectStore: (s: string) => stores.set(s, new Map()),
        },
      };
      req.result.transaction = (s: string) => {
        let store = stores.get(s);
        if (!store) {
          store = new Map();
          stores.set(s, store);
        }
        const result = (value: any) => {
          const req: any = { result: value };
          queueMicrotask(() => {
            req.onsuccess?.();
            tx.oncomplete?.();
          });
          return req;
        };
        const tx: any = {
          objectStore: () => ({
            put: (v: any, k: string) => {
              store!.set(k, v);
              return result(k);
            },
            get: (k: string) => {
              return result(store!.get(k));
            },
            delete: (k: string) => {
              store!.delete(k);
              return result(undefined);
            },
          }),
        };
        return tx;
      };
      queueMicrotask(() => {
        req.onupgradeneeded?.();
        req.onsuccess?.({ target: req });
      });
      return req;
    },
  };

  (globalThis as any).FileSystemFileHandle = class {};
  (globalThis as any).FileSystemFileHandle.prototype.createWritable = () => {};
  (globalThis as any).FileSystemDirectoryHandle = class {};
  (globalThis as any).FileSystemDirectoryHandle.prototype.queryPermission =
    () => {};
  (globalThis as any).FileSystemDirectoryHandle.prototype.requestPermission =
    () => {};
  (globalThis as any).FileSystemDirectoryHandle.prototype.values = () => {};

  (globalThis as any).window = {
    isSecureContext: true,
    showDirectoryPicker: async () => {},
  };
  const origLocks = (navigator as any).locks;
  Object.defineProperty(navigator, "locks", {
    value: {
      request: (_name: string, fn: () => any) => fn(),
    },
    configurable: true,
    writable: true,
  });

  return () => {
    (globalThis as any).indexedDB = origIndexedDB;
    (globalThis as any).window = origWindow;
    if (origLocks !== undefined) {
      Object.defineProperty(navigator, "locks", {
        value: origLocks,
        configurable: true,
        writable: true,
      });
    } else {
      delete (navigator as any).locks;
    }
    (globalThis as any).FileSystemFileHandle = origFSFile;
    (globalThis as any).FileSystemDirectoryHandle = origFSDir;
  };
}

test("browser storage persists and retrieves recent directory handle via IndexedDB", async () => {
  const restore = setupMockBrowser();
  try {
    const disk = new Disk();
    const dir = disk.dir();
    await saveRecentDirectory(dir);
    const retrieved = await getRecentDirectory();
    assert.equal(retrieved?.name, dir.name);

    await clearRecentDirectory();
    assert.equal(await getRecentDirectory(), null);
  } finally {
    restore();
  }
});

test("browser initial automatically restores project when permission is granted", async () => {
  const restore = setupMockBrowser();
  try {
    const disk = new Disk();
    disk.grant = "granted";
    await saveRecentDirectory(disk.root);

    const project = await browser.initial();
    assert.ok(project);
    assert.equal(project.root, "Project");
    assert.equal(browserStatus.get().recent, undefined);
  } finally {
    restore();
  }
});

test("browser initial prompts via status when permission is not granted, and openRecent requests it", async () => {
  const restore = setupMockBrowser();
  try {
    const disk = new Disk();
    disk.grant = "prompt";
    await saveRecentDirectory(disk.root);

    // Initial returns null but registers recent in browserStatus
    const initialResult = await browser.initial();
    assert.equal(initialResult, null);
    const recent = browserStatus.get().recent;
    assert.ok(recent);
    assert.equal(recent.name, "Project");

    // Calling openRecent with denied permission returns null
    disk.grant = "denied";
    const deniedResult = await openRecent();
    assert.equal(deniedResult, null);

    // Calling openRecent with granted permission loads the project
    disk.grant = "granted";
    const loadedResult = await openRecent();
    assert.ok(loadedResult);
    assert.equal(loadedResult.root, "Project");
    assert.equal(browserStatus.get().recent, undefined);
  } finally {
    restore();
  }
});

test("browser initial clears stored handle if folder is missing masterdata.toml", async () => {
  const restore = setupMockBrowser();
  try {
    const disk = new Disk();
    disk.files.delete("masterdata.toml");
    disk.grant = "granted";
    await saveRecentDirectory(disk.root);

    const result = await browser.initial();
    assert.equal(result, null);
    assert.equal(await getRecentDirectory(), null);
  } finally {
    restore();
  }
});

test("IndexedDB error, abort and synchronous failures never block project open or save", async () => {
  for (const fault of ["error", "abort", "throw", "open"]) {
    const restore = setupMockBrowser();
    let closed = 0;
    try {
      (globalThis as any).indexedDB = {
        open() {
          const req: any = {
            result: {
              close: () => closed++,
              transaction() {
                if (fault === "throw") throw new Error("store unavailable");
                const tx: any = {
                  error: new DOMException("quota", "QuotaExceededError"),
                  objectStore: () => ({
                    put: () => ({}),
                    get: () => ({}),
                    delete: () => ({}),
                  }),
                };
                queueMicrotask(() =>
                  fault === "abort" ? tx.onabort?.() : tx.onerror?.(),
                );
                return tx;
              },
            },
          };
          queueMicrotask(() =>
            fault === "open" ? req.onerror?.() : req.onsuccess?.(),
          );
          return req;
        },
      };
      const disk = new Disk();
      await saveRecentDirectory(disk.root);
      assert.equal(await getRecentDirectory(), null);
      await clearRecentDirectory();
      (window as any).showDirectoryPicker = async () => disk.root;
      const project = (await browser.open())!;
      assert(project.snapshot.sources.length);
      const session = new Session();
      session.load(project);
      session.update(
        project.snapshot.sources[0].id,
        ["records", 0, "title"],
        "after quota failure",
      );
      await browser.save({
        token: project.token,
        files: session.files(),
        folders: project.snapshot.folders,
        config: project.snapshot.config,
      });
      assert.match(
        String(disk.files.get("sources/table.yaml")),
        /after quota failure/,
      );
      assert.equal(closed, fault === "open" ? 0 : 4);
    } finally {
      restore();
    }
  }
});

function emptyDisk() {
  const disk = new Disk();
  disk.files.clear();
  disk.dirs = new Set([""]);
  (globalThis as any).window.showDirectoryPicker = async () => disk.root;
  return disk;
}
const writes = (disk: Disk) =>
  disk.log.filter((op) => /^(create|mkdir|remove|write):/.test(op));

test("browser create initializes the selected folder itself, starts a session and persists the handle", async () => {
  const restore = setupMockBrowser();
  try {
    const disk = emptyDisk();
    const created = await browser.create("  新しい  Project ");
    assert.ok(created);
    assert.deepEqual([...disk.files.keys()].sort(), ["masterdata.toml"]);
    assert.deepEqual([...disk.dirs].sort(), ["", "sources"]);
    assert.equal(
      parse(disk.files.get("masterdata.toml") as string).name,
      "新しい  Project",
    );
    assert.deepEqual(created.snapshot.sources, []);
    assert.equal(created.snapshot.config.name, "新しい  Project");
    // The same bytes as Desktop / CLI init, and reopenable without changes.
    const reopened = await load(disk.root);
    assert.equal(reopened.configText, disk.files.get("masterdata.toml"));
    assert.equal((await getRecentDirectory())?.name, disk.root.name);
    // The new session saves through the same adapter as an opened project.
    const session = new Session();
    session.load(created);
    const saved = await browser.save({
      token: created.token,
      files: session.files(),
      folders: session.saveFolders(session.state.snapshot),
      config: session.saveConfig(session.state.snapshot),
    });
    assert.ok(saved);
  } finally {
    restore();
  }
});

test("browser create never writes for invalid names, cancellation, existing projects or blocking entries", async () => {
  const restore = setupMockBrowser();
  try {
    for (const bad of ["", "   ", ".", "..", "a/b", "a\\b", 'a"b', "a\u0001b"])
      assert.notEqual(projectNameError(bad), "", JSON.stringify(bad));
    assert.equal(projectNameError(" 日本語 name-1 "), "");

    let disk = emptyDisk();
    let picks = 0;
    (globalThis as any).window.showDirectoryPicker = async () => {
      picks++;
      return disk.root;
    };
    await assert.rejects(browser.create("a/b"), /フォルダ名に使える文字/);
    assert.equal(picks, 0);
    assert.deepEqual(disk.log, []);

    (globalThis as any).window.showDirectoryPicker = async () => {
      throw new DOMException("cancel", "AbortError");
    };
    assert.equal(await browser.create("Name"), null);
    (globalThis as any).window.showDirectoryPicker = async () => {
      throw new DOMException("denied", "NotAllowedError");
    };
    await assert.rejects(browser.create("Name"), /denied/);
    assert.deepEqual(writes(disk), []);
    assert.equal(browserStatus.get().busy, false);

    // Existing project: nothing is written and the user is pointed at Open.
    disk = new Disk();
    (globalThis as any).window.showDirectoryPicker = async () => disk.root;
    const before = disk.content();
    await assert.rejects(browser.create("Other"), /プロジェクトを開く/);
    assert.deepEqual(disk.content(), before);
    assert.deepEqual(writes(disk), []);

    // Permission denied for the selected folder.
    disk = emptyDisk();
    disk.grant = "denied";
    await assert.rejects(browser.create("Name"), /許可されていません/);
    assert.deepEqual(writes(disk), []);

    // A file named sources blocks creation before masterdata.toml is written.
    disk = emptyDisk();
    disk.files.set("sources", "not a folder");
    await assert.rejects(browser.create("Name"), /sources という名前のfile/);
    assert.equal(disk.files.has("masterdata.toml"), false);

    // A folder named masterdata.toml is not touched either.
    disk = emptyDisk();
    disk.dirs.add("masterdata.toml");
    await assert.rejects(
      browser.create("Name"),
      /masterdata\.toml という名前のフォルダ/,
    );
    assert.deepEqual(writes(disk), []);

    // Interrupted-save journals use the normal recovery path, not creation.
    disk = emptyDisk();
    disk.dirs.add(".masterdata");
    disk.dirs.add(".masterdata/browser-transactions");
    disk.files.set(".masterdata/browser-transactions/x.json", "{}");
    await assert.rejects(browser.create("Name"), /中断した保存/);
    assert.equal(disk.files.has("masterdata.toml"), false);
  } finally {
    restore();
  }
});

test("browser create keeps existing files and removes only its own config when verification fails", async () => {
  const restore = setupMockBrowser();
  try {
    let disk = emptyDisk();
    disk.files.set("notes.txt", "keep");
    disk.dirs.add("sources");
    disk.files.set(
      "sources/a.yaml",
      "kind: type\nname: A\ncustom: {fields: []}\n",
    );
    assert.ok(await browser.create("Existing"));
    assert.equal(disk.files.get("notes.txt"), "keep");
    assert.ok(disk.files.has("sources/a.yaml"));
    assert.ok(disk.files.has("masterdata.toml"));

    disk = emptyDisk();
    disk.dirs.add("sources");
    disk.files.set("sources/bad.yaml", new Uint8Array([0xff, 0xfe, 0xfd]));
    await assert.rejects(browser.create("Broken"));
    assert.equal(disk.files.has("masterdata.toml"), false);
    assert.ok(disk.files.has("sources/bad.yaml"));
    assert.equal(browserStatus.get().busy, false);
  } finally {
    restore();
  }
});
