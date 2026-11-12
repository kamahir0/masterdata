import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { loadWorkspace, saveWorkspace, safePath } from "../desktop/files";

test("native host save preflights unknown folder contents and rolls back file/deleted-folder changes after removal fails", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "masterdata-save-safety-"));
  const originalRemove = fs.rmdir;
  try {
    await fs.mkdir(path.join(root, "sources/types/child"), { recursive: true });
    await fs.writeFile(path.join(root, "masterdata.toml"), 'name = "Safety"\n');
    const file = path.join(root, "sources/types/child/type.yaml");
    await fs.writeFile(file, "kind: type\nname: Stats\ncustom: {fields: []}\n");
    const { workspace, project } = await loadWorkspace(path.join(root, "masterdata.toml"));
    const request = { token: project.token, files: [], folders: [], config: project.snapshot.config };
    const unknown = path.join(root, "sources/types/notes.txt");
    await fs.writeFile(unknown, "keep me");
    await assert.rejects(saveWorkspace(workspace, request), /保存対象外/);
    assert.equal(await fs.readFile(file, "utf8"), project.saved[0].text);
    assert.equal(await fs.readFile(unknown, "utf8"), "keep me");
    await fs.unlink(unknown);
    fs.rmdir = (async (dir: any, ...args: any[]) => {
      if (dir === path.join(workspace.root, "sources/types")) throw new Error("injected directory removal failure");
      return (originalRemove as any)(dir, ...args);
    }) as typeof fs.rmdir;
    await assert.rejects(saveWorkspace(workspace, request), /元に戻しました/);
    assert.equal(await fs.readFile(file, "utf8"), project.saved[0].text);
    assert.deepEqual(workspace.baseline, project.saved);
    assert.deepEqual(workspace.folders, project.snapshot.folders);
    fs.rmdir = originalRemove;
    await saveWorkspace(workspace, request);
    await assert.rejects(fs.stat(path.join(root, "sources/types")), { code: "ENOENT" });
    for (const name of ["sources/C:/escape", "sources/C:escape", "sources/a\\..\\escape"])
      await assert.rejects(safePath(root, name), /Project外/);
  } finally {
    fs.rmdir = originalRemove;
    await fs.rm(root, { recursive: true, force: true });
  }
});

import { Session } from "../src/core/session";
import { displayDefinitions, updateDisplayDefinitions } from "../src/core/display-definitions";

async function viewWorkspace() {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "masterdata-sidecar-"));
  await fs.mkdir(path.join(root, "sources/arbitrary"), { recursive: true });
  await fs.writeFile(path.join(root, "masterdata.toml"), 'name = "任意の名前"\n');
  const schema = '# exact\r\nkind: schema\r\ntable: Item\r\nfields: [{key: 0, name: id, type: ulong}]\r\n';
  const view = '# view\r\nfunctionColumns: [{name: Rate, script: "return {"}]\r\nunknown: [1, 2]\r\n';
  await fs.writeFile(path.join(root, "sources/arbitrary/schema.yaml"), schema);
  await fs.writeFile(path.join(root, "sources/arbitrary/schema.yaml.view"), view);
  const { workspace, project } = await loadWorkspace(path.join(root, "masterdata.toml"), true);
  const session = new Session(); session.load(project);
  const current = () => session.state.snapshot.sources[0];
  const edit = () => updateDisplayDefinitions(session, current(), "functionColumns", [{ name: "Next", script: "return { text: 'x' };\n" }], "表示編集");
  const request = () => ({ token: project.token, files: session.files(), folders: session.state.snapshot.folders, config: session.state.snapshot.config });
  return { root, schema, view, workspace, project, session, current, edit, request };
}

