import TypeEditor from "./TypeEditor";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Alert, Button, Empty, Input, Modal, Select, Tabs, Tag } from "antd";
import { Database, FolderOpen, Save, RotateCw, ShieldCheck, Play } from "lucide-react";
import TableEditor, { type MigrationResult } from "./TableEditor";
import SourceCreation, { type CreationReport } from "./SourceCreation";
import {
  DeliveryPanel,
  ProjectCreatePanel,
  ProjectOverviewPanel,
  ProjectSettingsPanel,
  type BuildProfileInfo,
  type PublishTargetInfo,
  type SurfaceWorkspace,
} from "./ProjectSurfaces";
import ValueEditor from "./ValueEditor";
import {
  authoringValueSummary,
  authoringValuesEqual,
  nullAuthoringValue,
  type AuthoringValue,
  type ResolvedAuthoringField,
} from "./data-editor-types";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

type ProjectInfo = {
  project_root: string;
  config_path: string;
  project_id: string;
  name: string;
  version: string;
  source_roots: string[];
  artifact_root: string;
  csharp_output: string;
  binary_output: string;
  cache: string;
  profiles?: BuildProfileInfo[];
  publish_targets?: PublishTargetInfo[];
};

type Diagnostic = {
  code: string;
  kind: string;
  message: string;
  source?: string | null;
  line?: number | null;
  column?: number | null;
  schema_path?: string | null;
  value_path?: string | null;
  record_identity?: string | null;
  suggestion?: string | null;
  related_requirements?: string[];
};

type ApiDiagnostic = {
  code: string;
  kind: string;
  message: string;
  source: string | null;
  line: number | null;
  column: number | null;
  schemaPath: string | null;
  valuePath: string | null;
  recordIdentity: string | null;
  suggestion: string | null;
  relatedRequirements: string[];
};

type ApiError = { diagnostic: ApiDiagnostic };

type AuthoringCapabilities = {
  workspaceRead: boolean;
  workspaceWrite: boolean;
  validate: boolean;
  build: boolean;
};

type WorkspaceSourceFile = {
  path: string;
  sourceRoot: string;
  kind: string;
  table: string | null;
  typeName: string | null;
  diagnostic: Diagnostic | null;
};

type AuthoringWorkspace = {
  project: ProjectInfo;
  sourceRoots: string[];
  files: WorkspaceSourceFile[];
  folders?: { path: string; sourceRoot: string }[];
  capabilities: AuthoringCapabilities;
};

type DataEditorColumn = {
  name: string;
  typeName: string;
  editable: boolean;
  keyField: boolean;
  shape: ResolvedAuthoringField | null;
  readOnlyReason: string | null;
};

type DataEditorCell = {
  field: string;
  text: string;
  value: AuthoringValue;
  editable: boolean;
  readOnlyReason: string | null;
};

type DataEditorRow = {
  recordIndex: number;
  cells: DataEditorCell[];
  tags?: string[];
  tagsEditable?: boolean;
  tagsReadOnlyReason?: string | null;
};

type DataEditorAddCapability = {
  supported: boolean;
  reason: string | null;
};

type ValidationReport = {
  valid: boolean;
  files_scanned: number;
  schema_documents: number;
  data_documents: number;
  type_documents: number;
  tables: string[];
  types: string[];
  diagnostics: Diagnostic[];
};

type DataFileSnapshot = {
  path: string;
  table: string;
  baseSource: string;
  baseContentIdentity: string;
  columns: DataEditorColumn[];
  rows: DataEditorRow[];
  tagCandidates?: string[];
  tagCandidatesComplete?: boolean;
  addRow?: DataEditorAddCapability;
  validation: ValidationReport;
};

type AuthoringEdit = {
  recordIndex: number;
  field: string;
  value: AuthoringValue;
};

type AuthoringRecordField = {
  field: string;
  value: AuthoringValue;
};

type AuthoringRecordDraft = {
  fields: AuthoringRecordField[];
  tags: string[];
};

type AuthoringRecordMutation = {
  edits: AuthoringEdit[];
  addedRecords: AuthoringRecordDraft[];
  deletedRecordIndices: number[];
  tagEdits: RecordTagEdit[];
};

type RecordTagEdit = {
  recordIndex: number;
  tags: string[];
};

type AuthoringQuery = {
  search: string;
  filters: Array<{ field: string; operator: string; value?: AuthoringValue | null }>;
  sort: { field: string; direction: string } | null;
};

type BatchTarget = {
  recordIndex?: number;
  addedRecordIndex?: number;
  field: string;
};

type BatchCellChange = {
  recordIndex: number | null;
  addedRecordIndex: number | null;
  field: string;
  before: AuthoringValue;
  after: AuthoringValue;
};

type AuthoringClipboardShape = {
  rows: number;
  columns: number;
};

type AuthoringBatchPreview = {
  source: SourceEditPreview;
  targetCount: number;
  changedCellCount: number;
  changes: BatchCellChange[];
};

type AuthoringBatchCopyResult = {
  clipboardText: string;
  targetCount: number;
};

type DataFileQueryResult = {
  orderedRecordIndices: number[];
  totalCount: number;
  displayedCount: number;
  query: AuthoringQuery;
};

type AddedRecordDraft = {
  draftId: string;
  values: Record<string, AuthoringValue>;
  tags: string[];
};

type SourceEditPreview = {
  candidateSource: string;
  candidateContentIdentity: string;
  changed: boolean;
  validation: ValidationReport;
};

type SourceContentState = {
  path: string;
  contentIdentity: string;
  source: string;
};

type SourceSaveReport = {
  status: "success" | "conflict" | "failure" | "outcome_unknown";
  path: string;
  snapshot: DataFileSnapshot | null;
  current: SourceContentState | null;
  diagnostic: Diagnostic | null;
};

type BuildResponse = {
  project: ProjectInfo;
  schemaSourceContentHash: string;
  artifactRoot: string;
  csharpOutput: string;
  binaryOutput: string;
  cache: string;
  generatedFiles: string[];
  dryRun: boolean;
  profile: string | null;
};

type WorkspaceState =
  | { kind: "loading"; previous: AuthoringWorkspace | null }
  | { kind: "ready"; workspace: AuthoringWorkspace }
  | { kind: "error"; diagnostic: ApiDiagnostic; previous: AuthoringWorkspace | null };

function isTextEditingTarget(target: EventTarget | null): boolean {
  const element = target instanceof HTMLElement ? target : null;
  if (!element) return false;
  return element instanceof HTMLInputElement
    || element instanceof HTMLTextAreaElement
    || element.isContentEditable;
}

type OperationState<T> =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "done"; value: T }
  | { kind: "error"; diagnostic: ApiDiagnostic };

type EditorState = {
  snapshot: DataFileSnapshot;
  edits: Record<string, AuthoringEdit>;
  addedRecords: AddedRecordDraft[];
  pendingDeletes: number[];
  tagEdits: Record<string, string[]>;
  queryResult: DataFileQueryResult | null;
  historyPast: MutationHistoryState[];
  historyFuture: MutationHistoryState[];
  preview: SourceEditPreview;
  previewState: "current" | "pending" | "unavailable";
  previewError: ApiDiagnostic | null;
  revision: number;
  saving: boolean;
  loadError: ApiDiagnostic | null;
  conflict: SourceContentState | null;
  saveStatus: SourceSaveReport["status"] | null;
  saveDiagnostic: Diagnostic | null;
  view: "grid" | "diff" | "compare";
};

type MutationHistoryState = {
  edits: Record<string, AuthoringEdit>;
  addedRecords: AddedRecordDraft[];
  pendingDeletes: number[];
  tagEdits: Record<string, string[]>;
};

type Surface = "editor" | "overview" | "settings" | "delivery" | "create";

type PendingAction =
  | { kind: "reload" }
  | { kind: "open"; projectPath: string }
  | { kind: "create" }
  | { kind: "close" };

function asApiError(error: unknown): ApiError {
  if (
    typeof error === "object" &&
    error !== null &&
    "diagnostic" in error &&
    typeof error.diagnostic === "object" &&
    error.diagnostic !== null
  ) {
    return error as ApiError;
  }
  return {
    diagnostic: {
      code: "E-GUI-UNKNOWN",
      kind: "external_tool",
      message: String(error),
      source: null,
      line: null,
      column: null,
      schemaPath: null,
      valuePath: null,
      recordIdentity: null,
      suggestion: null,
      relatedRequirements: [],
    },
  };
}

function cellKey(recordIndex: number, field: string): string {
  return `${recordIndex}:${field}`;
}

function editorFromSnapshot(snapshot: DataFileSnapshot): EditorState {
  return {
    snapshot,
    edits: {},
    addedRecords: [],
    pendingDeletes: [],
    tagEdits: {},
    queryResult: null,
    historyPast: [],
    historyFuture: [],
    preview: {
      candidateSource: snapshot.baseSource,
      candidateContentIdentity: snapshot.baseContentIdentity,
      changed: false,
      validation: snapshot.validation,
    },
    previewState: "current",
    previewError: null,
    revision: 0,
    saving: false,
    loadError: null,
    conflict: null,
    saveStatus: null,
    saveDiagnostic: null,
    view: "grid",
  };
}

function baseCellValue(snapshot: DataFileSnapshot, recordIndex: number, field: string): AuthoringValue {
  return snapshot.rows
    .find((row) => row.recordIndex === recordIndex)
    ?.cells.find((cell) => cell.field === field)?.value ?? nullAuthoringValue();
}

function currentCellValue(editor: EditorState, recordIndex: number, field: string): AuthoringValue {
  return editor.edits[cellKey(recordIndex, field)]?.value ?? baseCellValue(editor.snapshot, recordIndex, field);
}

function editorIsDirty(editor: EditorState): boolean {
  return Object.keys(editor.edits).length > 0
    || editor.addedRecords.length > 0
    || editor.pendingDeletes.length > 0
    || Object.keys(editor.tagEdits).length > 0;
}

function draftCellKey(draftId: string, field: string): string {
  return `draft:${draftId}:${field}`;
}

function addCapability(snapshot: DataFileSnapshot): DataEditorAddCapability {
  return snapshot.addRow ?? {
    supported: false,
    reason: "Add Row capability is unavailable for this source snapshot.",
  };
}

function queryInputValue(column: DataEditorColumn | undefined, text: string): AuthoringValue {
  const shape = column?.shape?.shape;
  if (!shape) return { kind: "string", value: text };
  if (shape.kind === "primitive") {
    if (shape.primitive === "bool" && (text === "true" || text === "false")) {
      return { kind: "bool", value: text === "true" };
    }
    if (shape.primitive !== "string") return { kind: "number", value: text };
    return { kind: "string", value: text };
  }
  if (shape.kind === "value_object" && shape.underlying !== "string") {
    return shape.underlying === "bool" && (text === "true" || text === "false")
      ? { kind: "bool", value: text === "true" }
      : { kind: "number", value: text };
  }
  return { kind: "string", value: text };
}

function mutationForEditor(editor: EditorState): AuthoringRecordMutation {
  return {
    edits: Object.values(editor.edits),
    addedRecords: editor.addedRecords.map((draft) => ({
      fields: editor.snapshot.columns.map((column) => ({
        field: column.name,
        value: draft.values[column.name] ?? nullAuthoringValue(),
      })),
      tags: draft.tags,
    })),
    deletedRecordIndices: [...editor.pendingDeletes].sort((left, right) => left - right),
    tagEdits: Object.entries(editor.tagEdits).map(([recordIndex, tags]) => ({
      recordIndex: Number(recordIndex),
      tags,
    })),
  };
}

function mutationHistoryState(editor: EditorState): MutationHistoryState {
  return {
    edits: editor.edits,
    addedRecords: editor.addedRecords,
    pendingDeletes: editor.pendingDeletes,
    tagEdits: editor.tagEdits,
  };
}

const HISTORY_LIMIT = 50;

function boundedHistoryPush(history: MutationHistoryState[], state: MutationHistoryState): MutationHistoryState[] {
  if (history.length >= HISTORY_LIMIT) {
    window.alert("Undo history is full. The oldest undo entry will be discarded after this edit; the current buffer is preserved.");
  }
  return [...history, state].slice(-HISTORY_LIMIT);
}

function mutationHistoryFields(editor: EditorState): Pick<EditorState, "historyPast" | "historyFuture"> {
  return {
    historyPast: boundedHistoryPush(editor.historyPast, mutationHistoryState(editor)),
    historyFuture: [],
  };
}

function sourceName(path: string): string {
  return path.split("/").at(-1) ?? path;
}

function normalizePath(path: string): string {
  return path.replaceAll("\\", "/");
}

function diagnosticRecordIndex(diagnostic: Diagnostic): number | null {
  const match = diagnostic.record_identity?.match(/^record\[(\d+)]$/);
  return match ? Number(match[1]) : null;
}

