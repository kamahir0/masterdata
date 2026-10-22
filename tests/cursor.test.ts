import { test } from "node:test";
import assert from "node:assert/strict";
import { captureTarget, resolveTarget } from "../src/core/cursor";
import { moveItem, spliceItems } from "../src/core/identity";
import { defaultConfig, setIn, type Snapshot } from "../src/core/types";

test("nested targets follow row and duplicate array slots through insert, reorder and undo; deleted ancestors never retarget", () => {
  const original: Snapshot = { config: defaultConfig(), folders: [], sources: [{ id: "s", path: "data.yaml", data: { records: [{ items: [7, 7, 9] }, { items: [] }] } }] };
  const target = captureTarget(original, "s", ["records", 0, "items", 1]);
  const update = (snap: Snapshot, path: (string | number)[], value: any): Snapshot => ({ ...snap, sources: [{ ...snap.sources[0], data: setIn(snap.sources[0].data, path, value) }] });
  const rows = original.sources[0].data.records;
  const moved = update(original, ["records"], moveItem(rows, 0, 1));
  assert.deepEqual(resolveTarget(moved, target), ["records", 1, "items", 1]);
  const inserted = update(moved, ["records", 1, "items"], spliceItems(rows[0].items, 0, 0, 7));
  assert.deepEqual(resolveTarget(inserted, target), ["records", 1, "items", 2]);
  const movedChild = update(inserted, ["records", 1, "items"], moveItem(inserted.sources[0].data.records[1].items, 2, 0));
  assert.deepEqual(resolveTarget(movedChild, target), ["records", 1, "items", 0]);
  assert.deepEqual(resolveTarget(original, target), ["records", 0, "items", 1]);
  const deleted = update(original, ["records"], spliceItems(rows, 0, 1));
  assert.equal(resolveTarget(deleted, target), null);
  assert.equal(resolveTarget(update(inserted, ["records", 1, "items"], spliceItems(inserted.sources[0].data.records[1].items, 2, 1)), target), null);
});
