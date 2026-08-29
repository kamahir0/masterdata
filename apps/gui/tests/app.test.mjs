import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";

test("GUI delegates project loading to the Tauri command", async () => {
  const source = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
  assert.match(source, /invoke<ProjectInfo>\("project_info"/);
  assert.doesNotMatch(source, /child_process|readFileSync|readdir/);
});

