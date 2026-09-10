import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";

const source = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");

test("GUI keeps filesystem and YAML semantics behind Tauri commands", () => {
  assert.match(source, /invoke<AuthoringWorkspace>\("authoring_workspace"/);
  assert.match(source, /invoke<DataFileSnapshot>\("open_data_file"/);
  assert.match(source, /invoke<SourceEditPreview>\("preview_data_file"/);
  assert.match(source, /invoke<SourceSaveReport>\("save_data_file"/);
  assert.match(source, /invoke<SourceContentState>\("source_content"/);
  assert.doesNotMatch(source, /child_process|readFileSync|readdirSync|js-yaml|yaml\.parse|YAML\.parse/);
});

test("GUI exposes file-scoped dirty, save, validation, and diff interactions", () => {
  assert.match(source, /function editorIsDirty/);
  assert.match(source, /Object\.values\(editor\.edits\)/);
  assert.match(source, /event\.metaKey \|\| event\.ctrlKey/);
  assert.match(source, /activePath\) void saveFile\(activePath\)/);
  assert.match(source, /Buffer validation pending/);
  assert.match(source, /Unsaved source diff/);
  assert.match(source, /Build uses saved source only/);
});

test("GUI protects dirty buffers across navigation and external changes", () => {
  assert.match(source, /Save All/);
  assert.match(source, /Don't Save/);
  assert.match(source, /Cancel/);
  assert.match(source, /onCloseRequested/);
  assert.match(source, /File changed outside masterdata/);
  assert.match(source, /Compare/);
  assert.match(source, /Reload/);
  assert.match(source, /Overwrite/);
  assert.match(source, /overwriteExpectedIdentity/);
});

test("GUI uses shared manual validation and full canonical build", () => {
  assert.match(source, /invoke<ValidationReport>\("validate"/);
  assert.match(source, /invoke<BuildResponse>\("build"/);
  assert.match(source, /dryRun: false/);
  assert.match(source, /Disk \$\{manualValidation\.value\.valid/);
});

test("GUI keeps 64-bit primitive edits as text at the frontend boundary", () => {
  assert.match(source, /value: string/);
  assert.match(source, /<input[\s\S]*value=\{value\}/);
  assert.doesNotMatch(source, /parseInt\(|parseFloat\(|Number\(value\)|valueAsNumber/);
});


test("Explorer is a hierarchical keyboard tree with observable file states", () => {
  assert.match(source, /role="tree"/);
  assert.match(source, /role="treeitem"/);
  assert.match(source, /aria-expanded=\{expanded\}/);
  assert.match(source, /ArrowRight/);
  assert.match(source, /ArrowLeft/);
  assert.match(source, /save outcome unknown/);
  assert.match(source, /loadingPaths/);
});

test("background clean-file reload does not steal Explorer selection", () => {
  const start = source.indexOf("const openDataFile = useCallback");
  const end = source.indexOf("const loadWorkspace = useCallback", start);
  const openDataFile = source.slice(start, end);
  assert.doesNotMatch(openDataFile, /setActivePath\(/);
  assert.match(source, /void openDataFile\(root, path, true\)/);
});

test("shared preview collapses semantic no-op edits back to clean", () => {
  assert.match(source, /edits: preview\.changed \? latest\.edits : \{\}/);
});

test("Problems navigation survives asynchronous file opening", () => {
  assert.match(source, /pendingCellFocus/);
  assert.match(source, /\[activePath, editors, loadingPaths\]/);
});

test("save failure and unknown outcome expose explicit recovery", () => {
  assert.match(source, /Recheck Source/);
  assert.match(source, /Previous save outcome is unknown/);
  assert.match(source, /editor\.saveStatus === "outcome_unknown"/);
});

test("Problems identifies buffer, saved-source, and operation snapshots", () => {
  assert.match(source, /origin: "Buffer"/);
  assert.match(source, /origin: "Saved source"/);
  assert.match(source, /origin: "Operation"/);
});


test("clean-file reload failures stay file-scoped and keep project polling alive", () => {
  const start = source.indexOf("const openDataFile = useCallback");
  const end = source.indexOf("const loadWorkspace = useCallback", start);
  const openDataFile = source.slice(start, end);
  assert.match(openDataFile, /setFileOpenErrors/);
  assert.doesNotMatch(openDataFile, /setWorkspaceState/);
  assert.match(source, /fileOpenErrors=\{fileOpenErrors\}/);
});
