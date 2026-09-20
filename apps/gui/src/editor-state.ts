import { nullAuthoringValue, type AuthoringValue } from "./data-editor-types";

export type EditorMutationEdit = {
  recordIndex: number;
  field: string;
  value: AuthoringValue;
};

export type EditorMutationDraft = {
  draftId: string;
  values: Record<string, AuthoringValue>;
  tags: string[];
};

export type EditorMutationHistoryState = {
  edits: Record<string, EditorMutationEdit>;
  addedRecords: EditorMutationDraft[];
  pendingDeletes: number[];
  tagEdits: Record<string, string[]>;
};

export type EditorMutationState = EditorMutationHistoryState & {
  historyPast: EditorMutationHistoryState[];
  historyFuture: EditorMutationHistoryState[];
  revision: number;
  preview: { changed: boolean };
  previewState: "current" | "pending" | "unavailable";
  previewError: unknown | null;
  saveDiagnostic: unknown | null;
  queryResult: unknown | null;
  saving: boolean;
};

const HISTORY_LIMIT = 50;

function mutationHistoryState(editor: EditorMutationState): EditorMutationHistoryState {
  return {
    edits: editor.edits,
    addedRecords: editor.addedRecords,
    pendingDeletes: editor.pendingDeletes,
    tagEdits: editor.tagEdits,
  };
}

export function boundedHistoryPush(
  history: EditorMutationHistoryState[],
  state: EditorMutationHistoryState,
): EditorMutationHistoryState[] {
  if (history.length >= HISTORY_LIMIT) {
    window.alert("Undo history is full. The oldest undo entry will be discarded after this edit; the current buffer is preserved.");
  }
  return [...history, state].slice(-HISTORY_LIMIT);
}

function beginMutation<T extends EditorMutationState>(
  editor: T,
  changes: Partial<Pick<T, "edits" | "addedRecords" | "pendingDeletes" | "tagEdits">>,
): T {
  return {
    ...editor,
    ...changes,
    revision: editor.revision + 1,
    previewState: "pending",
    previewError: null,
    saveDiagnostic: null,
    queryResult: null,
    historyPast: boundedHistoryPush(editor.historyPast, mutationHistoryState(editor)),
    historyFuture: [],
  } as T;
}

export function addDraft<T extends EditorMutationState>(editor: T, draftId: string, fields: string[]): T {
  const values = Object.fromEntries(fields.map((field) => [field, nullAuthoringValue()])) as Record<string, AuthoringValue>;
  return beginMutation(editor, {
    addedRecords: [...editor.addedRecords, { draftId, values, tags: [] }],
  });
}

export function deleteDraft<T extends EditorMutationState>(editor: T, draftId: string): T {
  return beginMutation(editor, {
    addedRecords: editor.addedRecords.filter((draft) => draft.draftId !== draftId),
  });
}

export function deleteExisting<T extends EditorMutationState>(editor: T, recordIndex: number): T {
  return beginMutation(editor, {
    pendingDeletes: [...editor.pendingDeletes, recordIndex].sort((left, right) => left - right),
  });
}

export function undoExistingDelete<T extends EditorMutationState>(editor: T, recordIndex: number): T {
  return beginMutation(editor, {
    pendingDeletes: editor.pendingDeletes.filter((index) => index !== recordIndex),
  });
}

export function applyPreviewResult<T extends EditorMutationState>(editor: T, preview: T["preview"]): T {
  return {
    ...editor,
    edits: preview.changed ? editor.edits : {},
    addedRecords: preview.changed ? editor.addedRecords : [],
    pendingDeletes: preview.changed ? editor.pendingDeletes : [],
    tagEdits: preview.changed ? editor.tagEdits : {},
    preview,
    previewState: "current",
    previewError: null,
  } as T;
}
