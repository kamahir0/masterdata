import { afterEach, expect, test, vi } from "vitest";
import {
  addDraft,
  applyPreviewResult,
  boundedHistoryPush,
  deleteDraft,
  deleteExisting,
  undoExistingDelete,
  type EditorMutationState,
} from "../src/editor-state";
import type { AuthoringValue } from "../src/data-editor-types";

function editor(): EditorMutationState {
  return {
    edits: {},
    addedRecords: [],
    pendingDeletes: [],
    tagEdits: {},
    historyPast: [],
    historyFuture: [],
    revision: 0,
    preview: { changed: false },
    previewState: "current",
    previewError: null,
    saveDiagnostic: null,
    queryResult: null,
    saving: false,
  };
}

afterEach(() => vi.restoreAllMocks());

test("Add Row creates null placeholders and records one undo boundary", () => {
  const next = addDraft(editor(), "draft-1", ["id", "profile", "tags"]);

  expect(next.addedRecords).toEqual([{
    draftId: "draft-1",
    values: { id: { kind: "null" }, profile: { kind: "null" }, tags: { kind: "null" } },
    tags: [],
  }]);
  expect(next.revision).toBe(1);
  expect(next.previewState).toBe("pending");
  expect(next.historyPast).toHaveLength(1);
  expect(next.historyFuture).toEqual([]);
});

test("no-op preview clears structural draft state without changing the saved snapshot boundary", () => {
  const withDraft = addDraft(editor(), "draft-1", ["id"]);
  const removed = deleteDraft(withDraft, "draft-1");
  const clean = applyPreviewResult(removed, { changed: false });

  expect(clean.addedRecords).toEqual([]);
  expect(clean.pendingDeletes).toEqual([]);
  expect(clean.edits).toEqual({});
  expect(clean.tagEdits).toEqual({});
  expect(clean.previewState).toBe("current");
  expect(clean.preview.changed).toBe(false);
});

test("Undo Delete preserves an existing edit while removing only the pending deletion", () => {
  const value: AuthoringValue = { kind: "number", value: "11" };
  const edited = {
    ...editor(),
    edits: { "0:weight": { recordIndex: 0, field: "weight", value } },
  };
  const deleted = deleteExisting(edited, 0);
  const restored = undoExistingDelete(deleted, 0);

  expect(deleted.pendingDeletes).toEqual([0]);
  expect(restored.pendingDeletes).toEqual([]);
  expect(restored.edits["0:weight"].value).toEqual(value);
});

test("bounded history keeps the newest 50 states and warns before eviction", () => {
  const alert = vi.spyOn(window, "alert").mockImplementation(() => {});
  const state = (index: number) => ({
    edits: { [`cell-${index}`]: { recordIndex: index, field: "weight", value: { kind: "number", value: String(index) } } },
    addedRecords: [],
    pendingDeletes: [],
    tagEdits: {},
  });
  const history = Array.from({ length: 50 }, (_, index) => state(index));
  const next = boundedHistoryPush(history, state(50));

  expect(alert).toHaveBeenCalledOnce();
  expect(next).toHaveLength(50);
  expect(next[0]).toEqual(state(1));
  expect(next.at(-1)).toEqual(state(50));
});
