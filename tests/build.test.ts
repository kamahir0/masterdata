import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { loadWorkspace } from "../desktop/files";
import {
  buildWorkspace,
  publishBuild,
  dotnetExecutable,
} from "../desktop/build";
test(
  "native MasterMemory compiles and reloads nested, nullable, flags, reference and full ulong; Publish protects user files",
  { timeout: 180000, skip: !!process.env.SKIP_NATIVE_BUILD },
  async () => {
    const root = await fs.mkdtemp(path.join(os.tmpdir(), "masterdata-native-"));
    try {
      await fs.cp(path.resolve("samples/Adventure"), root, { recursive: true });
      await fs.writeFile(
        path.join(root, "sources/composite.yaml"),
        `kind: schema
table: translation
fields:
  - {key: 0, name: groupId, type: int}
  - {key: 1, name: locale, type: string}
  - {key: 2, name: parentGroup, type: int, nullable: true}
  - {key: 3, name: parentLocale, type: string, nullable: true}
primaryKey: {fields: [groupId, locale]}
references:
  - name: parent
    fields: [parentGroup, parentLocale]
    target: {table: translation, fields: [groupId, locale]}
records:
  - {groupId: 1, locale: ja, parentGroup: null, parentLocale: null}
  - {groupId: 1, locale: en, parentGroup: 1, parentLocale: ja}
`,
      );
      await fs.copyFile(
        path.resolve("tests/fixtures/display/drop.yaml"),
        path.join(root, "sources/drop.yaml"),
      );
      await fs.copyFile(
        path.resolve("tests/fixtures/display/drop.yaml.view"),
        path.join(root, "sources/drop.yaml.view"),
      );
      const { workspace, project } = await loadWorkspace(
        path.join(root, "masterdata.toml"),
      );
      const result = await buildWorkspace(workspace, project.snapshot, "", 7);
      assert(
        result.log.includes(
          "PASS MasterMemory build / reload / every selected row",
        ),
      );
      assert.equal(result.records, 14);
      const dropClass = await fs.readFile(
        path.join(result.directory, "Generated/DropPool.g.cs"), "utf8",
      );
      assert.match(dropClass, /public uint Weight/);
      assert.doesNotMatch(dropClass, /dropRate|DropRate|functionColumns|formatRules|sumBy/);
      assert(
        (await fs.stat(path.join(result.directory, "masterdata.bytes"))).size >
          0,
      );
      const generated = path.join(root, "output", "Generated");
      await fs.mkdir(generated, { recursive: true });
      await fs.writeFile(path.join(generated, "User.cs"), "// user");
      await fs.writeFile(
        path.join(generated, "Item.g.cs.meta"),
        "guid: keep-me",
      );
      const targets = project.snapshot.config.publish;
      await publishBuild(result, root, targets);
      assert.equal(
        await fs.readFile(path.join(generated, "User.cs"), "utf8"),
        "// user",
      );
      assert.equal(
        await fs.readFile(path.join(generated, "Item.g.cs.meta"), "utf8"),
        "guid: keep-me",
      );
      await fs.appendFile(
        path.join(generated, "Item.g.cs"),
        "// edited externally",
      );
      await assert.rejects(
        publishBuild(result, root, targets),
        /既存ファイルを保護/,
      );
    } finally {
      await fs.rm(root, { recursive: true });
    }
  },
);

test("packaged SDK resolution supports DOTNET_ROOT without a shell PATH", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "masterdata-sdk-"));
  try {
    const executable = path.join(
      root,
      process.platform === "win32" ? "dotnet.exe" : "dotnet",
    );
    await fs.writeFile(executable, "");
    assert.equal(
      await dotnetExecutable({ PATH: "", DOTNET_ROOT: root }),
      executable,
    );
  } finally {
    await fs.rm(root, { recursive: true });
  }
});

test("folder namespaces compile and reload references across the cutoff", { timeout: 180000, skip: !!process.env.SKIP_NATIVE_BUILD }, async t => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "masterdata-namespaces-"));
  t.after(() => fs.rm(root, { recursive: true, force: true }));
  await fs.cp(path.resolve("samples/Adventure"), root, {recursive: true});
  const file = path.join(root, "masterdata.toml");
  await fs.appendFile(file, '\n[namespaces]\nroot = "PjName"\nuseFolders = true\n[namespaces.folders]\n"Battle/Tutorial" = false\n');
  await fs.mkdir(path.join(root, "sources/Battle/Tutorial"), {recursive: true});
  await fs.rename(path.join(root, "sources/items"), path.join(root, "sources/Battle/Tutorial/Enemy"));
  const {workspace, project} = await loadWorkspace(file);
  const result = await buildWorkspace(workspace, project.snapshot, "", 1);
  await publishBuild(result, root, project.snapshot.config.publish);
  const read = (name: string) => fs.readFile(path.join(root, "output/Generated", name), "utf8");
  assert.match(await read("Item.g.cs"), /namespace PjName\.Battle;/);
  assert.match(await read("Item.g.cs"), /global::PjName\.types\.ItemCode/);
  assert.match(await read("Quest.g.cs"), /global::PjName\.Battle\.Item/);
  assert.match(result.log, /PASS MasterMemory build \/ reload/);
});
