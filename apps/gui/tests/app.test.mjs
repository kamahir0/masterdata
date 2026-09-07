import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";

test("GUI delegates project loading to the Tauri command", async () => {
  const source = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
  assert.match(source, /invoke<ProjectInfo>\("project_info"/);
  assert.match(source, /relatedRequirements/);
  assert.match(source, /type ApiError/);
  assert.doesNotMatch(source, /child_process|readFileSync|readdir/);
});

test("GUI exposes validation through the shared Tauri command", async () => {
  const source = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
  assert.match(source, /invoke<ValidationReport>\("validate"/);
  assert.match(source, /type ValidationReport/);
  assert.match(source, /ValidationPanel/);
  assert.match(source, /diagnostic\.related_requirements/);
  assert.match(source, /disabled=\{state\.kind !== "loaded"/);
});
