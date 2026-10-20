import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { parse } from "smol-toml";
import { readConfig, writeConfig, projectFileName } from "../desktop/config";
import { loadWorkspace, saveWorkspace } from "../desktop/files";
import { ProjectService } from "../desktop/service";
import { publishBuild } from "../desktop/build";
import type { LoadedProject, BuildResult } from "../src/core/types";

async function fixture(t: { after(fn: () => Promise<void>): void }) {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "masterdata-project-"));
  t.after(() => fs.rm(root, { recursive: true, force: true }));
  await fs.cp(path.resolve("samples/Adventure"), root, { recursive: true });
  return { root, file: path.join(root, projectFileName) };
}

test("project identity requires the fixed project file and validates known settings", async (t) => {
  const { root, file } = await fixture(t);
  const original = await fs.readFile(file, "utf8");
  const other = path.join(root, "other.toml");
  await fs.writeFile(other, original);
  for (const input of [root, other, path.join(root, "masterdata.yaml")])
    await assert.rejects(loadWorkspace(input), /masterdata.toml/);
  for (const text of [
    "name = [",
    'name = ""',
    '[project]\nname = "Old"',
    'name = "Test"\nprofiles = []',
    'name = "Test"\n[profiles.bad]\ninclude = [1]',
    'name = "Test"\n[publish]\nbinary = 3',
  ]) {
    await fs.writeFile(file, text);
    await assert.rejects(loadWorkspace(file), (error: Error) =>
      error.message.includes(file),
    );
    assert.equal(await fs.readFile(file, "utf8"), text);
  }
});

test("create, settings save and reopen use TOML with quoted profile names and retain unknown values", async (t) => {
  const { root } = await fixture(t);
  const service = new ProjectService();
  const project = (await service.request("create", [
    "新規作業",
    root,
  ])) as LoadedProject;
  const file = path.join(project.root, projectFileName);
  assert.equal(project.snapshot.config.name, "新規作業");
  await assert.rejects(fs.stat(path.join(project.root, "masterdata.yaml")), {
    code: "ENOENT",
  });
  await fs.appendFile(file, '\n[extension]\nkeep = "untouched"\n');
  const loaded = await loadWorkspace(file);
  const config = structuredClone(loaded.project.snapshot.config);
  config.name = "表示名だけ変更";
  config.profiles = {
    '本番.v2 "設定"': { include: ["common"], exclude: ["debug", "event"] },
  };
  config.publish = {
    csharp: "../Unity/Generated",
    binary: "../Unity/data.bytes",
  };
  await saveWorkspace(loaded.workspace, {
    token: loaded.project.token,
    files: [],
    folders: [],
    config,
  });
  assert.deepEqual((await loadWorkspace(file)).project.snapshot.config, config);
  assert.equal(
    (parse(await fs.readFile(file, "utf8")).extension as any).keep,
    "untouched",
  );
  assert.equal(path.basename(project.root), "新規作業");
});

test("source-only save leaves project comments, CRLF and omitted defaults byte-for-byte intact", async (t) => {
  const { file } = await fixture(t);
  const text = '# 手編集した設定\r\nname = "冒険の道具箱" # 名前\r\n';
  await fs.writeFile(file, text);
  const { workspace, project } = await loadWorkspace(file);
  const files = project.saved.map((f, i) =>
    i ? f : { ...f, text: `${f.text}\n# source edit\n` },
  );
  await saveWorkspace(workspace, {
    token: project.token,
    files,
    folders: project.snapshot.folders,
    config: project.snapshot.config,
  });
  assert.equal(await fs.readFile(file, "utf8"), text);
  assert.equal(workspace.configText, text);
  assert.throws(
    () => writeConfig({ ...project.snapshot.config, name: "" }, text),
    /name/,
  );
});

test("external project conflict prevents every source write and failed open retains the prior service workspace", async (t) => {
  const { file, root } = await fixture(t);
  const service = new ProjectService();
  const project = (await service.request("open", [file])) as LoadedProject;
  await assert.rejects(
    service.request("open", [path.join(root, "other.toml")]),
    /masterdata.toml/,
  );
  const before = await fs.readFile(file, "utf8");
  await fs.appendFile(file, "\n# external\n");
  const first = project.saved[0];
  const request = {
    token: project.token,
    files: project.saved.map((f) =>
      f.id === first.id ? { ...f, text: f.text + "\n# edited\n" } : f,
    ),
    folders: project.snapshot.folders,
    config: { ...project.snapshot.config, name: "変更" },
  };
  await assert.rejects(service.request("save", [request]), /設定が外部で変更/);
  assert.equal(
    await fs.readFile(path.join(root, "sources", first.path), "utf8"),
    first.text,
  );
  assert.equal(await fs.readFile(file, "utf8"), before + "\n# external\n");
  await fs.writeFile(file, before);
  await service.request("save", [request]);
  assert.equal(readConfig(await fs.readFile(file, "utf8")).name, "変更");
});

test("Publish cannot overwrite the TOML project file", async (t) => {
  const { file, root } = await fixture(t);
  const before = await fs.readFile(file, "utf8");
  await assert.rejects(
    publishBuild({} as BuildResult, root, {
      csharp: "",
      binary: projectFileName,
    }),
    /編集元/,
  );
  assert.equal(await fs.readFile(file, "utf8"), before);
});