test("native GUI view save is byte isolated, CLI loader ignores sidecars and GUI move/delete is paired", async () => {
  const { root, workspace, project, session, current, edit, request, schema } = await viewWorkspace();
  try {
    const canonical = path.join(root, "sources/arbitrary/schema.yaml");
    const attachment = `${canonical}.view`;
    const cli = await loadWorkspace(path.join(root, "masterdata.toml"));
    assert.equal(cli.project.saved.length, 1);
    assert.equal(cli.project.snapshot.sources[0].display, undefined);
    assert.equal(project.snapshot.sources.length, 1);
    assert.equal(project.saved.length, 2);
    edit();
    const captured = session.state.snapshot;
    session.acceptSave(await saveWorkspace(workspace, request()), captured);
    assert.equal(await fs.readFile(canonical, "utf8"), schema);
    const reloaded = await loadWorkspace(path.join(root, "masterdata.toml"), true);
    assert.equal((displayDefinitions(reloaded.project.snapshot.sources[0], "functionColumns").items[0] as any).name, "Next");
    const view = await fs.readFile(attachment, "utf8");
    session.move(current().path, "moved/schema.yml");
    session.acceptSave(await saveWorkspace(workspace, request()), session.state.snapshot);
    assert.equal(await fs.readFile(path.join(root, "sources/moved/schema.yml"), "utf8"), schema);
    assert.equal(await fs.readFile(path.join(root, "sources/moved/schema.yml.view"), "utf8"), view);
    await assert.rejects(fs.stat(canonical), { code: "ENOENT" });
    session.remove(["moved/schema.yml"]);
    await saveWorkspace(workspace, request());
    await assert.rejects(fs.stat(path.join(root, "sources/moved/schema.yml.view")), { code: "ENOENT" });
  } finally { await fs.rm(root, { recursive: true, force: true }); }
});

test("native GUI preflight protects changed companions, orphan collisions and unreadable view files", async () => {
  for (const conflict of ["view", "schema", "deleted", "orphan"]) {
    const { root, workspace, session, current, edit, request } = await viewWorkspace();
    try {
      const original = path.join(root, "sources/arbitrary/schema.yaml");
      if (conflict === "orphan") {
        session.move(current().path, "new.yaml");
        await fs.writeFile(path.join(root, "sources/new.yaml.view"), "orphan");
      } else {
        edit();
        if (conflict === "deleted") await fs.unlink(`${original}.view`);
        else await fs.writeFile(`${original}${conflict === "view" ? ".view" : ""}`, "external");
      }
      await assert.rejects(saveWorkspace(workspace, request()), /外部/);
      assert.equal((await fs.readdir(root)).includes(".masterdata"), false);
    } finally { await fs.rm(root, { recursive: true, force: true }); }
  }
  const { root } = await viewWorkspace();
  try {
    await fs.writeFile(path.join(root, "sources/arbitrary/schema.yaml.view"), new Uint8Array([0xff]));
    await assert.rejects(loadWorkspace(path.join(root, "masterdata.toml"), true));
    assert.equal((await loadWorkspace(path.join(root, "masterdata.toml"))).project.saved.length, 1);
  } finally { await fs.rm(root, { recursive: true, force: true }); }
});

test("native GUI sidecar failure rolls canonical files back and leaves memory edits pending", async () => {
  const { root, workspace, session, current, edit, request, schema, view } = await viewWorkspace();
  const rename = fs.rename;
  try {
    edit(); session.update(current().id, ["table"], "Changed");
    fs.rename = (async (from: any, to: any) => {
      if (String(to).endsWith(".view")) throw new Error("injected sidecar failure");
      return rename(from, to);
    }) as typeof fs.rename;
    await assert.rejects(saveWorkspace(workspace, request()), /元に戻しました/);
    assert.equal(await fs.readFile(path.join(root, "sources/arbitrary/schema.yaml"), "utf8"), schema);
    assert.equal(await fs.readFile(path.join(root, "sources/arbitrary/schema.yaml.view"), "utf8"), view);
    assert(session.dirty);
    assert.equal(workspace.baseline.length, 2);
  } finally {
    fs.rename = rename;
    await fs.rm(root, { recursive: true, force: true });
  }
});