function diagnosticField(diagnostic: Diagnostic): string | null {
  return diagnostic.message.match(/field `([^`]+)`/)?.[1] ?? null;
}

function diagnosticCellKey(editor: EditorState, candidateRecordIndex: number, field: string): string | null {
  let remaining = candidateRecordIndex;
  for (const row of editor.snapshot.rows) {
    if (editor.pendingDeletes.includes(row.recordIndex)) continue;
    if (remaining === 0) return cellKey(row.recordIndex, field);
    remaining -= 1;
  }
  const draft = editor.addedRecords[remaining];
  return draft ? draftCellKey(draft.draftId, field) : null;
}

function focusElement(element: HTMLElement | null): boolean {
  if (!element) return false;
  const focusable = element.matches("input, select, textarea, button, [tabindex='0']")
    ? element
    : element.querySelector<HTMLElement>("input:not([disabled]), select:not([disabled]), textarea:not([disabled]), button:not([disabled]), [tabindex='0']");
  if (!focusable) return false;
  focusable.focus();
  return true;
}

function focusValuePathOrCell(cell: string, valuePath: string | null): boolean {
  if (valuePath !== null) {
    const nested = document.querySelector<HTMLElement>(`[data-value-path="${CSS.escape(valuePath)}"]`);
    if (focusElement(nested)) return true;
  }
  return focusElement(document.querySelector<HTMLElement>(`[data-cell="${CSS.escape(cell)}"]`));
}

function App() {
  const [recoveries, setRecoveries] = useState<Record<string, MigrationResult>>({});
  const recoveryRef = useRef<Record<string, MigrationResult>>({});
  const [tableEpoch, setTableEpoch] = useState(0);
  const [migrationBusyRoot, setMigrationBusyRoot] = useState<string | null>(null);
  const migrationBusyRef = useRef<string | null>(null);
  const sourceMutationBlocked = useCallback((root: string) => !!recoveryRef.current[root] || migrationBusyRef.current === root, []);
  const recordRecovery = useCallback((root: string, result: MigrationResult | null) => {
    const next = { ...recoveryRef.current };
    if (result?.state === "recovery_required") next[root] = result; else delete next[root];
    recoveryRef.current = next; setRecoveries(next);
  }, []);
  const [creationOpen, setCreationOpen] = useState(false);
  const [creationTarget, setCreationTarget] = useState({ root: "", folder: "" });
  const [revealCreated, setRevealCreated] = useState<{ path: string; root: string } | null>(null);
  const [surface, setSurface] = useState<Surface>("editor");
  const [selectedProfile, setSelectedProfile] = useState("");
  const [settingsDirty, setSettingsDirty] = useState(false);
  const settingsSaveRef = useRef<() => Promise<boolean>>(async () => true);
  const settingsDirtyRef = useRef(false);
  const [deliveryBusy, setDeliveryBusy] = useState(false);
  const deliveryBusyRef = useRef(false);
  const [configRevision, setConfigRevision] = useState(0);

  const [workspaceState, setWorkspaceState] = useState<WorkspaceState>({
    kind: "loading",
    previous: null,
  });
  const [projectPathInput, setProjectPathInput] = useState("");
  const [activePath, setActivePath] = useState<string | null>(null);
  const [editors, setEditors] = useState<Record<string, EditorState>>({});
  const [manualValidation, setManualValidation] = useState<OperationState<ValidationReport>>({ kind: "idle" });
  const [buildState, setBuildState] = useState<OperationState<BuildResponse>>({ kind: "idle" });
  const [problemsOpen, setProblemsOpen] = useState(true);
  const [pendingAction, setPendingAction] = useState<PendingAction | null>(null);
  const [pendingActionBusy, setPendingActionBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [loadingPaths, setLoadingPaths] = useState<Set<string>>(() => new Set());
  const [fileOpenErrors, setFileOpenErrors] = useState<Record<string, ApiDiagnostic>>({});
  const previewTimers = useRef(new Map<string, number>());
  const editorsRef = useRef(editors);
  const workspaceStateRef = useRef(workspaceState);
  const workspaceGeneration = useRef(0);
  const draftSequence = useRef(0);
  const cellFocusStart = useRef(new Map<string, AuthoringValue>());
  const historyEditKey = useRef<string | null>(null);
  const pendingCellFocus = useRef<string | null>(null);
  const pendingValueFocus = useRef<string | null>(null);
  const pendingRecordFocus = useRef<{ path: string; recordIndex: number; expectedIdentity: string } | null>(null);

  useEffect(() => {
    editorsRef.current = editors;
  }, [editors]);

  useEffect(() => {
    workspaceStateRef.current = workspaceState;
  }, [workspaceState]);

  useEffect(() => {
    settingsDirtyRef.current = settingsDirty;
  }, [settingsDirty]);

  useEffect(() => {
    deliveryBusyRef.current = deliveryBusy;
  }, [deliveryBusy]);

  const registerSettingsSave = useCallback((save: () => Promise<boolean>) => {
    settingsSaveRef.current = save;
  }, []);

  useEffect(() => {
    if (!activePath || loadingPaths.has(activePath)) return;
    const recordFocus = pendingRecordFocus.current;
    if (recordFocus?.path === activePath) {
      const editor = editors[activePath];
      if (editor && editor.snapshot.baseContentIdentity !== recordFocus.expectedIdentity) {
        setNotice("Overview snapshot is stale; refresh Overview before opening this occurrence.");
        pendingRecordFocus.current = null;
        return;
      }
      if (editor?.queryResult) {
        setEditors((current) => current[activePath]
          ? { ...current, [activePath]: { ...current[activePath], queryResult: null } }
          : current);
        return;
      }
      const firstField = editor?.snapshot.columns[0]?.name;
      if (editor && firstField) {
        pendingCellFocus.current = cellKey(recordFocus.recordIndex, firstField);
        pendingValueFocus.current = null;
        pendingRecordFocus.current = null;
      }
    }
    const key = pendingCellFocus.current;
    if (!key) return;
    if (focusValuePathOrCell(key, pendingValueFocus.current)) {
      pendingCellFocus.current = null;
      pendingValueFocus.current = null;
    }
  }, [activePath, editors, loadingPaths]);

  const workspace = workspaceState.kind === "ready"
    ? workspaceState.workspace
    : workspaceState.kind === "error"
      ? workspaceState.previous
      : null;
  const projectRoot = workspace?.project.project_root ?? null;
  const recovery = projectRoot ? recoveries[projectRoot] : null;
  const mutationBlocked = !!recovery || (!!projectRoot && migrationBusyRoot === projectRoot);
  const activeFile = workspace?.files.find((file) => file.path === activePath) ?? null;
  const activeEditor = activePath ? editors[activePath] ?? null : null;
  const activeLoading = activePath ? loadingPaths.has(activePath) : false;
  const activeLoadDiagnostic = activeEditor && editorIsDirty(activeEditor)
    ? null
    : activeEditor?.loadError ?? (activePath ? fileOpenErrors[activePath] ?? null : null);
  const dirtyCount = Object.values(editors).filter(editorIsDirty).length;
  const totalDirtyCount = dirtyCount + (settingsDirty ? 1 : 0);

  const showNotice = useCallback((message: string) => {
    setNotice(message);
    window.setTimeout(() => setNotice(null), 2600);
  }, []);

  const refreshWorkspaceAfterConfigSave = useCallback(async (): Promise<boolean> => {
    const state = workspaceStateRef.current;
    const current = state.kind === "ready" ? state.workspace : state.previous;
    const root = current?.project.project_root;
    if (!root) return false;
    try {
      const next = await invoke<AuthoringWorkspace>("authoring_workspace", { projectPath: root });
      if (next.project.project_root !== root) return false;
      setWorkspaceState({ kind: "ready", workspace: next });
      setConfigRevision((revision) => revision + 1);
      setSelectedProfile((selected) => selected && next.project.profiles?.some((item) => item.name === selected) ? selected : "");
      return true;
    } catch (error) {
      setWorkspaceState({ kind: "error", diagnostic: asApiError(error).diagnostic, previous: current });
      showNotice("Settings were saved, but the project binding is invalid; source Save All stopped.");
      return false;
    }
  }, [showNotice]);

  const openDataFile = useCallback(async (root: string, path: string, force = false) => {
    const existing = editorsRef.current[path];
    if (!force && existing && !existing.loadError) {
      return;
    }
    const generation = workspaceGeneration.current;
    setLoadingPaths((current) => {
      const next = new Set(current);
      next.add(path);
      return next;
    });
    try {
      const snapshot = await invoke<DataFileSnapshot>("open_data_file", {
        projectPath: root,
        relativePath: path,
      });
      if (workspaceGeneration.current !== generation) return;
      setFileOpenErrors((current) => {
        if (!(path in current)) return current;
        const next = { ...current };
        delete next[path];
        return next;
      });
      setEditors((current) => ({ ...current, [path]: editorFromSnapshot(snapshot) }));
    } catch (error) {
      if (workspaceGeneration.current !== generation) return;
      const diagnostic = asApiError(error).diagnostic;
      setFileOpenErrors((current) => ({ ...current, [path]: diagnostic }));
      setEditors((current) => {
        const currentEditor = current[path];
        if (!currentEditor) return current;
        if (editorIsDirty(currentEditor)) {
          return {
            ...current,
            [path]: {
              ...currentEditor,
              saveDiagnostic: apiDiagnosticToDiagnostic(diagnostic),
            },
          };
        }
        return {
          ...current,
          [path]: { ...currentEditor, loadError: diagnostic },
        };
      });
    } finally {
      if (workspaceGeneration.current === generation) {
        setLoadingPaths((current) => {
          const next = new Set(current);
          next.delete(path);
          return next;
        });
      }
    }
  }, []);

  const loadWorkspace = useCallback(async (requestedProject: string | null) => {
    const generation = workspaceGeneration.current + 1;
    workspaceGeneration.current = generation;
    setCreationOpen(false);
    setRevealCreated(null);
    setCreationTarget({ root: "", folder: "" });
    setSelectedProfile("");
    for (const timer of previewTimers.current.values()) window.clearTimeout(timer);
    previewTimers.current.clear();
    setLoadingPaths(new Set());
    setFileOpenErrors({});
    setSettingsDirty(false);
    const previous = workspaceStateRef.current.kind === "ready"
      ? workspaceStateRef.current.workspace
      : workspaceStateRef.current.previous;
    setWorkspaceState({ kind: "loading", previous });
    setManualValidation({ kind: "idle" });
    setBuildState({ kind: "idle" });
    try {
      const next = await invoke<AuthoringWorkspace>("authoring_workspace", {
        projectPath: requestedProject,
      });
      const recovery = await invoke<MigrationResult | null>("migration_recovery_status", { projectPath: next.project.project_root });
      if (workspaceGeneration.current !== generation) return;
      recordRecovery(next.project.project_root, recovery);
      setWorkspaceState({ kind: "ready", workspace: next });
      setProjectPathInput(next.project.project_root);
      setEditors({});
      const first = next.files.find((file) => file.kind === "data") ?? next.files[0] ?? null;
      setActivePath(first?.path ?? null);
      if (first?.kind === "data") {
        await openDataFile(next.project.project_root, first.path, true);
      }
    } catch (error) {
      if (workspaceGeneration.current !== generation) return;
      setWorkspaceState({
        kind: "error",
        diagnostic: asApiError(error).diagnostic,
        previous,
      });
    }
  }, [recordRecovery, openDataFile]);

  useEffect(() => {
    void loadWorkspace(null);
  }, [loadWorkspace]);

  const schedulePreview = useCallback((root: string, path: string, editor: EditorState) => {
    const existing = previewTimers.current.get(path);
    if (existing !== undefined) {
      window.clearTimeout(existing);
    }
    // WHY: revision restarts after Save/Reload; snapshot identity rejects responses from the previous buffer.
    // EVIDENCE: GUI-DATA-VAL-005; regression tests in tests/authoring.test.tsx.
    const revision = editor.revision;
    const generation = workspaceGeneration.current;
    const timer = window.setTimeout(async () => {
      previewTimers.current.delete(path);
      try {
        const preview = await invoke<SourceEditPreview>("preview_data_file", {
          projectPath: root,
          relativePath: path,
          baseSource: editor.snapshot.baseSource,
          ...mutationForEditor(editor),
        });
        if (workspaceGeneration.current !== generation) return;
        setEditors((current) => {
          const latest = current[path];
          if (!latest || latest.revision !== revision || latest.snapshot !== editor.snapshot) {
            return current;
          }
          return {
            ...current,
            [path]: {
              ...latest,
              edits: preview.changed ? latest.edits : {},
              addedRecords: preview.changed ? latest.addedRecords : [],
              pendingDeletes: preview.changed ? latest.pendingDeletes : [],
              tagEdits: preview.changed ? latest.tagEdits : {},
              preview,
              previewState: "current",
              previewError: null,
            },
          };
        });
      } catch (error) {
        if (workspaceGeneration.current !== generation) return;
        const diagnostic = asApiError(error).diagnostic;
        setEditors((current) => {
          const latest = current[path];
          if (!latest || latest.revision !== revision || latest.snapshot !== editor.snapshot) {
            return current;
          }
          return {
            ...current,
            [path]: {
              ...latest,
              previewState: "unavailable",
              previewError: diagnostic,
            },
          };
        });
      }
    }, 320);
    previewTimers.current.set(path, timer);
  }, []);

  const updateCell = useCallback((path: string, recordIndex: number, field: string, value: AuthoringValue) => {
    if (!projectRoot) return;
    setEditors((current) => {
      const editor = current[path];
      if (!editor || editor.saving || editor.pendingDeletes.includes(recordIndex)) return current;
      const nextEdits = { ...editor.edits };
      const key = cellKey(recordIndex, field);
      if (authoringValuesEqual(value, currentCellValue(editor, recordIndex, field))) return current;
      const captureHistory = historyEditKey.current === key;
      if (captureHistory) historyEditKey.current = null;
      if (authoringValuesEqual(value, baseCellValue(editor.snapshot, recordIndex, field))) {
        delete nextEdits[key];
      } else {
        nextEdits[key] = { recordIndex, field, value };
      }
      const next: EditorState = {
        ...editor,
        edits: nextEdits,
        revision: editor.revision + 1,
        previewState: "pending",
        previewError: null,
        saveDiagnostic: null,
        queryResult: null,
        ...(captureHistory ? mutationHistoryFields(editor) : {}),
      };
      schedulePreview(projectRoot, path, next);
      return { ...current, [path]: next };
    });
  }, [projectRoot, schedulePreview]);

  const addRow = useCallback((path: string) => {
    if (!projectRoot) return;
    setEditors((current) => {
      const editor = current[path];
      if (!editor || editor.saving || !addCapability(editor.snapshot).supported) return current;
      const draftId = `draft-${draftSequence.current + 1}`;
      draftSequence.current += 1;
      const values: Record<string, AuthoringValue> = {};
      for (const column of editor.snapshot.columns) values[column.name] = nullAuthoringValue();
      const next: EditorState = {
        ...editor,
        addedRecords: [...editor.addedRecords, { draftId, values, tags: [] }],
        revision: editor.revision + 1,
        previewState: "pending",
        previewError: null,
        saveDiagnostic: null,
        queryResult: null,
        ...mutationHistoryFields(editor),
      };
      const firstField = editor.snapshot.columns[0]?.name;
      if (firstField) {
        pendingCellFocus.current = draftCellKey(draftId, firstField);
        pendingValueFocus.current = null;
      }
      schedulePreview(projectRoot, path, next);
      return { ...current, [path]: next };
    });
  }, [projectRoot, schedulePreview]);

  const updateDraftCell = useCallback((path: string, draftId: string, field: string, value: AuthoringValue) => {
    if (!projectRoot) return;
    setEditors((current) => {
      const editor = current[path];
      if (!editor || editor.saving) return current;
      if (!editor.addedRecords.some((draft) => draft.draftId === draftId)) return current;
      const currentDraft = editor.addedRecords.find((draft) => draft.draftId === draftId);
      const currentValue = currentDraft?.values[field] ?? nullAuthoringValue();
      if (authoringValuesEqual(value, currentValue)) return current;
      const key = draftCellKey(draftId, field);
      const captureHistory = historyEditKey.current === key;
      if (captureHistory) historyEditKey.current = null;
      const nextDrafts = editor.addedRecords.map((draft) => draft.draftId === draftId
        ? { ...draft, values: { ...draft.values, [field]: value } }
        : draft);
      const next: EditorState = {
        ...editor,
        addedRecords: nextDrafts,
        revision: editor.revision + 1,
        previewState: "pending",
        previewError: null,
        saveDiagnostic: null,
        queryResult: null,
        ...(captureHistory ? mutationHistoryFields(editor) : {}),
      };
      schedulePreview(projectRoot, path, next);
      return { ...current, [path]: next };
    });
  }, [projectRoot, schedulePreview]);

  const updateExistingTags = useCallback((path: string, recordIndex: number, tags: string[]) => {
    if (!projectRoot) return;
    setEditors((current) => {
      const editor = current[path];
      if (!editor || editor.saving || editor.pendingDeletes.includes(recordIndex)) return current;
      const base = editor.snapshot.rows.find((row) => row.recordIndex === recordIndex)?.tags ?? [];
      const nextTags = { ...editor.tagEdits };
      if (JSON.stringify(base) === JSON.stringify(tags)) delete nextTags[String(recordIndex)];
      else nextTags[String(recordIndex)] = tags;
      if (JSON.stringify(nextTags) === JSON.stringify(editor.tagEdits)) return current;
      const next: EditorState = {
        ...editor,
        tagEdits: nextTags,
        revision: editor.revision + 1,
        previewState: "pending",
        previewError: null,
        saveDiagnostic: null,
        queryResult: null,
        ...mutationHistoryFields(editor),
      };
      schedulePreview(projectRoot, path, next);
      return { ...current, [path]: next };
    });
  }, [projectRoot, schedulePreview]);

  const updateDraftTags = useCallback((path: string, draftId: string, tags: string[]) => {
    setEditors((current) => {
      const editor = current[path];
      if (!editor || editor.saving) return current;
      const nextDrafts = editor.addedRecords.map((draft) => draft.draftId === draftId ? { ...draft, tags } : draft);
      if (nextDrafts.every((draft, index) => draft === editor.addedRecords[index])) return current;
      const next: EditorState = {
        ...editor,
        addedRecords: nextDrafts,
        revision: editor.revision + 1,
        previewState: "pending",
        previewError: null,
        saveDiagnostic: null,
        queryResult: null,
        ...mutationHistoryFields(editor),
      };
      if (projectRoot) schedulePreview(projectRoot, path, next);
      return { ...current, [path]: next };
    });
  }, [projectRoot, schedulePreview]);

  const deleteExistingRow = useCallback((path: string, recordIndex: number) => {
    if (!projectRoot) return;
    setEditors((current) => {
      const editor = current[path];
      if (!editor || editor.saving || editor.pendingDeletes.includes(recordIndex)) return current;
      const next: EditorState = {
        ...editor,
        pendingDeletes: [...editor.pendingDeletes, recordIndex].sort((left, right) => left - right),
        revision: editor.revision + 1,
        previewState: "pending",
        previewError: null,
        saveDiagnostic: null,
        queryResult: null,
        ...mutationHistoryFields(editor),
      };
      schedulePreview(projectRoot, path, next);
      return { ...current, [path]: next };
    });
  }, [projectRoot, schedulePreview]);

  const undoExistingDelete = useCallback((path: string, recordIndex: number) => {
    if (!projectRoot) return;
    setEditors((current) => {
      const editor = current[path];
      if (!editor || editor.saving || !editor.pendingDeletes.includes(recordIndex)) return current;
      const next: EditorState = {
        ...editor,
        pendingDeletes: editor.pendingDeletes.filter((index) => index !== recordIndex),
        revision: editor.revision + 1,
        previewState: "pending",
        previewError: null,
        saveDiagnostic: null,
        queryResult: null,
        ...mutationHistoryFields(editor),
      };
      schedulePreview(projectRoot, path, next);
      return { ...current, [path]: next };
    });
  }, [projectRoot, schedulePreview]);

  const deleteDraftRow = useCallback((path: string, draftId: string) => {
    if (!projectRoot) return;
    setEditors((current) => {
      const editor = current[path];
      if (!editor || editor.saving || !editor.addedRecords.some((draft) => draft.draftId === draftId)) return current;
      const next: EditorState = {
        ...editor,
        addedRecords: editor.addedRecords.filter((draft) => draft.draftId !== draftId),
        revision: editor.revision + 1,
        previewState: "pending",
        previewError: null,
        saveDiagnostic: null,
        queryResult: null,
        ...mutationHistoryFields(editor),
      };
      schedulePreview(projectRoot, path, next);
      return { ...current, [path]: next };
    });
  }, [projectRoot, schedulePreview]);

  const undoBuffer = useCallback((path: string) => {
    if (!projectRoot) return;
    setEditors((current) => {
      const editor = current[path];
      const previous = editor?.historyPast.at(-1);
      if (!editor || editor.saving || !previous) return current;
      const next: EditorState = {
        ...editor,
        ...previous,
        historyPast: editor.historyPast.slice(0, -1),
        historyFuture: [...editor.historyFuture, mutationHistoryState(editor)],
        revision: editor.revision + 1,
        previewState: "pending",
        previewError: null,
        saveDiagnostic: null,
        queryResult: null,
      };
      schedulePreview(projectRoot, path, next);
      return { ...current, [path]: next };
    });
  }, [projectRoot, schedulePreview]);

  const redoBuffer = useCallback((path: string) => {
    if (!projectRoot) return;
    setEditors((current) => {
      const editor = current[path];
      const future = editor?.historyFuture.at(-1);
      if (!editor || editor.saving || !future) return current;
      const next: EditorState = {
        ...editor,
        ...future,
        historyPast: boundedHistoryPush(editor.historyPast, mutationHistoryState(editor)),
        historyFuture: editor.historyFuture.slice(0, -1),
        revision: editor.revision + 1,
        previewState: "pending",
        previewError: null,
        saveDiagnostic: null,
        queryResult: null,
      };
      schedulePreview(projectRoot, path, next);
      return { ...current, [path]: next };
    });
  }, [projectRoot, schedulePreview]);

  const applyBatchPreview = useCallback((path: string, batch: AuthoringBatchPreview) => {
    setEditors((current) => {
      const editor = current[path];
      if (!editor || editor.saving) return current;
      if (batch.changedCellCount === 0) return current;
      const edits = { ...editor.edits };
      const addedRecords = editor.addedRecords.map((draft) => ({ ...draft, values: { ...draft.values } }));
      for (const change of batch.changes) {
        if (change.recordIndex !== null) {
          const key = cellKey(change.recordIndex, change.field);
          if (authoringValuesEqual(change.after, baseCellValue(editor.snapshot, change.recordIndex, change.field))) delete edits[key];
          else edits[key] = { recordIndex: change.recordIndex, field: change.field, value: change.after };
        } else if (change.addedRecordIndex !== null) {
          const draft = addedRecords[change.addedRecordIndex];
          if (draft) draft.values[change.field] = change.after;
        }
      }
      return {
        ...current,
        [path]: {
          ...editor,
          edits,
          addedRecords,
          preview: batch.source,
          previewState: "current",
          previewError: null,
          queryResult: null,
          revision: editor.revision + 1,
          saveDiagnostic: null,
          ...mutationHistoryFields(editor),
        },
      };
    });
  }, []);

  const updateQueryResult = useCallback((path: string, result: DataFileQueryResult | null) => {
    setEditors((current) => current[path]
      ? { ...current, [path]: { ...current[path], queryResult: result } }
      : current);
  }, []);

  const saveFile = useCallback(async (path: string, overwriteExpectedIdentity?: string): Promise<boolean> => {
    const state = workspaceStateRef.current;
    const root = state.kind === "ready" ? state.workspace.project.project_root : state.previous?.project.project_root;
    const editor = editorsRef.current[path];
    if (root && sourceMutationBlocked(root)) return false;
    if (!root || !editor || !editorIsDirty(editor)) return true;
    if (editor.saveStatus === "outcome_unknown" && !overwriteExpectedIdentity) {
      showNotice("Recheck the source before saving again because the previous save outcome is unknown.");
      return false;
    }
    const generation = workspaceGeneration.current;

    setEditors((current) => current[path]
      ? { ...current, [path]: { ...current[path], saving: true, saveDiagnostic: null } }
      : current);
    try {
      const report = await invoke<SourceSaveReport>("save_data_file", {
        projectPath: root,
        relativePath: path,
        baseSource: editor.snapshot.baseSource,
        baseContentIdentity: editor.snapshot.baseContentIdentity,
        ...mutationForEditor(editor),
        overwriteExpectedIdentity: overwriteExpectedIdentity ?? null,
      });
      if (workspaceGeneration.current !== generation) return false;
      if (report.status === "success" && report.snapshot) {
        setEditors((current) => ({
          ...current,
          [path]: { ...editorFromSnapshot(report.snapshot!), view: current[path]?.view ?? "grid" },
        }));
        showNotice(`${sourceName(path)} saved`);
        return true;
      }
      setEditors((current) => {
        const latest = current[path];
        if (!latest) return current;
        return {
          ...current,
          [path]: {
            ...latest,
            saving: false,
            conflict: report.status === "conflict" ? report.current : latest.conflict,
            saveStatus: report.status,
            saveDiagnostic: report.diagnostic,
            view: report.status === "conflict" ? "compare" : latest.view,
          },
        };
      });
      return false;
    } catch (error) {
      if (workspaceGeneration.current !== generation) return false;
      const diagnostic = asApiError(error).diagnostic;
      setEditors((current) => current[path]
        ? {
            ...current,
            [path]: {
              ...current[path],
              saving: false,
              saveStatus: "failure",
              saveDiagnostic: {
                code: diagnostic.code,
                kind: diagnostic.kind,
                message: diagnostic.message,
                source: diagnostic.source,
                line: diagnostic.line,
                column: diagnostic.column,
                schema_path: diagnostic.schemaPath,
                record_identity: diagnostic.recordIdentity,
                suggestion: diagnostic.suggestion,
                related_requirements: diagnostic.relatedRequirements,
              },
            },
          }
        : current);
      return false;
    }
  }, [sourceMutationBlocked, showNotice]);

  const saveAll = useCallback(async (): Promise<boolean> => {
    if (settingsDirty && !(await settingsSaveRef.current())) return false;
    const paths = Object.entries(editorsRef.current)
      .filter(([, editor]) => editorIsDirty(editor))
      .map(([path]) => path);
    let allSaved = true;
    for (const path of paths) {
      if (!(await saveFile(path))) {
        allSaved = false;
      }
    }
    return allSaved;
  }, [saveFile, settingsDirty]);

  const performAction = useCallback(async (action: PendingAction) => {
    setPendingAction(null);
    if (action.kind === "close") {
      await getCurrentWindow().destroy();
      return;
    }
    if (action.kind === "create") {
      setSurface("create");
      return;
    }
    if (action.kind === "reload") {
      const root = workspaceStateRef.current.kind === "ready"
        ? workspaceStateRef.current.workspace.project.project_root
        : workspaceStateRef.current.previous?.project.project_root ?? null;
      if (root) await loadWorkspace(root);
      return;
    }
    await loadWorkspace(action.projectPath);
  }, [loadWorkspace]);

  const requestAction = useCallback((action: PendingAction) => {
    const hasDirty = settingsDirty || Object.values(editorsRef.current).some(editorIsDirty);
    if (deliveryBusy || hasDirty) {
      setPendingAction(action);
      if (deliveryBusy) showNotice("A Build or Publish operation is running. Project navigation will wait until it finishes.");
    } else {
      void performAction(action);
    }
  }, [deliveryBusy, performAction, settingsDirty, showNotice]);

  useEffect(() => {
    const listener = getCurrentWindow().onCloseRequested((event) => {
      if (deliveryBusyRef.current || settingsDirtyRef.current || Object.values(editorsRef.current).some(editorIsDirty)) {
        event.preventDefault();
        setPendingAction({ kind: "close" });
      }
    });
    return () => {
      void listener.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
        event.preventDefault();
        if (surface === "settings") {
          if (deliveryBusy) showNotice("Settings Save is blocked while Build or Publish is running.");
          else void settingsSaveRef.current();
        } else if (activePath) void saveFile(activePath);
      } else if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "z") {
        if (isTextEditingTarget(event.target)) return;
        event.preventDefault();
        if (activePath) {
          if (event.shiftKey) redoBuffer(activePath);
          else undoBuffer(activePath);
        }
      } else if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "y") {
        if (isTextEditingTarget(event.target)) return;
        event.preventDefault();
        if (activePath) redoBuffer(activePath);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [activePath, deliveryBusy, redoBuffer, saveFile, showNotice, surface, undoBuffer]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      const state = workspaceStateRef.current;
      const root = state.kind === "ready" ? state.workspace.project.project_root : null;
      if (!root) return;
      const generation = workspaceGeneration.current;
      for (const [path, editor] of Object.entries(editorsRef.current)) {
        if (editor.saving) continue;
        void invoke<SourceContentState>("source_content", {
          projectPath: root,
          relativePath: path,
        }).then((current) => {
          if (workspaceGeneration.current !== generation) return;
          const latest = editorsRef.current[path];
          if (!latest || latest.saving) return;
          if (current.contentIdentity === latest.snapshot.baseContentIdentity) {
            if (latest.conflict || latest.loadError || latest.saveStatus === "conflict") {
              if (latest.loadError) {
                setFileOpenErrors((all) => {
                  if (!(path in all)) return all;
                  const next = { ...all };
                  delete next[path];
                  return next;
                });
              }
              setEditors((all) => all[path]
                ? {
                    ...all,
                    [path]: {
                      ...all[path],
                      conflict: null,
                      loadError: null,
                      saveStatus: all[path].saveStatus === "conflict" ? null : all[path].saveStatus,
                    },
                  }
                : all);
            }
            return;
          }
          if (editorIsDirty(latest)) {
            setEditors((all) => all[path]
              ? { ...all, [path]: { ...all[path], conflict: current, saveStatus: "conflict", loadError: null } }
              : all);
          } else {
            void openDataFile(root, path, true);
          }
        }).catch(() => {
          // A transient polling failure must not discard an editor buffer.
        });
      }
    }, 1600);
    return () => window.clearInterval(timer);
  }, [openDataFile]);

  const validateDisk = useCallback(async () => {
    if (!workspace?.capabilities.validate || !projectRoot) return;
    const generation = workspaceGeneration.current;
    setManualValidation({ kind: "loading" });
    try {
      const report = await invoke<ValidationReport>("validate", { projectPath: projectRoot });
      if (workspaceGeneration.current !== generation) return;
      setManualValidation({ kind: "done", value: report });
      setProblemsOpen(true);
    } catch (error) {
      if (workspaceGeneration.current !== generation) return;
      setManualValidation({ kind: "error", diagnostic: asApiError(error).diagnostic });
    }
  }, [projectRoot, workspace]);

  const runBuild = useCallback(async () => {
    if (!workspace?.capabilities.build || !projectRoot || sourceMutationBlocked(projectRoot) || buildState.kind === "loading" || deliveryBusy) return;
    if (dirtyCount > 0) {
      showNotice("Build uses saved source only; unsaved changes are not included.");
    }
    const generation = workspaceGeneration.current;
    setBuildState({ kind: "loading" });
    setDeliveryBusy(true);
    try {
      const response = await invoke<BuildResponse>("build", {
        projectPath: projectRoot,
        dryRun: false,
        profile: selectedProfile || null,
      });
      if (workspaceGeneration.current !== generation) return;
      setBuildState({ kind: "done", value: response });
      showNotice("Build complete");
    } catch (error) {
      if (workspaceGeneration.current !== generation) return;
      setBuildState({ kind: "error", diagnostic: asApiError(error).diagnostic });
      setProblemsOpen(true);
    } finally {
      setDeliveryBusy(false);
    }
  }, [sourceMutationBlocked, buildState.kind, deliveryBusy, dirtyCount, projectRoot, selectedProfile, showNotice, workspace]);

  const selectFile = useCallback((file: WorkspaceSourceFile) => {
    setActivePath(file.path);
    if (file.kind === "data" && projectRoot) {
      void openDataFile(projectRoot, file.path);
    }
  }, [openDataFile, projectRoot]);

  const reloadConflict = useCallback(async (path: string) => {
    if (!projectRoot) return;
    if (!window.confirm(`Discard local changes in ${sourceName(path)} and reload the external version?`)) return;
    await openDataFile(projectRoot, path, true);
  }, [openDataFile, projectRoot]);

  const overwriteConflict = useCallback(async (path: string) => {
    const editor = editorsRef.current[path];
    if (!editor?.conflict) return;
    await saveFile(path, editor.conflict.contentIdentity);
  }, [saveFile]);

  const recheckSource = useCallback(async (path: string) => {
    const state = workspaceStateRef.current;
    const root = state.kind === "ready" ? state.workspace.project.project_root : state.previous?.project.project_root;
    const editor = editorsRef.current[path];
    if (!root || !editor) return;
    const generation = workspaceGeneration.current;
    try {
      const current = await invoke<SourceContentState>("source_content", {
        projectPath: root,
        relativePath: path,
      });
      if (workspaceGeneration.current !== generation) return;
      const latest = editorsRef.current[path];
      if (!latest) return;
      if (latest.previewState === "current"
        && current.contentIdentity === latest.preview.candidateContentIdentity) {
        await openDataFile(root, path, true);
        showNotice("Saved source content confirmed.");
        return;
      }
      if (current.contentIdentity === latest.snapshot.baseContentIdentity) {
        setEditors((all) => all[path]
          ? {
              ...all,
              [path]: {
                ...all[path],
                loadError: null,
                conflict: null,
                saveStatus: null,
                saveDiagnostic: null,
              },
            }
          : all);
        showNotice("Source state confirmed. Local changes are still unsaved.");
        return;
      }
      setEditors((all) => all[path]
        ? {
            ...all,
            [path]: {
              ...all[path],
              loadError: null,
              conflict: current,
              saveStatus: "conflict",
              view: "compare",
            },
          }
        : all);
    } catch (error) {
      if (workspaceGeneration.current !== generation) return;
      const diagnostic = apiDiagnosticToDiagnostic(asApiError(error).diagnostic);
      setEditors((all) => all[path]
        ? { ...all, [path]: { ...all[path], saveDiagnostic: diagnostic } }
        : all);
    }
  }, [openDataFile, showNotice]);

  const switchView = useCallback((path: string, view: EditorState["view"]) => {
    setEditors((current) => current[path]
      ? { ...current, [path]: { ...current[path], view } }
      : current);
  }, []);

  const focusDiagnostic = useCallback(async (diagnostic: Diagnostic) => {
    if (!workspace) return;
    const source = diagnostic.source ? normalizePath(diagnostic.source) : null;
    const file = source
      ? workspace.files.find((candidate) => source === normalizePath(`${workspace.project.project_root}/${candidate.path}`))
      : null;
    if (!file) return;
    const record = diagnosticRecordIndex(diagnostic);
    const field = diagnosticField(diagnostic);
    if (file.kind === "data" && record !== null && field) {
      const editor = editorsRef.current[file.path];
      const target = editor && diagnosticCellKey(editor, record, field);
      if (target) {
        pendingCellFocus.current = target;
        pendingValueFocus.current = diagnostic.value_path == null ? null : `${target}${diagnostic.value_path}`;
      }
    }
    selectFile(file);
    if (pendingCellFocus.current) {
      window.requestAnimationFrame(() => {
        const key = pendingCellFocus.current;
        if (!key) return;
        if (focusValuePathOrCell(key, pendingValueFocus.current)) {
          pendingCellFocus.current = null;
          pendingValueFocus.current = null;
        }
      });
    }
  }, [selectFile, workspace]);

  const refreshAfterCreation = useCallback(async (report: CreationReport) => {
    if (!projectRoot) return;
    const generation = workspaceGeneration.current;
    const next = await invoke<AuthoringWorkspace>("authoring_workspace", { projectPath: projectRoot });
    if (workspaceGeneration.current !== generation) return;
    // Creation refresh is navigation, not Project Reload: retain every dirty buffer.
    // EVIDENCE: GUI-CREATE-INT-008, GUI-CREATE-INT-009.
    setWorkspaceState({ kind: "ready", workspace: next });
    setActivePath(report.path);
    const root = report.folder ? next.folders?.find(folder => folder.path === report.path)?.sourceRoot
      : next.files.find(file => file.path === report.path)?.sourceRoot;
    setRevealCreated({ path: report.path, root: root ?? "" });
    setCreationOpen(false);
    const file = next.files.find(file => file.path === report.path);
    if (file?.kind === "data") await openDataFile(projectRoot, file.path);
    showNotice(`${sourceName(report.path)} created`);
  }, [projectRoot, openDataFile, showNotice]);

  const refreshMigrationFiles = useCallback(async (root: string, paths: string[]) => {
    const state = workspaceStateRef.current;
    if (state.kind !== "ready" || state.workspace.project.project_root !== root) return;
    const generation = workspaceGeneration.current;
    // Evict affected clean snapshots before awaiting reload, so they cannot be
    // edited against an obsolete schema (GUI-TABLE-STATE-004).
    const retained = { ...editorsRef.current };
    const reload = paths.filter(path => retained[path] && !editorIsDirty(retained[path]));
    for (const path of reload) delete retained[path];
    editorsRef.current = retained; setEditors(retained);
    const next = await invoke<AuthoringWorkspace>("authoring_workspace", { projectPath: root });
    if (workspaceGeneration.current !== generation) return;
    setWorkspaceState({ kind: "ready", workspace: next });
    setTableEpoch(epoch => epoch + 1);
    await Promise.all(reload.filter(path => next.files.some(file => file.path === path && file.kind === "data")).map(path => openDataFile(root, path, true)));
  }, [openDataFile]);
  const migrationResult = useCallback(async (root: string, result: MigrationResult) => {
    if (result.state === "recovery_required") recordRecovery(root, result);
    if (result.state === "success") await refreshMigrationFiles(root, result.files);
  }, [recordRecovery, refreshMigrationFiles]);
  const recheckMigration = async () => {
    if (!projectRoot || !recovery) return;
    try {
      const result = await invoke<MigrationResult | null>("recheck_migration", { projectPath: projectRoot });
      if (!result) await refreshMigrationFiles(projectRoot, recovery.files);
      recordRecovery(projectRoot, result);
    } catch (error) { showNotice(asApiError(error).diagnostic.message); }
  };

  const activeDiagnostics = activeEditor?.previewState === "current"
    ? activeEditor.preview.validation.diagnostics
    : [];
  const manualDiagnostics = manualValidation.kind === "done" ? manualValidation.value.diagnostics : [];
  const operationDiagnostics = [
    ...(activeEditor?.saveDiagnostic ? [activeEditor.saveDiagnostic] : []),
    ...(buildState.kind === "error" ? [apiDiagnosticToDiagnostic(buildState.diagnostic)] : []),
    ...(manualValidation.kind === "error" ? [apiDiagnosticToDiagnostic(manualValidation.diagnostic)] : []),
  ];
  const problems = [
    ...activeDiagnostics.map((diagnostic) => ({ diagnostic, origin: "Buffer" })),
    ...manualDiagnostics.map((diagnostic) => ({ diagnostic, origin: "Saved source" })),
    ...operationDiagnostics.map((diagnostic) => ({ diagnostic, origin: "Operation" })),
  ];

  return (
    <main className="app-shell">
      <header className="titlebar">
        <div className="brand-block">
          <div className="brand-mark"><Database size={18} /></div>
          <div>
            <strong>masterdata</strong>
            <span>{workspace?.project.name ?? "No project"}</span>
          </div>
        </div>
        <div className="project-open">
          <Input
            aria-label="Project path"
            value={projectPathInput}
            onChange={(event) => setProjectPathInput(event.target.value)}
            placeholder="Project folder path"
          />
          <Button
            htmlType="button"
            onClick={() => projectPathInput.trim() && requestAction({ kind: "open", projectPath: projectPathInput.trim() })}
          >
            <FolderOpen size={15} /> Open Project
          </Button>
        </div>
        <div className="command-bar">
          <Button htmlType="button" onClick={() => setSurface("overview")} disabled={!workspace}>Overview</Button>
          <Button htmlType="button" onClick={() => setSurface("settings")} disabled={!workspace}>Settings</Button>
          <Button htmlType="button" onClick={() => setSurface("delivery")} disabled={!workspace}>Delivery</Button>
          <Button htmlType="button" onClick={() => requestAction({ kind: "create" })}>Create Project</Button>
          <Button htmlType="button" onClick={() => setSurface("editor")} disabled={surface === "editor"}>Editor</Button>
          <Button htmlType="button" icon={<RotateCw size={15} />} onClick={() => requestAction({ kind: "reload" })} disabled={!workspace}>Reload</Button>
          <Button
            htmlType="button"
            icon={<Save size={15} />}
            onClick={() => surface === "settings" ? void settingsSaveRef.current() : activePath && void saveFile(activePath)}
            disabled={surface === "settings"
              ? mutationBlocked || !settingsDirty
              : mutationBlocked || !activeEditor || !editorIsDirty(activeEditor) || activeEditor.saving || activeEditor.saveStatus === "outcome_unknown" || !workspace?.capabilities.workspaceWrite}
          >
            {surface === "settings" ? "Save Settings" : activeEditor?.saving ? "Saving…" : "Save"}
          </Button>
          <Button htmlType="button" icon={<ShieldCheck size={15} />} onClick={() => void validateDisk()} disabled={!workspace?.capabilities.validate || manualValidation.kind === "loading"}>
            {manualValidation.kind === "loading" ? "Validating…" : "Validate"}
          </Button>
          <Button type="primary" htmlType="button" icon={<Play size={15} />} onClick={() => void runBuild()} disabled={mutationBlocked || deliveryBusy || !workspace?.capabilities.build || buildState.kind === "loading"}>
            {buildState.kind === "loading" ? "Building…" : "Build"}
          </Button>
        </div>
      </header>

      {totalDirtyCount > 0 && (
        <div className="unsaved-build-note">
          {dirtyCount > 0 && `${dirtyCount} unsaved file${dirtyCount === 1 ? "" : "s"}. `}
          {settingsDirty && "masterdata.toml has unsaved settings. "}
          Build uses saved source and config only after explicit Save.
        </div>
      )}

      <section className="surface-layout" hidden={surface !== "overview"}>
        <ProjectOverviewPanel
          key={`${projectRoot ?? "none"}:${configRevision}`}
          active={surface === "overview"}
          projectRoot={projectRoot}
          workspace={workspace as SurfaceWorkspace | null}
          table={activeFile?.table ?? workspace?.files.find((file) => file.kind === "data")?.table ?? null}
          dirtySourceCount={dirtyCount}
          dirtyConfig={settingsDirty}
          profile={selectedProfile}
          onProfileChange={setSelectedProfile}
          onNavigate={(path, recordIndex, expectedIdentity) => {
            const file = workspace?.files.find((candidate) => candidate.path === path);
            if (file) {
              setSurface("editor");
              const editor = editorsRef.current[path];
              if (editor && editor.snapshot.baseContentIdentity !== expectedIdentity) {
                showNotice("Overview snapshot is stale; refresh Overview before opening this occurrence.");
                return;
              }
              pendingRecordFocus.current = { path, recordIndex, expectedIdentity };
              selectFile(file);
            }
          }}
        />
      </section>
      <section className="surface-layout" hidden={surface !== "settings"}>
        <ProjectSettingsPanel
          active={surface === "settings"}
          projectRoot={projectRoot}
          mutationBlocked={mutationBlocked || deliveryBusy}
          onDirtyChange={setSettingsDirty}
          onRegisterSave={registerSettingsSave}
          onSaved={refreshWorkspaceAfterConfigSave}
        />
      </section>
      <section className="surface-layout" hidden={surface !== "delivery"}>
        <DeliveryPanel
          active={surface === "delivery"}
          projectRoot={projectRoot}
          workspace={workspace as SurfaceWorkspace | null}
          dirtySourceCount={dirtyCount}
          dirtyConfig={settingsDirty}
          profile={selectedProfile}
          onProfileChange={setSelectedProfile}
          buildBlocked={mutationBlocked}
          publishBlocked={!!migrationBusyRoot}
          onBusyChange={setDeliveryBusy}
        />
      </section>
      <section className="surface-layout" hidden={surface !== "create"}>
        <ProjectCreatePanel
          active={surface === "create"}
          onCreated={(root) => {
            setSurface("editor");
            void loadWorkspace(root);
          }}
        />
      </section>

      {surface === "editor" && <section className="workspace-layout">
        <aside className="explorer" aria-label="Workspace Explorer">
          <div className="pane-heading">
            <span>EXPLORER</span>
            <Button size="small" aria-label="New source artifact" disabled={mutationBlocked || !workspace?.capabilities.workspaceWrite} onClick={() => setCreationOpen(true)}>New</Button>
          </div>
          {workspaceState.kind === "loading" && !workspace && <div className="pane-message">Opening project…</div>}
          {workspaceState.kind === "error" && !workspace && (
            <DiagnosticBanner diagnostic={apiDiagnosticToDiagnostic(workspaceState.diagnostic)} />
          )}
          {workspace && (
            <SourceTree
              workspace={workspace}
              activePath={activePath}
              editors={editors}
              loadingPaths={loadingPaths}
              fileOpenErrors={fileOpenErrors}
              onSelect={(file) => {
                selectFile(file);
                const relative = file.sourceRoot && file.path.startsWith(`${file.sourceRoot}/`) ? file.path.slice(file.sourceRoot.length + 1) : file.path;
                setCreationTarget({ root: file.sourceRoot, folder: relative.split("/").slice(0, -1).join("/") });
              }}
              onFolderSelect={(root, folder) => setCreationTarget({ root, folder })}
              revealCreated={revealCreated}
            />
          )}
        </aside>

        <section className="editor-area">
          {workspaceState.kind === "error" && workspace && (
            <div className="workspace-error-strip">
              <strong>{workspaceState.diagnostic.code}</strong>
              <span>{workspaceState.diagnostic.message}</span>
            </div>
          )}
          {!workspace && workspaceState.kind !== "loading" && (
            <EmptyEditor title="Open a Masterdata project" copy="Enter a project folder path above. Project semantics are resolved by the shared Rust application service." />
          )}
          {recovery && <Alert role="alert" type="error" title="Recovery Required — source changes and Build are blocked"
            description={<><p>{recovery.diagnostic?.message}</p><p>{recovery.files.join(", ")}</p>{recovery.fileStates?.map(file => <p key={file.path}>{file.path}: {file.state}</p>)}{recovery.recoveryWorkspace && <p>Recovery workspace: {recovery.recoveryWorkspace}</p>}</>}
            action={<Button onClick={() => void recheckMigration()}>Recheck recovered source</Button>} />}
          {workspace && !activeFile && (
            <EmptyEditor title="Select a source file" copy="Choose a YAML document from the Workspace Explorer." />
          )}
          {activeFile?.kind === "schema" && projectRoot && <TableEditor key={`${projectRoot}:${activeFile.path}:${tableEpoch}`}
            projectPath={projectRoot} path={activeFile.path} canWrite={!!workspace?.capabilities.workspaceWrite && !mutationBlocked}
            dirtyPaths={Object.entries(editors).filter(([,editor]) => editorIsDirty(editor) || editor.saving).map(([path]) => path)}
            beginApply={paths => {
              if (sourceMutationBlocked(projectRoot) || paths.some(path => editorsRef.current[path] && (editorIsDirty(editorsRef.current[path]) || editorsRef.current[path].saving))) return false;
              migrationBusyRef.current = projectRoot; setMigrationBusyRoot(projectRoot); return true;
            }}
            endApply={() => { if (migrationBusyRef.current === projectRoot) { migrationBusyRef.current = null; setMigrationBusyRoot(null); } }}
            onResult={result => migrationResult(projectRoot, result)} />}
          {activeFile?.kind === "type" && projectRoot && <TypeEditor key={`${projectRoot}:${activeFile.path}:${tableEpoch}`}
            projectPath={projectRoot} path={activeFile.path} canWrite={!!workspace?.capabilities.workspaceWrite && !mutationBlocked}
            dirtyPaths={Object.entries(editors).filter(([,editor]) => editorIsDirty(editor) || editor.saving).map(([path]) => path)}
            beginApply={paths => {
              if (sourceMutationBlocked(projectRoot) || paths.some(path => editorsRef.current[path] && (editorIsDirty(editorsRef.current[path]) || editorsRef.current[path].saving))) return false;
              migrationBusyRef.current = projectRoot; setMigrationBusyRoot(projectRoot); return true;
            }}
            endApply={() => { if (migrationBusyRef.current === projectRoot) { migrationBusyRef.current = null; setMigrationBusyRoot(null); } }}
            onResult={result => migrationResult(projectRoot, result)} />}
          {activeFile && activeFile.kind !== "data" && activeFile.kind !== "schema" && activeFile.kind !== "type" && (
            <SourcePlaceholder file={activeFile} />
          )}
          {activeFile?.kind === "data" && activeLoading && (
            <EmptyEditor title={`Loading ${sourceName(activeFile.path)}…`} copy="Refreshing records through the shared application service." />
          )}
          {activeFile?.kind === "data" && !activeLoading && activeLoadDiagnostic && (
            <section className="placeholder-editor">
              <h2>{sourceName(activeFile.path)} is unavailable</h2>
              <p>The previous clean snapshot is not editable until the source can be loaded safely.</p>
              <DiagnosticBanner diagnostic={apiDiagnosticToDiagnostic(activeLoadDiagnostic)} />
            </section>
          )}
          {activeFile?.kind === "data" && !activeLoading && !activeLoadDiagnostic && !activeEditor && (
            <EmptyEditor title={`Opening ${sourceName(activeFile.path)}…`} copy="Loading records through the shared application service." />
          )}
          {activeFile?.kind === "data" && !activeLoading && !activeLoadDiagnostic && activeEditor && (
            <DataEditor
              mutationBlocked={mutationBlocked}
              file={activeFile}
              projectRoot={projectRoot!}
              editor={activeEditor}
              onCellChange={(recordIndex, field, value) => updateCell(activeFile.path, recordIndex, field, value)}
              onDraftCellChange={(draftId, field, value) => updateDraftCell(activeFile.path, draftId, field, value)}
              onCellFocus={(key) => { historyEditKey.current = key; }}
              onTagsChange={(recordIndex, tags) => updateExistingTags(activeFile.path, recordIndex, tags)}
              onDraftTagsChange={(draftId, tags) => updateDraftTags(activeFile.path, draftId, tags)}
              onBatchApplied={(batch) => applyBatchPreview(activeFile.path, batch)}
              onQueryResult={(result) => updateQueryResult(activeFile.path, result)}
              onUndo={() => undoBuffer(activeFile.path)}
              onRedo={() => redoBuffer(activeFile.path)}
              onAddRow={() => addRow(activeFile.path)}
              onDeleteExistingRow={(recordIndex) => deleteExistingRow(activeFile.path, recordIndex)}
              onUndoExistingDelete={(recordIndex) => undoExistingDelete(activeFile.path, recordIndex)}
              onDeleteDraftRow={(draftId) => deleteDraftRow(activeFile.path, draftId)}
              onSave={() => void saveFile(activeFile.path)}
              onRecheckSource={() => void recheckSource(activeFile.path)}
              onSwitchView={(view) => switchView(activeFile.path, view)}
              onReloadConflict={() => void reloadConflict(activeFile.path)}
              onOverwriteConflict={() => void overwriteConflict(activeFile.path)}
              cellFocusStart={cellFocusStart}
            />
          )}

          <section className={`problems-panel ${problemsOpen ? "open" : "collapsed"}`}>
            <Button className="problems-header" htmlType="button" onClick={() => setProblemsOpen((value) => !value)}>
              <span>PROBLEMS <b>{problems.length}</b></span>
              <span className="problem-snapshot">
                {activeEditor?.previewState === "pending" && "Buffer validation pending"}
                {activeEditor?.previewState === "unavailable" && "Buffer validation unavailable"}
                {activeEditor?.previewState === "current" && (activeEditor.preview.validation.valid ? "Buffer valid" : "Buffer has diagnostics")}
                {manualValidation.kind === "done" && ` · Disk ${manualValidation.value.valid ? "valid" : "invalid"}`}
              </span>
            </Button>
            {problemsOpen && (
              <div className="problems-body">
                {activeEditor?.previewError && <DiagnosticBanner diagnostic={apiDiagnosticToDiagnostic(activeEditor.previewError)} />}
                {problems.length === 0 ? (
                  <div className="no-problems">No diagnostics for the current buffer.</div>
                ) : (
                  problems.map(({ diagnostic, origin }, index) => (
                    <Button className="problem-row" htmlType="button" key={`${origin}-${diagnostic.code}-${index}`} onClick={() => void focusDiagnostic(diagnostic)}>
                      <span className="problem-icon">!</span>
                      <strong>{diagnostic.code}</strong>
                      <span>{diagnostic.message}</span>
                      <small>{origin} · {formatDiagnosticLocation(diagnostic)}</small>
                    </Button>
                  ))
                )}
                {buildState.kind === "done" && (
                  <div className="build-result-line">Build complete · profile {buildState.value.profile ?? "unfiltered"} · {buildState.value.generatedFiles.length} generated C# files · saved sources only</div>
                )}
              </div>
            )}
          </section>
        </section>
      </section>}

      <footer className="statusbar">
        <span>{activePath ?? "No source selected"}</span>
        <span>{activeEditor ? `${activeEditor.snapshot.rows.length + activeEditor.addedRecords.length} records · ${activeEditor.snapshot.columns.length} fields` : ""}</span>
        <span>{totalDirtyCount > 0 ? `${totalDirtyCount} dirty` : "Saved"}</span>
      </footer>

      {creationOpen && workspace && projectRoot && <SourceCreation key={projectRoot}
        projectPath={projectRoot}
        initialRootIndex={Math.max(0, workspace.sourceRoots.indexOf(creationTarget.root))}
        initialFolder={creationTarget.folder}
        canWrite={workspace.capabilities.workspaceWrite && !mutationBlocked}
        onCancel={() => {
          setCreationOpen(false);
          window.requestAnimationFrame(() => {
            const origin = [creationTarget.root, creationTarget.folder].filter(Boolean).join("/");
            const target = document.querySelector<HTMLElement>(`[data-tree-path="${CSS.escape(origin)}"]`)
              ?? document.querySelector<HTMLElement>('[aria-label="New source artifact"]');
            target?.focus();
          });
        }}
        onCreated={refreshAfterCreation} />}

      <Modal open={pendingAction !== null} title="Save changes before continuing?"
        closable={!pendingActionBusy} keyboard={!pendingActionBusy} mask={{ closable: false }}
        onCancel={() => !pendingActionBusy && setPendingAction(null)}
        footer={[
          <Button key="cancel" disabled={pendingActionBusy || deliveryBusy} onClick={() => setPendingAction(null)}>Cancel</Button>,
          <Button key="discard" disabled={pendingActionBusy || deliveryBusy} onClick={() => pendingAction && void performAction(pendingAction)}>Don't Save</Button>,
          <Button key="save" type="primary" disabled={mutationBlocked || deliveryBusy} loading={pendingActionBusy} onClick={async () => {
            if (!pendingAction) return;
            setPendingActionBusy(true);
            const action = pendingAction;
            const ok = await saveAll();
            setPendingActionBusy(false);
            if (ok) await performAction(action);
          }}>Save All</Button>,
        ]}>
        <p>{deliveryBusy ? "A Build or Publish operation is still running. This action will remain blocked until it finishes. " : ""}{dirtyCount} source file{dirtyCount === 1 ? " has" : "s have"}{settingsDirty ? " and masterdata.toml has" : ""} unsaved changes. The requested action would discard the current buffers.</p>
      </Modal>

      {notice && <Alert className="toast" title={notice} type="info" showIcon role="status" />}
    </main>
  );
}

type SourceTreeFolder = {
  kind: "folder";
  sourceRoot: string;
  name: string;
  key: string;
  depth: number;
  children: SourceTreeNode[];
};

type SourceTreeFile = {
  kind: "file";
  name: string;
  key: string;
  depth: number;
  file: WorkspaceSourceFile;
};

type SourceTreeNode = SourceTreeFolder | SourceTreeFile;

function SourceTree({
  workspace,
  activePath,
  editors,
  loadingPaths,
  fileOpenErrors,
  onSelect,
  onFolderSelect,
  revealCreated,
}: {
  workspace: AuthoringWorkspace;
  activePath: string | null;
  editors: Record<string, EditorState>;
  loadingPaths: Set<string>;
  fileOpenErrors: Record<string, ApiDiagnostic>;
  onSelect: (file: WorkspaceSourceFile) => void;
  onFolderSelect: (root: string, folder: string) => void;
  revealCreated: { path: string; root: string } | null;
}) {
  const [collapsed, setCollapsed] = useState<Set<string>>(() => new Set());
  const groups = useMemo(() => workspace.sourceRoots.map((root) => {
    const children: SourceTreeNode[] = [];
    const emptyFolders = (workspace.folders ?? []).filter(folder => folder.sourceRoot === root);
    for (const folder of emptyFolders) {
      const relative = root && folder.path.startsWith(`${root}/`) ? folder.path.slice(root.length + 1) : folder.path;
      let level = children;
      let prefix = root;
      for (const [index, segment] of relative.split("/").filter(Boolean).entries()) {
        prefix = prefix ? `${prefix}/${segment}` : segment;
        let node = level.find((node): node is SourceTreeFolder => node.kind === "folder" && node.name === segment);
        if (!node) { node = { kind: "folder", sourceRoot: root, name: segment, key: prefix, depth: index + 1, children: [] }; level.push(node); }
        level = node.children;
      }
    }
    for (const file of workspace.files.filter((candidate) => candidate.sourceRoot === root)) {
      const relative = file.path.startsWith(`${root}/`) ? file.path.slice(root.length + 1) : file.path;
      const parts = relative.split("/").filter(Boolean);
      let level = children;
      let prefix = root;
      for (const [index, segment] of parts.slice(0, -1).entries()) {
        prefix = prefix ? `${prefix}/${segment}` : segment;
        let folder = level.find(
          (node): node is SourceTreeFolder => node.kind === "folder" && node.name === segment,
        );
        if (!folder) {
          folder = { kind: "folder", sourceRoot: root, name: segment, key: prefix, depth: index + 1, children: [] };
          level.push(folder);
        }
        level = folder.children;
      }
      level.push({
        kind: "file",
        name: parts.at(-1) ?? file.path,
        key: file.path,
        depth: Math.max(1, parts.length),
        file,
      });
    }
    const sortNodes = (nodes: SourceTreeNode[]) => {
      nodes.sort((left, right) => {
        if (left.kind !== right.kind) return left.kind === "folder" ? -1 : 1;
        return left.name.localeCompare(right.name);
      });
      for (const node of nodes) if (node.kind === "folder") sortNodes(node.children);
    };
    sortNodes(children);
    return { root, children };
  }), [workspace]);

  useEffect(() => {
    if (!revealCreated) return;
    setCollapsed(current => new Set([...current].filter(key => key !== `root:${revealCreated.root}` && key !== revealCreated.path && !revealCreated.path.startsWith(`${key}/`))));
    const frame = window.requestAnimationFrame(() => document.querySelector<HTMLElement>(`[data-tree-path="${CSS.escape(revealCreated.path)}"]`)?.focus());
    return () => window.cancelAnimationFrame(frame);
  }, [revealCreated, workspace]);

  const toggleFolder = (key: string) => {
    setCollapsed((current) => {
      const next = new Set(current);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const handleTreeKey = (
    event: React.KeyboardEvent<HTMLElement>,
    folderKey?: string,
    expanded = false,
  ) => {
    const tree = event.currentTarget.closest('[role="tree"]');
    if (!tree) return;
    const items = Array.from(tree.querySelectorAll<HTMLElement>('[role="treeitem"]'));
    const index = items.indexOf(event.currentTarget);
    const depth = Number(event.currentTarget.dataset.depth ?? "0");
    const focusAt = (next: number) => items[Math.max(0, Math.min(items.length - 1, next))]?.focus();
    if (event.key === "ArrowDown") {
      event.preventDefault();
      focusAt(index + 1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      focusAt(index - 1);
    } else if (event.key === "Home") {
      event.preventDefault();
      focusAt(0);
    } else if (event.key === "End") {
      event.preventDefault();
      focusAt(items.length - 1);
    } else if (event.key === "ArrowRight" && folderKey) {
      event.preventDefault();
      if (!expanded) toggleFolder(folderKey);
      else if (Number(items[index + 1]?.dataset.depth ?? depth) > depth) focusAt(index + 1);
    } else if (event.key === "ArrowLeft") {
      if (folderKey && expanded) {
        event.preventDefault();
        toggleFolder(folderKey);
      } else {
        for (let cursor = index - 1; cursor >= 0; cursor -= 1) {
          if (Number(items[cursor].dataset.depth ?? "0") < depth) {
            event.preventDefault();
            items[cursor].focus();
            break;
          }
        }
      }
    } else if ((event.key === "Enter" || event.key === " ") && folderKey) {
      event.preventDefault();
      toggleFolder(folderKey);
    }
  };

  const renderNode = (node: SourceTreeNode): React.ReactNode => {
    if (node.kind === "folder") {
      const expanded = !collapsed.has(node.key);
      return (
        <div key={node.key}>
          <Button
            htmlType="button"
            role="treeitem"
            data-depth={node.depth}
            data-tree-path={node.key}
            aria-selected={activePath === node.key}
            aria-expanded={expanded}
            className="tree-folder"
            style={{ paddingLeft: `${10 + node.depth * 14}px` }}
            onFocus={() => {
              const root = node.sourceRoot;
              onFolderSelect(root, root ? node.key.slice(root.length + 1) : node.key);
            }}
            onClick={() => toggleFolder(node.key)}
            onKeyDown={(event) => handleTreeKey(event, node.key, expanded)}
          >
            <span className="folder-chevron" aria-hidden="true">{expanded ? "▾" : "▸"}</span>
            <span>{node.name}</span>
          </Button>
          {expanded && <div role="group">{node.children.map(renderNode)}</div>}
        </div>
      );
    }

    const editor = editors[node.file.path];
    const dirty = editorIsDirtySafe(editor);
    const loading = loadingPaths.has(node.file.path);
    const unavailable = Boolean(editor?.loadError || fileOpenErrors[node.file.path]);
    const stateLabels = [
      loading ? "loading" : null,
      editor?.saving ? "saving" : null,
      dirty ? "unsaved changes" : null,
      unavailable ? "source unavailable" : null,
      editor?.saveStatus === "failure" ? "save failed" : null,
      editor?.saveStatus === "outcome_unknown" ? "save outcome unknown" : null,
      editor?.conflict ? "external change conflict" : null,
    ].filter(Boolean);
    return (
      <Button
        key={node.key}
        htmlType="button"
        role="treeitem"
        data-depth={node.depth}
        data-tree-path={node.file.path}
        aria-selected={activePath === node.file.path}
        aria-label={`${node.file.path}${stateLabels.length ? `, ${stateLabels.join(", ")}` : ""}`}
        className={`tree-file ${activePath === node.file.path ? "active" : ""}`}
        style={{ paddingLeft: `${10 + node.depth * 14}px` }}
        onClick={() => onSelect(node.file)}
        onKeyDown={handleTreeKey}
        title={node.file.path}
      >
        <span className={`file-kind ${node.file.kind}`}>{node.file.kind === "data" ? "▦" : node.file.kind === "schema" ? "T" : node.file.kind === "type" ? "◇" : "!"}</span>
        <span className="tree-file-name">{node.name}</span>
        {dirty && <span className="dirty-dot" title="Unsaved changes">●</span>}
        {loading && <span className="file-state-badge" title="Loading">…</span>}
        {editor?.saving && <span className="file-state-badge" title="Saving">↻</span>}
        {unavailable && <span className="file-state-badge error" title="Source unavailable">×</span>}
        {editor?.saveStatus === "failure" && <span className="file-state-badge error" title="Save failed">×</span>}
        {editor?.saveStatus === "outcome_unknown" && <span className="file-state-badge warning" title="Save outcome unknown">?</span>}
        {editor?.conflict && <span className="conflict-badge" title="External change conflict">!</span>}
      </Button>
    );
  };

  return <div className="source-tree" role="tree" aria-label="Project source files">
    {groups.map(({ root, children }) => {
      const key = `root:${root}`;
      const expanded = !collapsed.has(key);
      return (
        <div className="source-root" key={root}>
          <Button
            htmlType="button"
            role="treeitem"
            data-depth="0"
            data-tree-path={root}
            aria-expanded={expanded}
            className="tree-root-label"
            onFocus={() => onFolderSelect(root, "")}
            onClick={() => toggleFolder(key)}
            onKeyDown={(event) => handleTreeKey(event, key, expanded)}
          >
            <span className="folder-chevron" aria-hidden="true">{expanded ? "▾" : "▸"}</span>
            {root || "."}
          </Button>
          {expanded && <div role="group">{children.map(renderNode)}</div>}
        </div>
      );
    })}
    {workspace.files.length === 0 && <div className="pane-message">No YAML source documents.</div>}
  </div>;
}

function editorIsDirtySafe(editor: EditorState | undefined): boolean {
  return editor ? editorIsDirty(editor) : false;
}

type GridRow =
  | { kind: "existing"; recordIndex: number; pendingDelete: boolean }
  | { kind: "added"; draft: AddedRecordDraft };

type GridRange = {
  startRow: number;
  startColumn: number;
  endRow: number;
  endColumn: number;
};

function DataEditor({
  mutationBlocked,
  file,
  projectRoot,
  editor,
  onCellChange,
  onDraftCellChange,
  onCellFocus,
  onTagsChange,
  onDraftTagsChange,
  onBatchApplied,
  onQueryResult,
  onUndo,
  onRedo,
  onAddRow,
  onDeleteExistingRow,
  onUndoExistingDelete,
  onDeleteDraftRow,
  onSave,
  onRecheckSource,
  onSwitchView,
  onReloadConflict,
  onOverwriteConflict,
  cellFocusStart,
}: {
  mutationBlocked: boolean;
  file: WorkspaceSourceFile;
  projectRoot: string;
  editor: EditorState;
  onCellChange: (recordIndex: number, field: string, value: AuthoringValue) => void;
  onDraftCellChange: (draftId: string, field: string, value: AuthoringValue) => void;
  onCellFocus: (key: string) => void;
  onTagsChange: (recordIndex: number, tags: string[]) => void;
  onDraftTagsChange: (draftId: string, tags: string[]) => void;
  onBatchApplied: (batch: AuthoringBatchPreview) => void;
  onQueryResult: (result: DataFileQueryResult | null) => void;
  onUndo: () => void;
  onRedo: () => void;
  onAddRow: () => void;
  onDeleteExistingRow: (recordIndex: number) => void;
  onUndoExistingDelete: (recordIndex: number) => void;
  onDeleteDraftRow: (draftId: string) => void;
  onSave: () => void;
  onRecheckSource: () => void;
  onSwitchView: (view: EditorState["view"]) => void;
  onReloadConflict: () => void;
  onOverwriteConflict: () => void;
  cellFocusStart: React.MutableRefObject<Map<string, AuthoringValue>>;
}) {
  const dirty = editorIsDirty(editor);
  const diagnostics = editor.previewState === "current" ? editor.preview.validation.diagnostics : [];
  const lastFocusedCell = useRef<string | null>(null);
  const [selectedRange, setSelectedRange] = useState<GridRange | null>(null);
  const draggingSelection = useRef(false);
  const [batchText, setBatchText] = useState("");
  const [batchPreview, setBatchPreview] = useState<AuthoringBatchPreview | null>(null);
  const [batchBusy, setBatchBusy] = useState(false);
  const [copyBusy, setCopyBusy] = useState(false);
  const [querySearch, setQuerySearch] = useState("");
  const [queryField, setQueryField] = useState("");
  const [queryValue, setQueryValue] = useState("");
  const [querySortField, setQuerySortField] = useState("");
  const [queryBusy, setQueryBusy] = useState(false);
  const [queryError, setQueryError] = useState<ApiDiagnostic | null>(null);
  const [queryNotice, setQueryNotice] = useState<string | null>(null);
  const [queryOperator, setQueryOperator] = useState("contains");
  const [querySortDirection, setQuerySortDirection] = useState("ascending");
  const queryRequestSequence = useRef(0);
  const previousAddedCount = useRef(editor.addedRecords.length);
  const [batchContext, setBatchContext] = useState<{ revision: number; selectionKey: string } | null>(null);
  const capability = addCapability(editor.snapshot);
  const rowsByRecordIndex = new Map(editor.snapshot.rows.map((row) => [row.recordIndex, row]));
  const draftsByQueryIndex = new Map(editor.addedRecords.map((draft, index) => [editor.snapshot.rows.length + index, draft]));
  const queryOrder = editor.queryResult?.orderedRecordIndices;
  const gridRows: GridRow[] = queryOrder
    ? queryOrder.flatMap<GridRow>((recordIndex) => {
        const row = rowsByRecordIndex.get(recordIndex);
        if (row) return [{ kind: "existing" as const, recordIndex: row.recordIndex, pendingDelete: editor.pendingDeletes.includes(row.recordIndex) }];
        const draft = draftsByQueryIndex.get(recordIndex);
        return draft ? [{ kind: "added" as const, draft }] : [];
      })
    : [
        ...editor.snapshot.rows.map((row) => ({
          kind: "existing" as const,
          recordIndex: row.recordIndex,
          pendingDelete: editor.pendingDeletes.includes(row.recordIndex),
        })),
        ...editor.addedRecords.map((draft) => ({ kind: "added" as const, draft })),
      ];
  const gridCellKeys = gridRows.map((row) => editor.snapshot.columns.map((column) =>
    row.kind === "existing"
      ? cellKey(row.recordIndex, column.name)
      : draftCellKey(row.draft.draftId, column.name)));

  useEffect(() => {
    lastFocusedCell.current = null;
  }, [file.path]);

  useEffect(() => {
    const stopDragging = () => { draggingSelection.current = false; };
    window.addEventListener("mouseup", stopDragging);
    return () => window.removeEventListener("mouseup", stopDragging);
  }, []);

  useEffect(() => {
    if (editor.view !== "grid" || !lastFocusedCell.current) return;
    const key = lastFocusedCell.current;
    window.requestAnimationFrame(() => {
      focusElement(document.querySelector<HTMLElement>(`[data-cell="${CSS.escape(key)}"]`));
    });
  }, [editor.view]);

  const selectedTargets = useCallback((): BatchTarget[] => {
    const range = selectedRange ?? { startRow: 0, startColumn: 0, endRow: 0, endColumn: 0 };
    const startRow = Math.min(range.startRow, range.endRow);
    const endRow = Math.max(range.startRow, range.endRow);
    const startColumn = Math.min(range.startColumn, range.endColumn);
    const endColumn = Math.max(range.startColumn, range.endColumn);
    const targets: BatchTarget[] = [];
    for (let rowIndex = startRow; rowIndex <= endRow; rowIndex += 1) {
      const row = gridRows[rowIndex];
      if (!row) continue;
      for (let columnIndex = startColumn; columnIndex <= endColumn; columnIndex += 1) {
        const column = editor.snapshot.columns[columnIndex];
        if (!column) continue;
        targets.push(row.kind === "existing"
          ? { recordIndex: row.recordIndex, field: column.name }
          : { addedRecordIndex: editor.addedRecords.findIndex((draft) => draft.draftId === row.draft.draftId), field: column.name });
      }
    }
    return targets;
  }, [editor.addedRecords, editor.snapshot.columns, gridRows, selectedRange]);
  const selectionContextKey = `${JSON.stringify(editor.queryResult ? {
    query: editor.queryResult.query,
    orderedRecordIndices: editor.queryResult.orderedRecordIndices,
  } : null)}:${JSON.stringify(selectedRange)}`;
  const batchIsStale = batchPreview !== null
    && batchContext !== null
    && (batchContext.revision !== editor.revision || batchContext.selectionKey !== selectionContextKey);

  const targetAt = (rowIndex: number, columnIndex: number): BatchTarget | null => {
    const row = gridRows[rowIndex];
    const column = editor.snapshot.columns[columnIndex];
    if (!row || !column) return null;
    return row.kind === "existing"
      ? { recordIndex: row.recordIndex, field: column.name }
      : { addedRecordIndex: editor.addedRecords.findIndex((draft) => draft.draftId === row.draft.draftId), field: column.name };
  };

  const pasteTargets = async (clipboardText: string): Promise<BatchTarget[]> => {
    const shape = await invoke<AuthoringClipboardShape>("authoring_clipboard_shape", { clipboardText });
    const anchor = selectedRange ?? { startRow: 0, startColumn: 0, endRow: 0, endColumn: 0 };
    const targets: BatchTarget[] = [];
    for (let rowOffset = 0; rowOffset < shape.rows; rowOffset += 1) {
      for (let columnOffset = 0; columnOffset < shape.columns; columnOffset += 1) {
        const target = targetAt(anchor.startRow + rowOffset, anchor.startColumn + columnOffset);
        if (!target) {
          throw {
            diagnostic: {
              code: "E-AUTHORING-BATCH-SHAPE",
              kind: "validation",
              message: "Clipboard rectangle extends beyond the visible authoring grid.",
              source: null, line: null, column: null, schemaPath: null, valuePath: null,
              recordIdentity: null, suggestion: null, relatedRequirements: ["GUI-GRID-001"],
            },
          };
        }
        targets.push(target);
      }
    }
    return targets;
  };

  const previewBatch = async (fill: boolean, clipboardText = batchText) => {
    const requestSelectionKey = selectionContextKey;
    setBatchBusy(true);
    setQueryError(null);
    try {
      const targets = fill ? selectedTargets() : await pasteTargets(clipboardText);
      if (targets.length === 0) return;
      const result = await invoke<AuthoringBatchPreview>("preview_data_file_batch", {
        projectPath: projectRoot,
        relativePath: file.path,
        baseSource: editor.snapshot.baseSource,
        currentMutation: mutationForEditor(editor),
        request: { targets, clipboardText, fill },
      });
      setBatchPreview(result);
      setBatchContext({ revision: editor.revision, selectionKey: requestSelectionKey });
    } catch (error) {
      setQueryError(asApiError(error).diagnostic);
    } finally {
      setBatchBusy(false);
    }
  };

  const copySelection = async () => {
    const targets = selectedTargets();
    if (targets.length === 0) return;
    setCopyBusy(true);
    setQueryError(null);
    try {
      const result = await invoke<AuthoringBatchCopyResult>("copy_data_file_batch", {
        projectPath: projectRoot,
        relativePath: file.path,
        baseSource: editor.snapshot.baseSource,
        request: { targets, currentMutation: mutationForEditor(editor) },
      });
      await navigator.clipboard.writeText(result.clipboardText);
    } catch (error) {
      setQueryError(asApiError(error).diagnostic);
    } finally {
      setCopyBusy(false);
    }
  };

  const readClipboardAndPreview = async () => {
    try {
      const clipboardText = await navigator.clipboard.readText();
      setBatchText(clipboardText);
      await previewBatch(false, clipboardText);
    } catch (error) {
      setQueryError(asApiError(error).diagnostic);
    }
  };

  const runQuery = useCallback(async () => {
    const requestId = ++queryRequestSequence.current;
    if (queryField && queryOperator !== "is-null" && queryOperator !== "is-invalid" && queryValue.length === 0) {
      setQueryBusy(false);
      setQueryError({
        code: "E-AUTHORING-QUERY-INPUT",
        kind: "validation",
        message: "The selected filter operator requires a value.",
        source: null,
        line: null,
        column: null,
        schemaPath: null,
        valuePath: null,
        recordIdentity: null,
        suggestion: null,
        relatedRequirements: ["AUTHORING-QUERY-002"],
      });
      return;
    }
    if (!querySearch && !queryField && !querySortField) {
      setQueryBusy(false);
      onQueryResult(null);
      setBatchPreview(null);
      setBatchContext(null);
      setSelectedRange((current) => current ? { ...current, endRow: current.startRow, endColumn: current.startColumn } : current);
      setQueryError(null);
      return;
    }
    setQueryBusy(true);
    setQueryError(null);
    setBatchPreview(null);
    setBatchContext(null);
    try {
      const result = await invoke<DataFileQueryResult>("query_data_file", {
        projectPath: projectRoot,
        request: {
          relativePath: file.path,
          baseSource: editor.snapshot.baseSource,
          mutation: mutationForEditor(editor),
          query: {
            search: querySearch,
            filters: queryField && (queryValue.length > 0 || queryOperator === "is-null" || queryOperator === "is-invalid")
              ? [{ field: queryField, operator: queryOperator, ...(queryOperator === "is-null" || queryOperator === "is-invalid" ? {} : { value: queryInputValue(editor.snapshot.columns.find((column) => column.name === queryField), queryValue) }) }]
              : [],
            sort: querySortField ? { field: querySortField, direction: querySortDirection } : null,
          },
        },
      });
      if (requestId === queryRequestSequence.current) {
        onQueryResult(result);
        setSelectedRange((current) => current ? { ...current, endRow: current.startRow, endColumn: current.startColumn } : current);
      }
    } catch (error) {
      if (requestId === queryRequestSequence.current) {
        onQueryResult(null);
        setQueryError(asApiError(error).diagnostic);
      }
    } finally {
      if (requestId === queryRequestSequence.current) setQueryBusy(false);
    }
  }, [editor, file.path, onQueryResult, projectRoot, queryField, queryOperator, querySearch, querySortDirection, querySortField, queryValue]);

  useEffect(() => {
    const previous = previousAddedCount.current;
    previousAddedCount.current = editor.addedRecords.length;
    if (editor.addedRecords.length > previous) {
      setQuerySearch("");
      setQueryField("");
      setQueryValue("");
      setQueryOperator("contains");
      setQuerySortField("");
      setQuerySortDirection("ascending");
      setQueryNotice("Query cleared after Add Row.");
      return;
    }
    if (editor.revision === 0 || (!querySearch && !queryField && !querySortField)) return;
    void runQuery();
    // The callback captures the current query controls; editor revision is the
    // only trigger so editing a cell refreshes an active query exactly once.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editor.revision]);

  return (
    <section className="data-editor">
      <header className="editor-tabs">
        <div className="document-tab">
          <span className="file-kind data">▦</span>
          <strong>{sourceName(file.path)}</strong>
          {dirty && <span className="tab-dirty">●</span>}
        </div>
        <Tabs className="view-tabs" size="small" activeKey={editor.view}
          onChange={(key) => onSwitchView(key as EditorState["view"])}
          items={[{ key: "grid", label: "Data" }, { key: "diff", label: "Diff" },
            ...(editor.conflict ? [{ key: "compare", label: "Conflict" }] : [])]} />
        <div className="editor-actions">
          <span className={`validation-state ${editor.previewState}`}>{validationLabel(editor)}</span>
          <Button
            htmlType="button"
            aria-label="Add Row"
            onClick={onAddRow}
            disabled={mutationBlocked || !capability.supported || editor.saving}
          >
            Add Row
          </Button>
          <Button htmlType="button" aria-label="Undo" onClick={onUndo} disabled={mutationBlocked || editor.saving || editor.historyPast.length === 0}>Undo</Button>
          <Button htmlType="button" aria-label="Redo" onClick={onRedo} disabled={mutationBlocked || editor.saving || editor.historyFuture.length === 0}>Redo</Button>
          <Button htmlType="button" onClick={onSave} disabled={mutationBlocked || !dirty || editor.saving || editor.saveStatus === "outcome_unknown"}>{editor.saving ? "Saving…" : "Save"}</Button>
        </div>
      </header>

      <div className="authoring-toolbar" aria-label="Data authoring tools">
        <Input aria-label="Data search" placeholder="Search current buffer" value={querySearch} onChange={(event) => setQuerySearch(event.target.value)} onPressEnter={() => void runQuery()} />
        <Select aria-label="Data filter field" allowClear placeholder="Filter field" value={queryField || undefined} onChange={(value) => setQueryField(value ?? "")} options={editor.snapshot.columns.map((column) => ({ value: column.name, label: column.name }))} />
        <Select aria-label="Data filter operator" value={queryOperator} onChange={setQueryOperator} options={[{ value: "contains", label: "contains" }, { value: "equals", label: "equals" }, { value: "not-equals", label: "not equals" }, { value: "less-than", label: "<" }, { value: "greater-than", label: ">" }, { value: "is-null", label: "is null" }, { value: "is-invalid", label: "is invalid" }]} />
        <Input aria-label="Data filter value" placeholder="Filter contains" value={queryValue} onChange={(event) => setQueryValue(event.target.value)} />
        <Select aria-label="Data sort field" allowClear placeholder="Sort by" value={querySortField || undefined} onChange={(value) => setQuerySortField(value ?? "")} options={editor.snapshot.columns.map((column) => ({ value: column.name, label: column.name }))} />
        <Select aria-label="Data sort direction" value={querySortDirection} onChange={setQuerySortDirection} options={[{ value: "ascending", label: "A→Z" }, { value: "descending", label: "Z→A" }]} />
        <Button htmlType="button" onClick={() => void runQuery()} loading={queryBusy}>Query</Button>
        {editor.queryResult && <span className="query-result">{editor.queryResult.displayedCount} / {editor.queryResult.totalCount} rows</span>}
        <Input.TextArea aria-label="Clipboard TSV" rows={1} placeholder="Paste TSV for the selected scalar range" value={batchText} onChange={(event) => setBatchText(event.target.value)} />
        <Button htmlType="button" onClick={() => void previewBatch(false)} disabled={batchBusy}>Paste preview</Button>
        <Button htmlType="button" onClick={() => void previewBatch(true)} disabled={batchBusy}>Fill preview</Button>
        <Button htmlType="button" onClick={() => void copySelection()} loading={copyBusy} disabled={batchBusy || copyBusy}>Copy selection</Button>
        <Button htmlType="button" onClick={() => void readClipboardAndPreview()}>Read clipboard & preview</Button>
      </div>
      <div className="selection-status" role="status" aria-live="polite">
        {selectedRange ? `${selectedTargets().length} cells selected` : "One cell active"}
        {batchIsStale && " · batch preview expired; review again"}
      </div>
      {queryNotice && <Alert type="info" showIcon closable onClose={() => setQueryNotice(null)} title={queryNotice} />}
      {queryError && <Alert type="error" showIcon title={queryError.code} description={queryError.message} />}

      {!capability.supported && (
        <div className="add-row-reason" role="status">
          <strong>Add Row unavailable</strong>
          <span>{capability.reason ?? "This Table is outside the initial Required Primitive scope."}</span>
        </div>
      )}

      {(editor.saveStatus === "failure" || editor.saveStatus === "outcome_unknown") && (
        <div className="conflict-strip save-recovery-strip">
          <div>
            <strong>{editor.saveStatus === "outcome_unknown" ? "Previous save outcome is unknown." : "Save failed."}</strong>
            <span>Local changes are preserved. Recheck the workspace source before continuing recovery.</span>
          </div>
          <div>
            <Button htmlType="button" onClick={onRecheckSource}>Recheck Source</Button>
            {editor.saveStatus === "failure" && <Button htmlType="button" disabled={mutationBlocked} onClick={onSave}>Retry Save</Button>}
          </div>
        </div>
      )}

      {editor.conflict && (
        <div className="conflict-strip">
          <div>
            <strong>File changed outside masterdata.</strong>
            <span>Your unsaved buffer is preserved. Normal Save will not overwrite the external version.</span>
          </div>
          <div>
            <Button htmlType="button" onClick={() => onSwitchView("compare")}>Compare</Button>
            <Button htmlType="button" onClick={onReloadConflict}>Reload</Button>
            <Button danger htmlType="button" disabled={mutationBlocked} onClick={onOverwriteConflict}>Overwrite</Button>
          </div>
        </div>
      )}

      {editor.view === "grid" && (
        <div className="grid-scroll">
          <table className="record-grid">
            <thead>
              <tr>
                <th className="row-number">#</th>
                {editor.snapshot.columns.map((column) => (
                  <th key={column.name}>
                    <div className="column-heading">
                      <strong>{column.name}</strong>
                      <span>{column.typeName}</span>
                      {column.keyField && <em>KEY</em>}
                      {!column.editable && !column.keyField && <em title={column.readOnlyReason ?? undefined}>READ ONLY</em>}
                    </div>
                  </th>
                ))}
                <th className="tag-column">$tags</th>
              </tr>
            </thead>
            <tbody>
              {gridRows.map((gridRow, gridRowIndex) => (
                <tr
                  key={gridRow.kind === "existing" ? `record-${gridRow.recordIndex}` : gridRow.draft.draftId}
                  className={gridRow.kind === "existing" && gridRow.pendingDelete ? "pending-delete" : ""}
                >
                  <th className="row-number">
                    {gridRow.kind === "existing" ? gridRow.recordIndex + 1 : "new"}
                    {gridRow.kind === "existing" && gridRow.pendingDelete ? (
                      <>
                        <span className="row-state pending">Pending delete</span>
                        <Button
                          size="small"
                          htmlType="button"
                          aria-label={`Undo Delete record ${gridRow.recordIndex + 1}`}
                          onClick={() => onUndoExistingDelete(gridRow.recordIndex)}
                          disabled={mutationBlocked || editor.saving}
                        >
                          Undo Delete
                        </Button>
                      </>
                    ) : gridRow.kind === "existing" ? (
                      <Button
                        size="small"
                        htmlType="button"
                        aria-label={`Delete record ${gridRow.recordIndex + 1}`}
                        onClick={() => onDeleteExistingRow(gridRow.recordIndex)}
                        disabled={mutationBlocked || editor.saving}
                      >
                        Delete
                      </Button>
                    ) : (
                      <>
                        <span className="row-state added">New draft</span>
                        <Button
                          size="small"
                          htmlType="button"
                          aria-label={`Delete new row ${gridRowIndex + 1}`}
                          onClick={() => onDeleteDraftRow(gridRow.draft.draftId)}
                          disabled={mutationBlocked || editor.saving}
                        >
                          Delete
                        </Button>
                      </>
                    )}
                  </th>
                  {editor.snapshot.columns.map((column, columnIndex) => {
                    const key = gridCellKeys[gridRowIndex][columnIndex];
                    const snapshotCell = gridRow.kind === "existing"
                      ? editor.snapshot.rows.find((row) => row.recordIndex === gridRow.recordIndex)
                        ?.cells.find((cell) => cell.field === column.name)
                      : undefined;
                    const value = gridRow.kind === "existing"
                      ? currentCellValue(editor, gridRow.recordIndex, column.name)
                      : gridRow.draft.values[column.name] ?? nullAuthoringValue();
                    const cellDiagnostics = diagnostics.filter((diagnostic) =>
                      diagnostic.source != null &&
                      normalizePath(diagnostic.source) === normalizePath(`${projectRoot}/${file.path}`) &&
                      diagnosticField(diagnostic) === column.name &&
                      diagnosticRecordIndex(diagnostic) !== null &&
                      diagnosticCellKey(editor, diagnosticRecordIndex(diagnostic)!, column.name) === key);
                    const hasDiagnostic = cellDiagnostics.length > 0;
                    const invalidPaths = new Set(cellDiagnostics.map((diagnostic) => diagnostic.value_path ?? ""));
                    const changed = gridRow.kind === "added" || key in editor.edits;
                    const isSelected = selectedRange !== null
                      && gridRowIndex >= Math.min(selectedRange.startRow, selectedRange.endRow)
                      && gridRowIndex <= Math.max(selectedRange.startRow, selectedRange.endRow)
                      && columnIndex >= Math.min(selectedRange.startColumn, selectedRange.endColumn)
                      && columnIndex <= Math.max(selectedRange.startColumn, selectedRange.endColumn);
                    const editable = !mutationBlocked && (gridRow.kind === "added"
                      ? !editor.saving && column.shape !== null
                      : column.editable && snapshotCell?.editable === true && !gridRow.pendingDelete && !editor.saving);
                    const readOnlyReason = gridRow.kind === "added"
                      ? undefined
                      : snapshotCell?.readOnlyReason ?? (column.keyField
                        ? "Key fields are read-only on saved records."
                        : column.readOnlyReason ?? undefined);
                    return (
                      <td key={column.name} className={`${changed ? "changed" : ""} ${hasDiagnostic ? "invalid" : ""} ${isSelected ? "selected" : ""}`}>
                        <div
                          className="cell-wrap"
                          data-cell={key}
                          onMouseDown={(event) => {
                            if (event.button !== 0) return;
                            draggingSelection.current = true;
                            setSelectedRange((current) => event.shiftKey && current
                              ? { ...current, endRow: gridRowIndex, endColumn: columnIndex }
                              : { startRow: gridRowIndex, startColumn: columnIndex, endRow: gridRowIndex, endColumn: columnIndex });
                          }}
                          onMouseEnter={() => {
                            if (draggingSelection.current) {
                              setSelectedRange((current) => current
                                ? { ...current, endRow: gridRowIndex, endColumn: columnIndex }
                                : { startRow: gridRowIndex, startColumn: columnIndex, endRow: gridRowIndex, endColumn: columnIndex });
                            }
                          }}
                          onFocusCapture={() => {
                            onCellFocus(key);
                            cellFocusStart.current.set(key, value);
                            lastFocusedCell.current = key;
                          }}
                          onKeyDown={(event) => {
                            const input = event.target instanceof HTMLInputElement ? event.target : null;
                            const textSelectionActive = input?.selectionStart != null
                              && input.selectionEnd != null
                              && input.selectionStart !== input.selectionEnd;
                            if ((event.metaKey || event.ctrlKey)
                              && !event.altKey
                              && !textSelectionActive
                              && (event.key.toLowerCase() === "v"
                                || (event.key.toLowerCase() === "c" && selectedTargets().length > 1))) {
                              event.preventDefault();
                              if (event.key.toLowerCase() === "c") void copySelection();
                              else void readClipboardAndPreview();
                              return;
                            }
                            handleGridKey(event, gridRowIndex, columnIndex, gridCellKeys, selectedRange, setSelectedRange, () => {
                              const initial = cellFocusStart.current.get(key);
                              if (initial === undefined) return;
                              if (gridRow.kind === "added") {
                                onDraftCellChange(gridRow.draft.draftId, column.name, initial);
                              } else {
                                onCellChange(gridRow.recordIndex, column.name, initial);
                              }
                            });
                          }}
                        >
                          {column.shape && (editable || column.keyField || snapshotCell?.editable === true) ? (
                            <ValueEditor
                              field={column.shape}
                              value={value}
                              label={`${gridRow.kind === "added" ? "new record" : `record ${gridRow.recordIndex + 1}`} ${column.name}`}
                              cellKey={key}
                              editable={editable}
                              invalidPaths={invalidPaths}
                              onChange={(next) => gridRow.kind === "added"
                                ? onDraftCellChange(gridRow.draft.draftId, column.name, next)
                                : onCellChange(gridRow.recordIndex, column.name, next)}
                            />
                          ) : (
                            <div className="read-only-value">
                              <output data-value-path={key}>{authoringValueSummary(value)}</output>
                              {readOnlyReason && <span>{readOnlyReason}</span>}
                            </div>
                          )}
                          {hasDiagnostic && <span className="cell-error" title="Validation diagnostic">!</span>}
                        </div>
                      </td>
                    );
                  })}
                  <td className="tag-cell">
                    {(() => {
                      const snapshotRow = gridRow.kind === "existing"
                        ? editor.snapshot.rows.find((row) => row.recordIndex === gridRow.recordIndex)
                        : undefined;
                      const tags = gridRow.kind === "existing"
                        ? editor.tagEdits[String(gridRow.recordIndex)] ?? snapshotRow?.tags ?? []
                        : gridRow.draft.tags;
                      const editable = !mutationBlocked && !editor.saving && (gridRow.kind === "added"
                        ? true
                        : snapshotRow?.tagsEditable !== false && !gridRow.pendingDelete);
                      return <RecordTagEditor
                        tags={tags}
                        suggestions={editor.snapshot.tagCandidates ?? []}
                        suggestionsComplete={editor.snapshot.tagCandidatesComplete !== false}
                        editable={editable}
                        reason={snapshotRow?.tagsReadOnlyReason ?? undefined}
                        label={`${gridRow.kind === "added" ? "new record" : `record ${gridRow.recordIndex + 1}`} tags`}
                        onChange={(next) => gridRow.kind === "added"
                          ? onDraftTagsChange(gridRow.draft.draftId, next)
                          : onTagsChange(gridRow.recordIndex, next)}
                      />;
                    })()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {gridRows.length === 0 && <div className="empty-grid">This data file has no records.</div>}
        </div>
      )}

      {editor.view === "diff" && (
        <DiffView
          title="Unsaved source diff"
          leftLabel="Saved base"
          rightLabel="Local buffer"
          left={editor.snapshot.baseSource}
          right={editor.preview.candidateSource}
          pending={editor.previewState !== "current"}
        />
      )}

      {editor.view === "compare" && editor.conflict && (
        <DiffView
          title="External change conflict"
          leftLabel="External source"
          rightLabel="Local save candidate"
          left={editor.conflict.source}
          right={editor.preview.candidateSource}
          pending={editor.previewState !== "current"}
        />
      )}
      <Modal
        open={batchPreview !== null}
        title="Scalar range preview"
        onCancel={() => { setBatchPreview(null); setBatchContext(null); }}
        onOk={() => {
          if (batchPreview && !batchIsStale) onBatchApplied(batchPreview);
          setBatchPreview(null);
          setBatchContext(null);
        }}
        okButtonProps={{ disabled: batchIsStale }}
        okText="Apply to Buffer"
        cancelText="Cancel"
      >
        {batchPreview && <>
          <p>{batchPreview.targetCount} target cells · {batchPreview.changedCellCount} changed. This only updates the local buffer.</p>
          {batchIsStale && <Alert type="warning" title="Preview expired" description="The buffer or selected range changed. Cancel and create a new preview." />}
          <pre className="batch-preview-source">{batchPreview.source.candidateSource}</pre>
          {!batchPreview.source.validation.valid && <Alert type="warning" title="Candidate is domain-invalid" description="The invalid value remains visible and can be corrected before Save." />}
        </>}
      </Modal>
    </section>
  );
}

function RecordTagEditor({
  tags,
  suggestions,
  suggestionsComplete,
  editable,
  reason,
  label,
  onChange,
}: {
  tags: string[];
  suggestions: string[];
  suggestionsComplete: boolean;
  editable: boolean;
  reason?: string;
  label: string;
  onChange: (tags: string[]) => void;
}) {
  const [input, setInput] = useState("");
  const commit = () => {
    if (!editable) return;
    onChange([...tags, input]);
    setInput("");
  };
  return (
    <div className="record-tags" aria-label={label}>
      <div className="record-tag-list">
        {tags.map((tag, index) => (
          <Tag key={`${index}:${tag}`} closable={editable} onClose={(event) => { event.preventDefault(); onChange(tags.filter((_, tagIndex) => tagIndex !== index)); }}>
            {tag.length === 0 ? "(empty)" : tag}
          </Tag>
        ))}
        {tags.length === 0 && <span className="no-tags">none</span>}
      </div>
      {editable ? <Input
        aria-label={`${label} entry`}
        list={`tag-suggestions-${label.replaceAll(" ", "-")}`}
        value={input}
        placeholder="Add tag; Enter to commit"
        onChange={(event) => setInput(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") { event.preventDefault(); commit(); }
          if (event.key === "Escape") { event.preventDefault(); setInput(""); }
          if (event.key === "Backspace" && input.length === 0 && tags.length > 0) onChange(tags.slice(0, -1));
        }}
      /> : <span className="tag-read-only-reason">{reason ?? "Tags are read-only."}</span>}
      {editable && suggestions.length > 0 && <div className="tag-suggestions" aria-label={`${label} suggestions`}>
        {suggestions.slice(0, 8).map((suggestion) => <Button key={suggestion} size="small" htmlType="button" onClick={() => onChange([...tags, suggestion])}>{suggestion}</Button>)}
        {!suggestionsComplete && <small>Candidate list incomplete</small>}
        <datalist id={`tag-suggestions-${label.replaceAll(" ", "-")}`}>{suggestions.map((suggestion) => <option key={suggestion} value={suggestion} />)}</datalist>
      </div>}
    </div>
  );
}

function validationLabel(editor: EditorState): string {
  if (editor.previewState === "pending") return "Validating buffer…";
  if (editor.previewState === "unavailable") return "Validation unavailable";
  return editor.preview.validation.valid ? "Buffer valid" : `${editor.preview.validation.diagnostics.length} problems`;
}

function handleGridKey(
  event: React.KeyboardEvent<HTMLDivElement>,
  rowIndex: number,
  columnIndex: number,
  cellKeys: string[][],
  selectedRange: GridRange | null,
  onRangeChange: (range: GridRange) => void,
  cancel: () => void,
) {
  const input = event.target instanceof HTMLInputElement ? event.target : null;
  if (event.key === "Escape") {
    event.preventDefault();
    if (selectedRange
      && (selectedRange.startRow !== selectedRange.endRow || selectedRange.startColumn !== selectedRange.endColumn)) {
      onRangeChange({ startRow: rowIndex, startColumn: columnIndex, endRow: rowIndex, endColumn: columnIndex });
      input?.blur();
      return;
    }
    cancel();
    input?.blur();
    return;
  }
  if (event.key === "F2") {
    if (input?.type === "text") {
      event.preventDefault();
      input.select();
    }
    return;
  }
  if (!input || input.type !== "text") return;
  let nextRow = rowIndex;
  let nextColumn = columnIndex;
  if (event.key === "ArrowUp") nextRow -= 1;
  else if (event.key === "ArrowDown" || event.key === "Enter") nextRow += 1;
  else if (event.key === "ArrowLeft" && input.selectionStart === 0) nextColumn -= 1;
  else if (event.key === "ArrowRight" && input.selectionStart === input.value.length) nextColumn += 1;
  else return;
  if (nextRow < 0 || nextRow >= cellKeys.length || nextColumn < 0 || nextColumn >= (cellKeys[nextRow]?.length ?? 0)) return;
  event.preventDefault();
  if (event.shiftKey && (event.key === "ArrowUp" || event.key === "ArrowDown" || event.key === "ArrowLeft" || event.key === "ArrowRight")) {
    const anchorRow = selectedRange?.startRow ?? rowIndex;
    const anchorColumn = selectedRange?.startColumn ?? columnIndex;
    onRangeChange({ startRow: anchorRow, startColumn: anchorColumn, endRow: nextRow, endColumn: nextColumn });
  } else {
    onRangeChange({ startRow: nextRow, startColumn: nextColumn, endRow: nextRow, endColumn: nextColumn });
  }
  const next = cellKeys[nextRow][nextColumn];
  focusValuePathOrCell(next, null);
}

function DiffView({
  title,
  leftLabel,
  rightLabel,
  left,
  right,
  pending,
}: {
  title: string;
  leftLabel: string;
  rightLabel: string;
  left: string;
  right: string;
  pending: boolean;
}) {
  const leftLines = left.split("\n");
  const rightLines = right.split("\n");
  const count = Math.max(leftLines.length, rightLines.length);
  return (
    <section className="diff-view">
      <header>
        <div><span className="dialog-kicker">SOURCE DIFF</span><h2>{title}</h2></div>
        {pending && <span className="diff-pending">Refreshing shared source preview…</span>}
      </header>
      {!pending && (
        <div className="diff-grid">
          <div className="diff-column"><strong>{leftLabel}</strong></div>
          <div className="diff-column"><strong>{rightLabel}</strong></div>
          {Array.from({ length: count }, (_, index) => {
            const a = leftLines[index] ?? "";
            const b = rightLines[index] ?? "";
            const changed = a !== b;
            return [
              <pre key={`a-${index}`} className={changed ? "changed" : ""}><span>{index + 1}</span>{a}</pre>,
              <pre key={`b-${index}`} className={changed ? "changed" : ""}><span>{index + 1}</span>{b}</pre>,
            ];
          })}
        </div>
      )}
    </section>
  );
}

function SourcePlaceholder({ file }: { file: WorkspaceSourceFile }) {
  return (
    <section className="placeholder-editor">
      <span className={`large-kind ${file.kind}`}>{file.kind === "schema" ? "T" : file.kind === "type" ? "◇" : "!"}</span>
      <h2>{sourceName(file.path)}</h2>
      <p><code>{file.kind}</code> typed editor is not part of the current existing-record authoring slice.</p>
      <dl>
        <div><dt>Path</dt><dd>{file.path}</dd></div>
        {file.table && <div><dt>Table</dt><dd>{file.table}</dd></div>}
        {file.typeName && <div><dt>Type</dt><dd>{file.typeName}</dd></div>}
      </dl>
      {file.diagnostic && <DiagnosticBanner diagnostic={file.diagnostic} />}
    </section>
  );
}

function EmptyEditor({ title, copy }: { title: string; copy: string }) {
  return <section className="empty-editor"><Empty description={<><h2>{title}</h2><p>{copy}</p></>} /></section>;
}

function DiagnosticBanner({ diagnostic }: { diagnostic: Diagnostic }) {
  return (
    <Alert className="diagnostic-banner" type="error" showIcon
      title={diagnostic.code}
      description={<>{diagnostic.message}{diagnostic.suggestion && <p>{diagnostic.suggestion}</p>}</>} />
  );
}

function apiDiagnosticToDiagnostic(diagnostic: ApiDiagnostic): Diagnostic {
  return {
    code: diagnostic.code,
    kind: diagnostic.kind,
    message: diagnostic.message,
    source: diagnostic.source,
    line: diagnostic.line,
    column: diagnostic.column,
    schema_path: diagnostic.schemaPath,
    value_path: diagnostic.valuePath,
    record_identity: diagnostic.recordIdentity,
    suggestion: diagnostic.suggestion,
    related_requirements: diagnostic.relatedRequirements,
  };
}

function formatDiagnosticLocation(diagnostic: Diagnostic): string {
  const parts = [];
  if (diagnostic.source) parts.push(normalizePath(diagnostic.source).split("/").slice(-3).join("/"));
  if (diagnostic.record_identity) parts.push(diagnostic.record_identity);
  if (diagnostic.line != null) parts.push(`L${diagnostic.line}${diagnostic.column != null ? `:${diagnostic.column}` : ""}`);
  return parts.join(" · ");
}

export default App;
