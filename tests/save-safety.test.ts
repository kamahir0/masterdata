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
