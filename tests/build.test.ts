import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { loadWorkspace } from "../desktop/files";
import { buildWorkspace, publishBuild } from "../desktop/build";
test(
  "native MasterMemory compiles and reloads nested, nullable, flags, reference and full ulong; Publish protects user files",
  { timeout: 180000, skip: !!process.env.SKIP_NATIVE_BUILD },
  async () => {
    const root = await fs.mkdtemp(path.join(os.tmpdir(), "masterdata-native-"));
    try {
      await fs.cp(path.resolve("samples/Adventure"), root, { recursive: true });
      const { workspace, project } = await loadWorkspace(root);
      const result = await buildWorkspace(workspace, project.snapshot, "", 7);
      assert(
        result.log.includes(
          "PASS MasterMemory build / reload / every selected row",
        ),
      );
      assert.equal(result.records, 7);
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
