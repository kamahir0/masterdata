import { startGridReorder } from "./grid-reorder";
import TypeEditor from "./TypeEditor";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { Alert, Button, ConfigProvider, Dropdown, Empty, Input, Modal, Popover, Select, Tabs, Tag, theme as antdTheme } from "antd";
import { ArrowRight, ChevronDown, ChevronsUp, Database, FilePlus2, FolderOpen, FolderPlus, GripVertical, MoreHorizontal, RefreshCw, Search, Settings, X } from "lucide-react";
import TableEditor, { type MigrationResult } from "./TableEditor";
import SourceCreation, { type Category, type CreationReport } from "./SourceCreation";
import InlineSourceCreation from "./InlineSourceCreation";
import { ApplicationSettingsModal } from "./ApplicationSettings";
import {
  type EffectiveTheme,
  type ThemePreference,
  applyThemeToDom,
  getOsPrefersDark,
  resolveEffectiveTheme,
} from "./theme";
import {
  type InitialApplicationUserState,
  type RecentProject,
  bootstrapApplicationUserState,
  persistNativeRecentProjects,
  persistNativeThemePreference,
  sanitizeRecentProjects,
} from "./user-state";

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
  addDraft,
  applyPreviewResult,
  boundedHistoryPush,
  deleteDraft,
  deleteExisting,
  serializeAddedRecords,
  type EditorRowRef,
  undoExistingDelete as undoExistingDeleteState,
} from "./editor-state";
import { migrationRefreshPlan, resolveDirtyPathMutation } from "./authoring-workflow";
import {
  authoringValueSummary,
  authoringValuesEqual,
  nullAuthoringValue,
  type AuthoringValue,
  type ResolvedAuthoringField,
} from "./data-editor-types";
import { invoke, navigationMark, navigationCommit } from "./navigation-trace";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

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

type WorkspaceSourceFile = {
  path: string;
  sourceRoot: string;
  kind: string;
  table: string | null;
  typeName: string | null;
  hasInlineRecords: boolean;
  diagnostic: Diagnostic | null;
};

type TableField = { key: number; name: string; type: string; nullable: boolean; array: boolean };
type TableContext = {
  table: string;
  schemaPath: string;
  schemaContentIdentity: string;
  schemaSource: string;
  recordSources: { path: string; inline: boolean }[];
  selectedRecordSource: string | null;
  schema: { schema: { table: string; fields: TableField[]; primaryKey?: { fields: string[] }; secondaryKeys?: { fields: string[] }[] }; fieldTypes: string[] };
};
type SchemaDraftPreview = {
  candidateSource: string;
  candidateContentIdentity: string;
  changed: boolean;
  validation: ValidationReport;
  selectedSnapshot: DataFileSnapshot | null;
};
type SchemaDraftState = {
  root: string;
  path: string;
  table: string;
  baseSource: string;
  baseContentIdentity: string;
  baseFields: TableField[];
  fields: TableField[];
  historyPast: TableField[][];
  historyFuture: TableField[][];
  revision: number;
  preview: SchemaDraftPreview | null;
  previewState: "pending" | "current" | "error";
  previewError: Diagnostic | null;
  saving: boolean;
  saveStatus: "success" | "conflict" | "failure" | "outcome_unknown" | null;
  saveDiagnostic: Diagnostic | null;
};
type SchemaDraftSaveReport = {
  status: "success" | "conflict" | "failure" | "outcome_unknown";
  path: string;
  candidateContentIdentity: string;
  current: SourceContentState | null;
  diagnostic: Diagnostic | null;
};
type TableContextSaveReport = {
  files: {
    path: string;
    status: "success" | "unchanged" | "conflict" | "failure" | "outcome_unknown" | "not_attempted";
    candidateContentIdentity: string;
    current: SourceContentState | null;
    diagnostic: Diagnostic | null;
  }[];
};
function schemaDraftIsDirty(draft: SchemaDraftState): boolean {
  return draft.fields.length !== draft.baseFields.length || draft.fields.some((field, index) => {
    const base = draft.baseFields[index];
    return !base || field.name !== base.name || field.key !== base.key || field.type !== base.type || field.nullable !== base.nullable || field.array !== base.array;
  });
}
function schemaDraftFromContext(root: string, context: TableContext): SchemaDraftState {
  const fields = context.schema.schema.fields.map(field => ({ ...field }));
  return { root, path: context.schemaPath, table: context.table, baseSource: context.schemaSource,
    baseContentIdentity: context.schemaContentIdentity, baseFields: fields, fields,
    historyPast: [], historyFuture: [], revision: 0, preview: null, previewState: "pending",
    previewError: null, saving: false, saveStatus: null, saveDiagnostic: null };
}
type ColumnIntent =
  | { operation: "add_default"; table: string; beforeField?: string }
  | { operation: "reorder"; table: string; field: string; newIndex: number }
  | { operation: "rename"; table: string; field: string; newName: string }
  | { operation: "change_declaration"; table: string; field: string; type: string; nullable: boolean; array: boolean };

type AuthoringWorkspace = {
  project: ProjectInfo;
  sourceRoots: string[];
  files: WorkspaceSourceFile[];
  folders?: { path: string; sourceRoot: string }[];
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
  recordOrder?: { kind: "existing" | "added"; index: number }[];
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

type SourcePathEntryState = {
  path: string;
  exists: boolean;
  regularFile: boolean;
  symlink: boolean;
  contentIdentity: string | null;
};

type SourcePathMutationReport = {
  status: "success" | "conflict" | "failure" | "outcome_unknown";
  sourcePath: string;
  destinationPath: string;
  sourceState: SourcePathEntryState | null;
  destinationState: SourcePathEntryState | null;
  diagnostic: Diagnostic | null;
};

type SourcePathStateReport = {
  source: SourcePathEntryState;
  destination: SourcePathEntryState;
};

type PathMutationResult =
  | { kind: "report"; report: SourcePathMutationReport }
  | { kind: "error"; diagnostic: ApiDiagnostic }
  | { kind: "state"; state: SourcePathStateReport };

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
  | { kind: "idle"; previous: null }
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
  rowOrder: EditorRowRef[] | null;
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
  rowOrder: EditorRowRef[] | null;
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
    rowOrder: null,
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
    || Object.keys(editor.tagEdits).length > 0
    || !rowOrderIsDefault(editor);
}

function defaultRowOrder(editor: EditorState): EditorRowRef[] {
  return [
    ...editor.snapshot.rows.map(row => ({ kind: "existing" as const, recordIndex: row.recordIndex })),
    ...editor.addedRecords.map(draft => ({ kind: "added" as const, draftId: draft.draftId })),
  ];
}

function rowOrderIsDefault(editor: EditorState): boolean {
  if (!editor.rowOrder) return true;
  const baseline = defaultRowOrder(editor);
  return editor.rowOrder.length === baseline.length && editor.rowOrder.every((row, index) =>
    row.kind === baseline[index].kind && (row.kind === "existing"
      ? row.recordIndex === (baseline[index] as Extract<EditorRowRef, {kind: "existing"}>).recordIndex
      : row.draftId === (baseline[index] as Extract<EditorRowRef, {kind: "added"}>).draftId));
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
  const order = editor.rowOrder && !rowOrderIsDefault(editor) ? editor.rowOrder : null;
  const deleted = order ? new Set(editor.pendingDeletes) : null;
  const addedIndices = order ? new Map(editor.addedRecords.map((draft, index) => [draft.draftId, index])) : null;
  return {
    edits: Object.values(editor.edits),
    addedRecords: serializeAddedRecords(editor.snapshot.columns.map((column) => column.name), editor.addedRecords),
    deletedRecordIndices: [...editor.pendingDeletes].sort((left, right) => left - right),
    tagEdits: Object.entries(editor.tagEdits).map(([recordIndex, tags]) => ({
      recordIndex: Number(recordIndex),
      tags,
    })),
    ...(order ? { recordOrder: order.filter(row => row.kind === "added" || !deleted!.has(row.recordIndex)).map(row => row.kind === "existing"
      ? { kind: "existing" as const, index: row.recordIndex }
      : { kind: "added" as const, index: addedIndices!.get(row.draftId)! }) } : {}),
  };
}

function mutationHistoryState(editor: EditorState): MutationHistoryState {
  return {
    edits: editor.edits,
    addedRecords: editor.addedRecords,
    pendingDeletes: editor.pendingDeletes,
    tagEdits: editor.tagEdits,
    rowOrder: editor.rowOrder,
  };
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
  // Ant keeps a closed popover's controls in the DOM; focusing them would consume Problems navigation.
  if (element.closest('.ant-popover-hidden, [hidden], [inert], [aria-hidden="true"]')) return false;
  const focusable = element.matches("input, select, textarea, button, [tabindex='0']")
    ? element
    : element.querySelector<HTMLElement>("input:not([disabled]), select:not([disabled]), textarea:not([disabled]), button:not([disabled]), [tabindex='0']");
  if (!focusable) return false;
  focusable.focus();
  return document.activeElement === focusable;
}

function focusValuePathOrCell(cell: string, valuePath: string | null): boolean {
  if (valuePath !== null) {
    const nested = document.querySelector<HTMLElement>(`[data-value-path="${CSS.escape(valuePath)}"]`);
    if (focusElement(nested)) return true;
  }
  const target = document.querySelector<HTMLElement>(`[data-cell="${CSS.escape(cell)}"]`);
  if (!target) return false;
  // Grid cells outside the roving tab stop still need programmatic Problems focus.
  target.focus();
  if (document.activeElement !== target) return false;
  if (valuePath !== null) document.dispatchEvent(new CustomEvent("masterdata:focus-value", { detail: { cell, valuePath } }));
  return true;
}

function App({
  sourcePollingIntervalMs = 1600,
  previewDelayMs = 320,
  initialState,
}: {
  sourcePollingIntervalMs?: number | null;
  previewDelayMs?: number;
  initialState?: InitialApplicationUserState;
} = {}) {
  const [recoveries, setRecoveries] = useState<Record<string, MigrationResult>>({});
  const recoveryRef = useRef<Record<string, MigrationResult>>({});
  const [tableEpoch, setTableEpoch] = useState(0);
  const [schemaAction, setSchemaAction] = useState<{ path: string; operation: "add" | "rename"; field: string; serial: number } | null>(null);
  const schemaActionSerial = useRef(0);
  const [migrationBusyRoot, setMigrationBusyRoot] = useState<string | null>(null);
  const migrationBusyRef = useRef<string | null>(null);
  const sourceMutationBlocked = useCallback((root: string) => !!recoveryRef.current[root] || migrationBusyRef.current === root, []);
  const recordRecovery = useCallback((root: string, result: MigrationResult | null) => {
    const next = { ...recoveryRef.current };
    if (result?.state === "recovery_required") next[root] = result; else delete next[root];
    recoveryRef.current = next; setRecoveries(next);
  }, []);
  const [creationOpen, setCreationOpen] = useState(false);
  const [inlineCreation, setInlineCreation] = useState<{ root: string; folder: string; category: Category; table: string } | null>(null);
  const [creationTarget, setCreationTarget] = useState({ root: "", folder: "" });
  const [creationPreset, setCreationPreset] = useState<{ category: Category; table: string; filename?: string; name?: string }>({ category: "table", table: "" });
  const [revealCreated, setRevealCreated] = useState<{ path: string; root: string } | null>(null);
  const [pathMutationTarget, setPathMutationTarget] = useState<{ sourcePath: string; destinationPath: string; sourceRoot: string } | null>(null);
  const [pathMutationPhase, setPathMutationPhase] = useState<"form" | "dirty" | "result">("form");
  const [pathMutationBusy, setPathMutationBusy] = useState(false);
  const [pathMutationResult, setPathMutationResult] = useState<PathMutationResult | null>(null);
  const [surface, setSurface] = useState<Surface>("editor");
  const createReturnSurface = useRef<Surface>("editor");
  const [selectedProfile, setSelectedProfile] = useState("");
  const [settingsDirty, setSettingsDirty] = useState(false);
  const settingsSaveRef = useRef<() => Promise<boolean>>(async () => true);
  const settingsDirtyRef = useRef(false);
  const [deliveryBusy, setDeliveryBusy] = useState(false);
  const deliveryBusyRef = useRef(false);
  const [configRevision, setConfigRevision] = useState(0);

  const [workspaceState, setWorkspaceState] = useState<WorkspaceState>({ kind: "loading", previous: null });
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>(() => initialState?.recentProjects ?? []);
  const [projectPickerBusy, setProjectPickerBusy] = useState(false);
  const [activePath, setActivePath] = useState<string | null>(null);
  const [explorerPath, setExplorerPath] = useState<string | null>(null);
  const [tableContextState, setTableContextState] = useState<{ root: string; path: string; epoch: number; context: TableContext } | null>(null);
  const [schemaDrafts, setSchemaDrafts] = useState<Record<string, SchemaDraftState>>({});
  const schemaDraftsRef = useRef(schemaDrafts);
  const [tableContextError, setTableContextError] = useState<ApiDiagnostic | null>(null);
  const [tableSaveFailure, setTableSaveFailure] = useState<TableContextSaveReport | null>(null);
  const pendingColumnFocus = useRef<{ table: string; field: string | "last"; grip?: boolean } | null>(null);
  const [selectedTable, setSelectedTable] = useState<string | null>(null);
  const dataEditorUi = useRef(new Map<string, DataEditorUiState>());
  const [editors, setEditors] = useState<Record<string, EditorState>>({});
  const [manualValidation, setManualValidation] = useState<OperationState<ValidationReport>>({ kind: "idle" });
  const [buildState, setBuildState] = useState<OperationState<BuildResponse>>({ kind: "idle" });
  const [problemsOpen, setProblemsOpen] = useState(false);
  const [pendingAction, setPendingAction] = useState<PendingAction | null>(null);
  const [pendingActionBusy, setPendingActionBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [themePreference, setThemePreferenceState] = useState<ThemePreference>(() => initialState?.themePreference ?? "system");
  const [osDark, setOsDark] = useState<boolean>(getOsPrefersDark);
  const [appSettingsOpen, setAppSettingsOpen] = useState(false);

  useEffect(() => {
    if (!initialState) {
      bootstrapApplicationUserState().then((loaded) => {
        setThemePreferenceState(loaded.themePreference);
        setRecentProjects(loaded.recentProjects);
      }).catch(() => {});
    }
  }, [initialState]);

  const effectiveTheme = useMemo(
    () => resolveEffectiveTheme(themePreference, osDark),
    [themePreference, osDark]
  );

  useEffect(() => {
    if (typeof window === "undefined" || typeof window.matchMedia !== "function") return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent) => {
      setOsDark(e.matches);
    };
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);

  useEffect(() => {
    applyThemeToDom(effectiveTheme);
  }, [effectiveTheme]);

  const handleThemePreferenceChange = useCallback((pref: ThemePreference) => {
    persistNativeThemePreference(pref).catch(() => {});
    setThemePreferenceState(pref);
  }, []);

  const antThemeConfig = useMemo(() => {
    const isDark = effectiveTheme === "dark";
    return {
      algorithm: isDark ? antdTheme.darkAlgorithm : antdTheme.defaultAlgorithm,
      token: {
        colorPrimary: isDark ? "#8b7cf7" : "#6355d8",
        borderRadius: 8,
        colorBgContainer: isDark ? "#171e30" : "#ffffff",
        colorBgElevated: isDark ? "#1c2438" : "#f8fafc",
        colorText: isDark ? "#d7dde7" : "#1a202c",
        colorBorder: isDark ? "#273241" : "#d0d7de",
        fontSize: 13,
        fontFamily: 'Inter, system-ui, -apple-system, "Segoe UI", sans-serif',
      },
    };
  }, [effectiveTheme]);

  const [loadingPaths, setLoadingPaths] = useState<Set<string>>(() => new Set());
  const [fileOpenErrors, setFileOpenErrors] = useState<Record<string, ApiDiagnostic>>({});
  const previewTimers = useRef(new Map<string, number>());
  const editorsRef = useRef(editors);
  const workspaceStateRef = useRef(workspaceState);
  const workspaceGeneration = useRef(0);
  const draftSequence = useRef(0);
  const historyEditKey = useRef<string | null>(null);
  const pendingCellFocus = useRef<string | null>(null);
  const pendingValueFocus = useRef<string | null>(null);
  const pendingRecordFocus = useRef<{ path: string; recordIndex: number; expectedIdentity: string } | null>(null);

  useEffect(() => {
    editorsRef.current = editors;
  }, [editors]);

  useEffect(() => { schemaDraftsRef.current = schemaDrafts; }, [schemaDrafts]);

  useEffect(() => {
    workspaceStateRef.current = workspaceState;
  }, [workspaceState]);

  useEffect(() => {
    settingsDirtyRef.current = settingsDirty;
  }, [settingsDirty]);

  const rememberProject = useCallback((project: AuthoringWorkspace["project"]) => {
    setRecentProjects((current) => {
      const next = sanitizeRecentProjects([
        { root: project.project_root, name: project.name },
        ...current.filter((item) => item.root !== project.project_root),
      ]);
      persistNativeRecentProjects(next).catch(() => {});
      return next;
    });
  }, []);

  const removeRecentProject = useCallback((root: string) => {
    setRecentProjects((current) => {
      const next = current.filter((item) => item.root !== root);
      persistNativeRecentProjects(next).catch(() => {});
      return next;
    });
  }, []);

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
  const savedTableContext = tableContextState?.root === projectRoot && tableContextState.path === activePath && tableContextState.epoch === tableEpoch ? tableContextState.context : null;
  const activeSchemaDraft = savedTableContext ? schemaDrafts[savedTableContext.schemaPath] ?? null : null;
  const tableContext = savedTableContext && activeSchemaDraft
    ? { ...savedTableContext, schema: { ...savedTableContext.schema, schema: { ...savedTableContext.schema.schema, fields: activeSchemaDraft.fields } } }
    : savedTableContext;
  const activeEditor = activePath ? editors[activePath] ?? null : null;
  const activeSchemaPreview = activeSchemaDraft && schemaDraftIsDirty(activeSchemaDraft) && activeSchemaDraft.previewState === "current"
    ? activeSchemaDraft.preview : null;
  const displayEditor = activeEditor && activeSchemaDraft
    ? { ...activeEditor,
        snapshot: { ...activeEditor.snapshot, columns: (() => {
          const columns = activeSchemaPreview?.selectedSnapshot?.columns ?? activeEditor.snapshot.columns;
          if (activeSchemaDraft.fields.every((field, index) => field.name === activeSchemaDraft.baseFields[index]?.name)) return columns;
          const order = new Map(activeSchemaDraft.fields.map((field, index) => [field.name, index]));
          return [...columns].sort((left, right) => (order.get(left.name) ?? Number.MAX_SAFE_INTEGER) - (order.get(right.name) ?? Number.MAX_SAFE_INTEGER));
        })() },
        preview: activeSchemaPreview ? { ...activeEditor.preview, validation: activeSchemaPreview.validation } : activeEditor.preview }
    : activeEditor;
  useLayoutEffect(() => navigationCommit(activePath), [activePath, editors, tableContextState]);
  const activeLoading = activePath ? loadingPaths.has(activePath) : false;
  const activeLoadDiagnostic = activeEditor && editorIsDirty(activeEditor)
    ? null
    : activeEditor?.loadError ?? (activePath ? fileOpenErrors[activePath] ?? null : null);
  useEffect(() => {
    const pending = pendingColumnFocus.current;
    if (!pending || !tableContext || pending.table !== tableContext.table) return;
    const field = pending.field === "last"
      ? tableContext.schema.schema.fields.at(-1)?.name
      : pending.field;
    if (!field) return;
    const name = document.querySelector<HTMLInputElement>(`[data-column-field="${CSS.escape(field)}"]`);
    const target = pending.grip ? name?.closest(".unified-column-header")?.querySelector<HTMLButtonElement>(".column-grab") : name;
    if (!target) return;
    const frame = window.requestAnimationFrame(() => {
      target.focus({ preventScroll: pending.grip });
      pendingColumnFocus.current = null;
    });
    return () => window.cancelAnimationFrame(frame);
  }, [tableContext, activeEditor, activeSchemaDraft]);
  const dirtyCount = new Set([
    ...Object.entries(editors).filter(([, editor]) => editorIsDirty(editor)).map(([path]) => path),
    ...Object.entries(schemaDrafts).filter(([, draft]) => schemaDraftIsDirty(draft)).map(([path]) => path),
  ]).size;
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
      const bindingChanged = current.project.project_id !== next.project.project_id
        || current.sourceRoots.length !== next.sourceRoots.length
        || current.sourceRoots.some((sourceRoot, index) => sourceRoot !== next.sourceRoots[index]);
      const dirtySources = Object.values(editorsRef.current).some(editorIsDirty);
      setWorkspaceState({ kind: "ready", workspace: next });
      setConfigRevision((revision) => revision + 1);
      setSelectedProfile((selected) => selected && next.project.profiles?.some((item) => item.name === selected) ? selected : "");
      if (bindingChanged && dirtySources) {
        showNotice("Project binding changed after Settings Save. Dirty source buffers were kept, and Save All stopped for review.");
        return false;
      }
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

  const loadWorkspace = useCallback(async (requestedProject: string | null, initialDiscovery = false) => {
    const generation = workspaceGeneration.current + 1;
    workspaceGeneration.current = generation;
    setCreationOpen(false);
    setInlineCreation(null);
    setSchemaAction(null);
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
      rememberProject(next.project);
      setEditors({});
      setSchemaDrafts({});
      const first = next.files.find((file) => file.kind === "data" || file.hasInlineRecords) ?? next.files[0] ?? null;
      setActivePath(first?.path ?? null);
      setExplorerPath(first?.path ?? null);
      setSelectedTable(first?.table ?? null);
      if (first && (first.kind === "data" || first.hasInlineRecords)) {
        await openDataFile(next.project.project_root, first.path, true);
      }
    } catch (error) {
      if (workspaceGeneration.current !== generation) return;
      const diagnostic = asApiError(error).diagnostic;
      if (initialDiscovery && !previous && diagnostic.code === "E-PROJECT-NOT-FOUND") {
        setWorkspaceState({ kind: "idle", previous: null });
        return;
      }
      setWorkspaceState({
        kind: "error",
        diagnostic,
        previous,
      });
    }
  }, [recordRecovery, openDataFile, rememberProject]);

  useEffect(() => {
    if (!projectRoot || !activeFile || (activeFile.kind !== "schema" && activeFile.kind !== "data")) {
      setTableContextState(null);
      setTableContextError(null);
      return;
    }
    let disposed = false;
    setTableContextError(null);
    void invoke<TableContext>("open_table_context", { projectPath: projectRoot, relativePath: activeFile.path })
      .then((context) => {
        if (disposed) return;
        setTableContextState({ root: projectRoot, path: activeFile.path, epoch: tableEpoch, context });
        setSchemaDrafts(current => {
          const existing = current[context.schemaPath];
          if (existing?.root === projectRoot && (existing.baseContentIdentity === context.schemaContentIdentity || schemaDraftIsDirty(existing))) return current;
          return { ...current, [context.schemaPath]: schemaDraftFromContext(projectRoot, context) };
        });
        if (activeFile.kind === "schema" && !activeFile.hasInlineRecords && context.selectedRecordSource && context.selectedRecordSource !== activeFile.path) {
          setActivePath(context.selectedRecordSource);
          void openDataFile(projectRoot, context.selectedRecordSource);
        }
      }).catch(error => { if (!disposed) { setTableContextState(null); setTableContextError(asApiError(error).diagnostic); } });
    return () => { disposed = true; };
  }, [projectRoot, activeFile?.path, activeFile?.kind, activeFile?.hasInlineRecords, tableEpoch, openDataFile]);

  const recordDraftSignature = savedTableContext?.recordSources.map(source => {
    const editor = editors[source.path];
    return editor && editorIsDirty(editor)
      ? `${source.path}:${editor.revision}:${editor.previewState}:${editor.preview.candidateContentIdentity}`
      : "";
  }).join("|") ?? "";
  useEffect(() => {
    const context = savedTableContext;
    const draft = context ? schemaDrafts[context.schemaPath] : null;
    if (!projectRoot || !context || !draft || !schemaDraftIsDirty(draft)) return;
    const dirtyEditors = context.recordSources.map(source => ({ path: source.path, editor: editors[source.path] }))
      .filter(({ editor }) => editor && editorIsDirty(editor)) as { path: string; editor: EditorState }[];
    if (dirtyEditors.some(({ editor }) => editor.previewState !== "current")) return;
    const recordDrafts = dirtyEditors.map(({ path, editor }) => ({ path, candidateSource: editor.preview.candidateSource }));
    let cancelled = false;
    const timer = window.setTimeout(() => {
      void invoke<SchemaDraftPreview>("preview_schema_draft", {
        projectPath: projectRoot, schemaPath: context.schemaPath, baseSource: draft.baseSource,
        fields: draft.fields.map(({ name, type, nullable, array }) => ({ name, type, nullable, array })),
        recordDrafts, selectedRecordPath: activePath && context.recordSources.some(source => source.path === activePath) ? activePath : null,
      }).then(preview => {
        if (cancelled) return;
        setSchemaDrafts(current => {
          const latest = current[context.schemaPath];
          if (!latest || latest.revision !== draft.revision || latest.baseContentIdentity !== draft.baseContentIdentity) return current;
          return { ...current, [context.schemaPath]: { ...latest, preview, previewState: "current", previewError: null } };
        });
      }).catch(cause => {
        if (cancelled) return;
        const diagnostic = apiDiagnosticToDiagnostic(asApiError(cause).diagnostic);
        setSchemaDrafts(current => {
          const latest = current[context.schemaPath];
          if (!latest || latest.revision !== draft.revision) return current;
          return { ...current, [context.schemaPath]: { ...latest, previewState: "error", previewError: diagnostic } };
        });
      });
    }, 100);
    return () => { cancelled = true; window.clearTimeout(timer); };
  }, [projectRoot, savedTableContext?.schemaPath, savedTableContext?.schemaContentIdentity, activePath, activeSchemaDraft?.revision, recordDraftSignature]);


  useEffect(() => {
    void loadWorkspace(null, true);
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
            [path]: applyPreviewResult(latest, preview),
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
    }, previewDelayMs);
    previewTimers.current.set(path, timer);
  }, [previewDelayMs]);

  const updateCell = useCallback((path: string, recordIndex: number, field: string, value: AuthoringValue, operation = false) => {
    if (!projectRoot) return;
    setEditors((current) => {
      const editor = current[path];
      if (!editor || editor.saving || editor.pendingDeletes.includes(recordIndex)) return current;
      const nextEdits = { ...editor.edits };
      const key = cellKey(recordIndex, field);
      if (authoringValuesEqual(value, currentCellValue(editor, recordIndex, field))) return current;
      const captureHistory = operation || historyEditKey.current === key;
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

  const addRow = useCallback((path: string, at?: number) => {
    if (!projectRoot) return;
    setEditors((current) => {
      const editor = current[path];
      if (!editor || editor.saving || !addCapability(editor.snapshot).supported) return current;
      const draftId = `draft-${draftSequence.current + 1}`;
      draftSequence.current += 1;
      const next = addDraft(editor, draftId, editor.snapshot.columns.map((column) => column.name));
      if (at !== undefined) {
        const currentOrder = editor.rowOrder ?? defaultRowOrder(editor);
        next.rowOrder = [...currentOrder.slice(0, at), { kind: "added", draftId }, ...currentOrder.slice(at)];
      }
      const firstField = editor.snapshot.columns[0]?.name;
      if (firstField) {
        pendingCellFocus.current = draftCellKey(draftId, firstField);
        pendingValueFocus.current = null;
      }
      schedulePreview(projectRoot, path, next);
      return { ...current, [path]: next };
    });
  }, [projectRoot, schedulePreview]);

  const moveRow = useCallback((path: string, row: EditorRowRef, destination: number) => {
    if (!projectRoot) return;
    setEditors(current => {
      const editor = current[path];
      if (!editor || editor.saving || editor.queryResult || editor.pendingDeletes.length) return current;
      const order = editor.rowOrder ?? defaultRowOrder(editor);
      const from = order.findIndex(item => item.kind === row.kind && (item.kind === "existing"
        ? item.recordIndex === (row as Extract<EditorRowRef, {kind: "existing"}>).recordIndex
        : item.draftId === (row as Extract<EditorRowRef, {kind: "added"}>).draftId));
      if (from < 0 || destination < 0 || destination >= order.length || from === destination) return current;
      const nextOrder = [...order];
      const [moved] = nextOrder.splice(from, 1);
      nextOrder.splice(destination, 0, moved);
      const next: EditorState = { ...editor, rowOrder: nextOrder, revision: editor.revision + 1,
        previewState: "pending", previewError: null, saveDiagnostic: null, queryResult: null,
        ...mutationHistoryFields(editor) };
      schedulePreview(projectRoot, path, next);
      return { ...current, [path]: next };
    });
  }, [projectRoot, schedulePreview]);

  const updateDraftCell = useCallback((path: string, draftId: string, field: string, value: AuthoringValue, operation = false) => {
    if (!projectRoot) return;
    setEditors((current) => {
      const editor = current[path];
      if (!editor || editor.saving) return current;
      if (!editor.addedRecords.some((draft) => draft.draftId === draftId)) return current;
      const currentDraft = editor.addedRecords.find((draft) => draft.draftId === draftId);
      const currentValue = currentDraft?.values[field] ?? nullAuthoringValue();
      if (authoringValuesEqual(value, currentValue)) return current;
      const key = draftCellKey(draftId, field);
      const captureHistory = operation || historyEditKey.current === key;
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
      const next = deleteExisting(editor, recordIndex);
      schedulePreview(projectRoot, path, next);
      return { ...current, [path]: next };
    });
  }, [projectRoot, schedulePreview]);

  const undoExistingDelete = useCallback((path: string, recordIndex: number) => {
    if (!projectRoot) return;
    setEditors((current) => {
      const editor = current[path];
      if (!editor || editor.saving || !editor.pendingDeletes.includes(recordIndex)) return current;
      const next = undoExistingDeleteState(editor, recordIndex);
      schedulePreview(projectRoot, path, next);
      return { ...current, [path]: next };
    });
  }, [projectRoot, schedulePreview]);

  const deleteDraftRow = useCallback((path: string, draftId: string) => {
    if (!projectRoot) return;
    setEditors((current) => {
      const editor = current[path];
      if (!editor || editor.saving || !editor.addedRecords.some((draft) => draft.draftId === draftId)) return current;
      const next = deleteDraft(editor, draftId);
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

  const applyBatchPreview = useCallback((path: string, batch: AuthoringBatchPreview, expectedRevision: number) => {
    setEditors((current) => {
      const editor = current[path];
      if (!editor || editor.saving || editor.revision !== expectedRevision) return current;
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

  const changeSchemaDraft = useCallback((schemaPath: string, field: string, change: Pick<TableField, "type" | "nullable" | "array">) => {
    setSchemaDrafts(current => {
      const draft = current[schemaPath];
      if (!draft || draft.saving || draft.saveStatus === "outcome_unknown") return current;
      const fields = draft.fields.map(item => item.name === field ? { ...item, ...change } : item);
      if (fields.every((item, index) => item.type === draft.fields[index].type && item.nullable === draft.fields[index].nullable && item.array === draft.fields[index].array)) return current;
      return { ...current, [schemaPath]: { ...draft, fields,
        historyPast: [...draft.historyPast, draft.fields].slice(-100), historyFuture: [], revision: draft.revision + 1,
        preview: null, previewState: "pending", previewError: null, saveStatus: null, saveDiagnostic: null } };
    });
  }, []);

  const reorderSchemaDraft = useCallback((schemaPath: string, field: string, newIndex: number) => {
    setSchemaDrafts(current => {
      const draft = current[schemaPath];
      if (!draft || draft.saving || draft.saveStatus === "outcome_unknown") return current;
      const from = draft.fields.findIndex(item => item.name === field);
      if (from < 0 || newIndex < 0 || newIndex >= draft.fields.length || from === newIndex) return current;
      const fields = [...draft.fields];
      const [moved] = fields.splice(from, 1);
      fields.splice(newIndex, 0, moved);
      return { ...current, [schemaPath]: { ...draft, fields,
        historyPast: [...draft.historyPast, draft.fields].slice(-100), historyFuture: [], revision: draft.revision + 1,
        preview: null, previewState: "pending", previewError: null, saveStatus: null, saveDiagnostic: null } };
    });
  }, []);

  const undoSchemaDraft = useCallback((schemaPath: string, redo = false) => {
    setSchemaDrafts(current => {
      const draft = current[schemaPath];
      const history = redo ? draft?.historyFuture : draft?.historyPast;
      const previous = history?.at(-1);
      if (!draft || draft.saving || draft.saveStatus === "outcome_unknown" || !previous) return current;
      return { ...current, [schemaPath]: { ...draft, fields: previous,
        historyPast: redo ? [...draft.historyPast, draft.fields].slice(-100) : draft.historyPast.slice(0, -1),
        historyFuture: redo ? draft.historyFuture.slice(0, -1) : [...draft.historyFuture, draft.fields].slice(-100),
        revision: draft.revision + 1, preview: null, previewState: "pending", previewError: null,
        saveStatus: null, saveDiagnostic: null } };
    });
  }, []);

  const finishSchemaSave = useCallback(async (draft: SchemaDraftState, saved: SourceContentState, notify = true) => {
    const schemaPath = draft.path;
    const dirtyPaths = Object.entries(editorsRef.current)
      .filter(([, editor]) => editor.snapshot.table === draft.table && editorIsDirty(editor))
      .map(([path]) => path);
    const refreshed = await Promise.all(dirtyPaths.map(async path => {
      try {
        return { path, snapshot: await invoke<DataFileSnapshot>("open_data_file", { projectPath: draft.root, relativePath: path }), error: null };
      } catch (cause) {
        return { path, snapshot: null, error: apiDiagnosticToDiagnostic(asApiError(cause).diagnostic) };
      }
    }));
    const nextDraft: SchemaDraftState = { ...draft, baseSource: saved.source, baseContentIdentity: saved.contentIdentity,
      baseFields: draft.fields, historyPast: [], historyFuture: [], preview: null, previewState: "pending",
      saving: false, saveStatus: "success", saveDiagnostic: null };
    setSchemaDrafts(current => ({ ...current, [schemaPath]: nextDraft }));
    schemaDraftsRef.current = { ...schemaDraftsRef.current, [schemaPath]: nextDraft };
    const nextEditors = { ...editorsRef.current };
    for (const { path, snapshot, error } of refreshed) {
      const editor = nextEditors[path];
      if (!editor || !editorIsDirty(editor)) continue;
      if (error || !snapshot) {
        nextEditors[path] = { ...editor, saveDiagnostic: error };
        continue;
      }
      const expectedIdentity = path === schemaPath ? saved.contentIdentity : editor.snapshot.baseContentIdentity;
      if (snapshot.baseContentIdentity !== expectedIdentity) {
        nextEditors[path] = { ...editor, saveStatus: "conflict", conflict: { path, source: snapshot.baseSource, contentIdentity: snapshot.baseContentIdentity }, view: "compare" };
        continue;
      }
      const updated: EditorState = { ...editor, snapshot, queryResult: null,
        revision: editor.revision + 1, previewState: "pending", previewError: null };
      nextEditors[path] = updated;
      schedulePreview(draft.root, path, updated);
    }
    editorsRef.current = nextEditors;
    setEditors(nextEditors);
    setTableEpoch(epoch => epoch + 1);
    await Promise.all(Object.entries(nextEditors)
      .filter(([, editor]) => editor.snapshot.table === draft.table && !editorIsDirty(editor))
      .map(([path]) => openDataFile(draft.root, path, true)));
    if (notify) showNotice(`${sourceName(schemaPath)} saved`);
  }, [openDataFile, schedulePreview, showNotice]);

  const saveSchemaDraft = useCallback(async (schemaPath: string): Promise<boolean> => {
    const draft = schemaDraftsRef.current[schemaPath];
    if (!draft || !schemaDraftIsDirty(draft)) return true;
    if (sourceMutationBlocked(draft.root) || draft.saving || draft.saveStatus === "outcome_unknown") return false;
    setSchemaDrafts(current => current[schemaPath] ? { ...current, [schemaPath]: { ...current[schemaPath], saving: true, saveDiagnostic: null } } : current);
    try {
      const report = await invoke<SchemaDraftSaveReport>("save_schema_draft", {
        projectPath: draft.root, schemaPath, baseSource: draft.baseSource,
        baseContentIdentity: draft.baseContentIdentity,
        fields: draft.fields.map(({ name, type, nullable, array }) => ({ name, type, nullable, array })),
      });
      if (report.status !== "success") {
        setSchemaDrafts(current => current[schemaPath] ? { ...current, [schemaPath]: { ...current[schemaPath], saving: false, saveStatus: report.status, saveDiagnostic: report.diagnostic } } : current);
        return false;
      }
      const saved = await invoke<SourceContentState>("source_content", { projectPath: draft.root, relativePath: schemaPath });
      if (saved.contentIdentity !== report.candidateContentIdentity) {
        setSchemaDrafts(current => current[schemaPath] ? { ...current, [schemaPath]: { ...current[schemaPath], saving: false, saveStatus: "outcome_unknown",
          saveDiagnostic: { code: "E-FIELD-DECL-WRITE-VERIFY", kind: "validation", message: "The saved schema changed before the editor could verify it.", source: schemaPath } } } : current);
        return false;
      }
      await finishSchemaSave(draft, saved);
      return true;
    } catch (cause) {
      const diagnostic = apiDiagnosticToDiagnostic(asApiError(cause).diagnostic);
      const status = cause && typeof cause === "object" && "diagnostic" in cause ? "failure" : "outcome_unknown";
      setSchemaDrafts(current => current[schemaPath] ? { ...current, [schemaPath]: { ...current[schemaPath], saving: false, saveStatus: status, saveDiagnostic: diagnostic } } : current);
      return false;
    }
  }, [finishSchemaSave, sourceMutationBlocked]);

  const saveCurrentTableContext = useCallback(async (context: TableContext, selectedPath: string | null): Promise<boolean> => {
    const schema = schemaDraftsRef.current[context.schemaPath];
    const inline = context.recordSources.some(source => source.path === context.schemaPath && source.inline)
      ? editorsRef.current[context.schemaPath] : null;
    const record = selectedPath && selectedPath !== context.schemaPath ? editorsRef.current[selectedPath] : null;
    const schemaDirty = !!schema && schemaDraftIsDirty(schema);
    const inlineDirty = !!inline && editorIsDirty(inline);
    const recordDirty = !!record && editorIsDirty(record);
    if (!schemaDirty && !inlineDirty && !recordDirty) return true;
    const workspaceState = workspaceStateRef.current;
    const root = schema?.root ?? (workspaceState.kind === "ready" ? workspaceState.workspace.project.project_root : workspaceState.previous?.project.project_root);
    if (!root || sourceMutationBlocked(root) || schema?.saving || inline?.saving || record?.saving
      || schema?.saveStatus === "outcome_unknown" || inline?.saveStatus === "outcome_unknown" || record?.saveStatus === "outcome_unknown") return false;
    const generation = workspaceGeneration.current;
    setTableSaveFailure(null);
    if (schemaDirty) {
      schemaDraftsRef.current = { ...schemaDraftsRef.current, [context.schemaPath]: { ...schema!, saving: true, saveDiagnostic: null } };
      setSchemaDrafts(schemaDraftsRef.current);
    }
    const dirtyRecordPaths = [inlineDirty ? context.schemaPath : null, recordDirty ? selectedPath : null].filter((path): path is string => !!path);
    if (dirtyRecordPaths.length) {
      editorsRef.current = Object.fromEntries(Object.entries(editorsRef.current).map(([path, editor]) =>
        [path, dirtyRecordPaths.includes(path) ? { ...editor, saving: true, saveDiagnostic: null } : editor]));
      setEditors(editorsRef.current);
    }
    try {
      const report = await invoke<TableContextSaveReport>("save_current_table_context", {
        projectPath: root,
        request: {
          schemaPath: context.schemaPath,
          selectedRecordSource: selectedPath,
          schemaDraft: schemaDirty ? { baseSource: schema!.baseSource, baseContentIdentity: schema!.baseContentIdentity,
            fields: schema!.fields.map(({ name, type, nullable, array }) => ({ name, type, nullable, array })) } : null,
          inlineRecordDraft: inlineDirty ? { baseSource: inline!.snapshot.baseSource,
            baseContentIdentity: inline!.snapshot.baseContentIdentity, mutation: mutationForEditor(inline!) } : null,
          recordDraft: recordDirty ? { baseSource: record!.snapshot.baseSource,
            baseContentIdentity: record!.snapshot.baseContentIdentity, mutation: mutationForEditor(record!) } : null,
        },
      });
      if (workspaceGeneration.current !== generation) return false;
      const byPath = new Map(report.files.map(file => [file.path, file]));
      for (const path of dirtyRecordPaths) {
      const recordResult = byPath.get(path);
      if (recordResult) {
        if (recordResult.status === "success" || recordResult.status === "unchanged") {
          try {
            const snapshot = await invoke<DataFileSnapshot>("open_data_file", { projectPath: root, relativePath: path });
            if (workspaceGeneration.current !== generation) return false;
            if (snapshot.baseContentIdentity !== recordResult.candidateContentIdentity) {
              const conflict = { path, source: snapshot.baseSource, contentIdentity: snapshot.baseContentIdentity };
              recordResult.status = "conflict";
              recordResult.current = conflict;
              recordResult.diagnostic = { code: "E-TABLE-SAVE-POST-COMMIT-CONFLICT", kind: "validation",
                message: "Source changed again after the Save commit. Compare the current source before continuing.", source: path };
              const current = editorsRef.current;
              const next = { ...current, [path]: { ...current[path], saving: false, saveStatus: "conflict" as const,
                saveDiagnostic: recordResult.diagnostic, conflict, view: "compare" as const } };
              editorsRef.current = next;
              setEditors(next);
              continue;
            }
            const current = editorsRef.current;
            const next = { ...current, [path]: { ...editorFromSnapshot(snapshot), view: current[path]?.view ?? "grid" } };
            editorsRef.current = next;
            setEditors(next);
            if (path === context.schemaPath && !schemaDirty) setTableEpoch(epoch => epoch + 1);
          } catch (cause) {
            const diagnostic = apiDiagnosticToDiagnostic(asApiError(cause).diagnostic);
            const current = editorsRef.current;
            const committed = recordResult.current?.contentIdentity === recordResult.candidateContentIdentity
              ? recordResult.current : null;
            const next = { ...current, [path]: committed
              ? { ...editorFromSnapshot({ ...current[path].snapshot, baseSource: committed.source,
                  baseContentIdentity: committed.contentIdentity }), loadError: asApiError(cause).diagnostic }
              : { ...current[path], saving: false, saveStatus: "outcome_unknown" as const, saveDiagnostic: diagnostic } };
            editorsRef.current = next;
            setEditors(next);
            if (!committed) {
              recordResult.status = "outcome_unknown";
              recordResult.diagnostic = diagnostic;
            }
          }
        } else {
          const current = editorsRef.current;
          const next = { ...current, [path]: { ...current[path], saving: false,
            saveStatus: recordResult.status === "not_attempted" ? null : recordResult.status,
            saveDiagnostic: recordResult.diagnostic,
            conflict: recordResult.status === "conflict" ? recordResult.current : current[path].conflict,
            view: recordResult.status === "conflict" ? "compare" as const : current[path].view } };
          editorsRef.current = next;
          setEditors(next);
        }
      }
      }
      const schemaResult = schemaDirty ? byPath.get(context.schemaPath) : null;
      if (schemaResult && schema) {
        if (schemaResult.status === "success" || schemaResult.status === "unchanged") {
          const saved = schemaResult.current ?? await invoke<SourceContentState>("source_content", { projectPath: root, relativePath: context.schemaPath });
          if (saved.contentIdentity === schemaResult.candidateContentIdentity) await finishSchemaSave(schema, saved, false);
          else {
            schemaResult.status = "outcome_unknown";
            schemaResult.diagnostic = { code: "E-TABLE-SAVE-WRITE-VERIFY", kind: "validation", message: "Saved schema could not be verified.", source: context.schemaPath };
          }
        }
        if (schemaResult.status !== "success" && schemaResult.status !== "unchanged") {
          setSchemaDrafts(current => current[context.schemaPath] ? { ...current, [context.schemaPath]: {
            ...current[context.schemaPath], saving: false,
            saveStatus: schemaResult.status === "not_attempted" ? null : schemaResult.status === "unchanged" ? "success" : schemaResult.status,
            saveDiagnostic: schemaResult.diagnostic,
          } } : current);
        }
      }
      const complete = report.files.every(file => file.status === "success" || file.status === "unchanged");
      if (complete) showNotice("Current Table changes saved");
      else setTableSaveFailure({ files: [...report.files] });
      return complete;
    } catch (cause) {
      const diagnostic = apiDiagnosticToDiagnostic(asApiError(cause).diagnostic);
      if (schemaDirty) setSchemaDrafts(current => current[context.schemaPath] ? { ...current, [context.schemaPath]: {
        ...current[context.schemaPath], saving: false, saveStatus: "failure", saveDiagnostic: diagnostic } } : current);
      if (dirtyRecordPaths.length) setEditors(current => Object.fromEntries(Object.entries(current).map(([path, editor]) =>
        [path, dirtyRecordPaths.includes(path) ? { ...editor, saving: false, saveStatus: "failure", saveDiagnostic: diagnostic } : editor])));
      showNotice(diagnostic.message);
      return false;
    }
  }, [finishSchemaSave, showNotice, sourceMutationBlocked]);

  const recheckSchemaDraft = useCallback(async (schemaPath: string) => {
    const draft = schemaDraftsRef.current[schemaPath];
    if (!draft) return;
    try {
      const saved = await invoke<SourceContentState>("source_content", { projectPath: draft.root, relativePath: schemaPath });
      const preview = await invoke<SchemaDraftPreview>("preview_schema_draft", {
        projectPath: draft.root, schemaPath, baseSource: draft.baseSource,
        fields: draft.fields.map(({ name, type, nullable, array }) => ({ name, type, nullable, array })),
        recordDrafts: [], selectedRecordPath: null,
      });
      if (saved.contentIdentity === preview.candidateContentIdentity) {
        await finishSchemaSave(draft, saved);
      } else {
        setSchemaDrafts(current => current[schemaPath] ? { ...current, [schemaPath]: { ...current[schemaPath],
          saveStatus: saved.contentIdentity === draft.baseContentIdentity ? "failure" : "conflict",
          saveDiagnostic: saved.contentIdentity === draft.baseContentIdentity ? null : { code: "E-FIELD-DECL-CONFLICT", kind: "validation", message: "The schema source changed outside this draft.", source: schemaPath },
        } } : current);
      }
    } catch (cause) { showNotice(asApiError(cause).diagnostic.message); }
  }, [finishSchemaSave, showNotice]);

  const reloadSchemaDraft = useCallback(async (schemaPath: string) => {
    const draft = schemaDraftsRef.current[schemaPath];
    if (!draft) return;
    try {
      const context = await invoke<TableContext>("open_table_context", { projectPath: draft.root, relativePath: schemaPath });
      setSchemaDrafts(current => ({ ...current, [schemaPath]: schemaDraftFromContext(draft.root, context) }));
      setTableEpoch(epoch => epoch + 1);
    } catch (cause) { showNotice(asApiError(cause).diagnostic.message); }
  }, [showNotice]);

  const saveAll = useCallback(async (): Promise<boolean> => {
    if (settingsDirty && !(await settingsSaveRef.current())) return false;
    let allSaved = true;
    const handledInline = new Set<string>();
    for (const path of Object.keys(schemaDraftsRef.current).filter(path => schemaDraftIsDirty(schemaDraftsRef.current[path]))) {
      const draft = schemaDraftsRef.current[path];
      const inlineDirty = !!editorsRef.current[path] && editorIsDirty(editorsRef.current[path]);
      if (inlineDirty) {
        handledInline.add(path);
        try {
          const context = await invoke<TableContext>("open_table_context", { projectPath: draft.root, relativePath: path });
          if (!(await saveCurrentTableContext(context, path))) allSaved = false;
        } catch (cause) {
          showNotice(asApiError(cause).diagnostic.message);
          allSaved = false;
        }
      } else if (!(await saveSchemaDraft(path))) allSaved = false;
    }
    const paths = Object.entries(editorsRef.current)
      .filter(([path, editor]) => !handledInline.has(path) && editorIsDirty(editor))
      .map(([path]) => path);
    for (const path of paths) {
      if (!(await saveFile(path))) {
        allSaved = false;
      }
    }
    return allSaved;
  }, [saveCurrentTableContext, saveFile, saveSchemaDraft, settingsDirty, showNotice]);

  const performAction = useCallback(async (action: PendingAction) => {
    setPendingAction(null);
    if (action.kind === "close") {
      await getCurrentWindow().destroy();
      return;
    }
    if (action.kind === "create") {
      if (surface !== "create") createReturnSurface.current = surface;
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
  }, [loadWorkspace, surface]);

  const requestAction = useCallback((action: PendingAction) => {
    const hasDirty = settingsDirty || Object.values(editorsRef.current).some(editorIsDirty) || Object.values(schemaDraftsRef.current).some(schemaDraftIsDirty);
    if (deliveryBusy || hasDirty) {
      setPendingAction(action);
      if (deliveryBusy) showNotice("A Build or Publish operation is running. Project navigation will wait until it finishes.");
    } else {
      void performAction(action);
    }
  }, [deliveryBusy, performAction, settingsDirty, showNotice]);

  const openProjectPicker = useCallback(async () => {
    setProjectPickerBusy(true);
    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
        title: "Open Masterdata Project",
        ...(projectRoot ? { defaultPath: projectRoot } : {}),
      });
      if (typeof selected === "string") requestAction({ kind: "open", projectPath: selected });
    } catch (error) {
      const state = workspaceStateRef.current;
      const previous = state.kind === "ready" ? state.workspace : state.kind === "error" ? state.previous : null;
      setWorkspaceState({ kind: "error", diagnostic: asApiError(error).diagnostic, previous });
    } finally {
      setProjectPickerBusy(false);
    }
  }, [projectRoot, requestAction]);

  useEffect(() => {
    const listener = getCurrentWindow().onCloseRequested((event) => {
      if (deliveryBusyRef.current || settingsDirtyRef.current || Object.values(editorsRef.current).some(editorIsDirty) || Object.values(schemaDraftsRef.current).some(schemaDraftIsDirty)) {
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
        } else if (tableContext) void saveCurrentTableContext(tableContext, activePath && tableContext.recordSources.some(source => source.path === activePath) ? activePath : tableContext.selectedRecordSource);
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
  }, [activePath, deliveryBusy, redoBuffer, saveCurrentTableContext, showNotice, surface, tableContext, undoBuffer]);

  useEffect(() => {
    if (sourcePollingIntervalMs == null) return;
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
    }, sourcePollingIntervalMs);
    return () => window.clearInterval(timer);
  }, [openDataFile, sourcePollingIntervalMs]);

  const validateDisk = useCallback(async () => {
    if (!workspace || !projectRoot) return;
    const generation = workspaceGeneration.current;
    setManualValidation({ kind: "loading" });
    try {
      const report = await invoke<ValidationReport>("validate", { projectPath: projectRoot });
      if (workspaceGeneration.current !== generation) return;
      setManualValidation({ kind: "done", value: report });
    } catch (error) {
      if (workspaceGeneration.current !== generation) return;
      setManualValidation({ kind: "error", diagnostic: asApiError(error).diagnostic });
    }
  }, [projectRoot, workspace]);

  const runBuild = useCallback(async () => {
    if (!workspace || !projectRoot || sourceMutationBlocked(projectRoot) || buildState.kind === "loading" || deliveryBusy) return;
    if (dirtyCount > 0 || settingsDirty) {
      showNotice("Build uses saved source and config only; unsaved changes are not included.");
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
    } finally {
      setDeliveryBusy(false);
    }
  }, [sourceMutationBlocked, buildState.kind, deliveryBusy, dirtyCount, projectRoot, selectedProfile, settingsDirty, showNotice, workspace]);

  const selectFile = useCallback((file: WorkspaceSourceFile) => {
    navigationMark("selection", file.path);
    setActivePath(file.path);
    setExplorerPath(file.path);
    if (file.table) setSelectedTable(file.table);
    setSurface("editor");
    const relative = file.sourceRoot && file.path.startsWith(`${file.sourceRoot}/`) ? file.path.slice(file.sourceRoot.length + 1) : file.path;
    setCreationTarget({ root: file.sourceRoot, folder: relative.split("/").slice(0, -1).join("/") });
    if ((file.kind === "data" || file.hasInlineRecords) && projectRoot) {
      void openDataFile(projectRoot, file.path);
    }
  }, [openDataFile, projectRoot]);

  const openPathMutation = useCallback((requested?: WorkspaceSourceFile) => {
    const target = requested ?? activeFile;
    if (!target || !projectRoot || mutationBlocked) return;
    setPathMutationTarget({
      sourcePath: target.path,
      destinationPath: target.path,
      sourceRoot: target.sourceRoot,
    });
    setPathMutationResult(null);
    setPathMutationPhase("form");
  }, [activeFile, mutationBlocked, projectRoot, workspace]);

  const closePathMutation = useCallback(() => {
    if (pathMutationBusy) return;
    setPathMutationTarget(null);
    setPathMutationResult(null);
    setPathMutationPhase("form");
  }, [pathMutationBusy]);

  const removeEditorForPath = useCallback((path: string) => {
    const timer = previewTimers.current.get(path);
    if (timer !== undefined) {
      window.clearTimeout(timer);
      previewTimers.current.delete(path);
    }
    const retained = { ...editorsRef.current };
    delete retained[path];
    editorsRef.current = retained;
    setEditors(retained);
    setFileOpenErrors((current) => {
      if (!(path in current)) return current;
      const next = { ...current };
      delete next[path];
      return next;
    });
  }, []);

  const executePathMutation = useCallback(async (sourcePath: string, destinationPath: string, sourceRoot: string) => {
    const root = workspaceStateRef.current.kind === "ready"
      ? workspaceStateRef.current.workspace.project.project_root
      : workspaceStateRef.current.previous?.project.project_root;
    if (!root || sourceMutationBlocked(root)) return;
    setPathMutationBusy(true);
    setPathMutationResult(null);
    try {
      const report = await invoke<SourcePathMutationReport>("rename_source_file", {
        projectPath: root,
        request: { sourcePath, destinationPath },
      });
      if (report.status === "success") {
        // Invalidate in-flight reads, previews, and polling callbacks that
        // still address the old path before rebinding the editor state.
        workspaceGeneration.current += 1;
        removeEditorForPath(sourcePath);
        dataEditorUi.current.delete(`${root}:${sourcePath}`);
        dataEditorUi.current.delete(`${root}:${destinationPath}`);
        setLoadingPaths((current) => {
          if (!current.has(sourcePath)) return current;
          const next = new Set(current);
          next.delete(sourcePath);
          return next;
        });
        if (pendingRecordFocus.current?.path === sourcePath) {
          pendingRecordFocus.current = { ...pendingRecordFocus.current, path: destinationPath };
        }
        setPathMutationResult({ kind: "report", report });
        setPathMutationPhase("result");
        setActivePath(destinationPath);
        setExplorerPath(destinationPath);
        setRevealCreated({ path: destinationPath, root: sourceRoot });
        try {
          const next = await invoke<AuthoringWorkspace>("authoring_workspace", { projectPath: root });
          setWorkspaceState({ kind: "ready", workspace: next });
          const destinationFile = next.files.find((file) => file.path === destinationPath);
          const destinationRoot = destinationFile?.sourceRoot ?? sourceRoot;
          setRevealCreated({ path: destinationPath, root: destinationRoot });
          if (destinationFile && (destinationFile.kind === "data" || destinationFile.hasInlineRecords)) await openDataFile(root, destinationPath, true);
          showNotice(`${sourceName(sourcePath)} moved to ${destinationPath}`);
        } catch (error) {
          // The mutation report is authoritative even if the post-success
          // Explorer refresh cannot be completed. Keep the report visible and
          // leave unrelated dirty buffers untouched for the next refresh.
          showNotice(`Source moved, but Explorer refresh failed: ${asApiError(error).diagnostic.message}`);
        }
      } else {
        setPathMutationResult({ kind: "report", report });
        setPathMutationPhase("result");
      }
    } catch (error) {
      setPathMutationResult({ kind: "error", diagnostic: asApiError(error).diagnostic });
      setPathMutationPhase("result");
    } finally {
      setPathMutationBusy(false);
    }
  }, [openDataFile, removeEditorForPath, showNotice, sourceMutationBlocked]);

  const submitPathMutation = useCallback(async () => {
    const target = pathMutationTarget;
    if (!target || !target.destinationPath.trim() || target.sourcePath === target.destinationPath) return;
    const editor = editorsRef.current[target.sourcePath];
    if (editor && editorIsDirty(editor)) {
      setPathMutationPhase("dirty");
      return;
    }
    await executePathMutation(target.sourcePath, target.destinationPath, target.sourceRoot);
  }, [executePathMutation, pathMutationTarget]);

  const saveDirtyPathTargetAndMove = useCallback(async () => {
    const target = pathMutationTarget;
    if (!target) return;
    await resolveDirtyPathMutation({
      decision: "save",
      save: async () => {
        setPathMutationBusy(true);
        const saved = await saveFile(target.sourcePath);
        setPathMutationBusy(false);
        return saved;
      },
      discard: async () => {},
      mutate: () => executePathMutation(target.sourcePath, target.destinationPath, target.sourceRoot),
    });
  }, [executePathMutation, pathMutationTarget, saveFile]);

  const discardDirtyPathTargetAndMove = useCallback(async () => {
    const target = pathMutationTarget;
    if (!target) return;
    await resolveDirtyPathMutation({
      decision: "discard",
      save: async () => false,
      discard: () => removeEditorForPath(target.sourcePath),
      mutate: () => executePathMutation(target.sourcePath, target.destinationPath, target.sourceRoot),
    });
  }, [executePathMutation, pathMutationTarget, removeEditorForPath]);

  const recheckPathMutation = useCallback(async () => {
    const target = pathMutationTarget;
    const root = workspaceStateRef.current.kind === "ready"
      ? workspaceStateRef.current.workspace.project.project_root
      : workspaceStateRef.current.previous?.project.project_root;
    if (!target || !root) return;
    setPathMutationBusy(true);
    try {
      const state = await invoke<SourcePathStateReport>("source_path_state", {
        projectPath: root,
        request: { sourcePath: target.sourcePath, destinationPath: target.destinationPath },
      });
      setPathMutationResult({ kind: "state", state });
    } catch (error) {
      setPathMutationResult({ kind: "error", diagnostic: asApiError(error).diagnostic });
    } finally {
      setPathMutationBusy(false);
    }
  }, [pathMutationTarget]);

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
    if ((file.kind === "data" || file.hasInlineRecords) && record !== null && field) {
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
    setExplorerPath(report.path);
    setSelectedTable(next.files.find((file) => file.path === report.path)?.table ?? null);
    const root = report.folder ? next.folders?.find(folder => folder.path === report.path)?.sourceRoot
      : next.files.find(file => file.path === report.path)?.sourceRoot;
    setRevealCreated({ path: report.path, root: root ?? "" });
    setCreationOpen(false);
    const file = next.files.find(file => file.path === report.path);
    if (file && (file.kind === "data" || file.hasInlineRecords)) await openDataFile(projectRoot, file.path);
    showNotice(`${sourceName(report.path)} created`);
  }, [projectRoot, openDataFile, showNotice]);

  const refreshMigrationFiles = useCallback(async (root: string, paths: string[]) => {
    const state = workspaceStateRef.current;
    if (state.kind !== "ready" || state.workspace.project.project_root !== root) return;
    const generation = workspaceGeneration.current;
    // Evict affected clean snapshots before awaiting reload, so they cannot be
    // edited against an obsolete schema (GUI-UNIFIED-004).
    const { reloadPaths: reload, retainedEditors: retained } = migrationRefreshPlan(
      editorsRef.current,
      paths,
      editorIsDirty,
    );
    editorsRef.current = retained; setEditors(retained);
    setTableEpoch(epoch => epoch + 1);
    const next = await invoke<AuthoringWorkspace>("authoring_workspace", { projectPath: root });
    if (workspaceGeneration.current !== generation) return;
    setWorkspaceState({ kind: "ready", workspace: next });
    await Promise.all(reload.filter(path => next.files.some(file => file.path === path && (file.kind === "data" || file.hasInlineRecords))).map(path => openDataFile(root, path, true)));
  }, [openDataFile]);
  const migrationResult = useCallback(async (root: string, result: MigrationResult) => {
    if (result.state === "recovery_required") recordRecovery(root, result);
    if (result.state === "success") await refreshMigrationFiles(root, result.files);
  }, [recordRecovery, refreshMigrationFiles]);
  const applyColumnIntent = async (input: ColumnIntent) => {
    if (!projectRoot || sourceMutationBlocked(projectRoot)) throw new Error("Source changes are currently unavailable.");
    if (!tableContext || tableContext.table !== input.table) throw new Error("Reload the Table before changing columns.");
    if (input.operation === "change_declaration") {
      changeSchemaDraft(tableContext.schemaPath, input.field, input);
      return;
    }
    if (input.operation === "reorder") {
      reorderSchemaDraft(tableContext.schemaPath, input.field, input.newIndex);
      pendingColumnFocus.current = { table: input.table, field: input.field, grip: true };
      return;
    }
    if (activeSchemaDraft && schemaDraftIsDirty(activeSchemaDraft)) throw new Error("Save or undo the pending type and modifier changes before changing the Table structure.");
    migrationBusyRef.current = projectRoot;
    setMigrationBusyRoot(projectRoot);
    try {
      const dirtyPaths = Object.entries(editorsRef.current)
        .filter(([, editor]) => editorIsDirty(editor) || editor.saving)
        .map(([path]) => path);
      const expectedSources = [{ path: tableContext.schemaPath, contentIdentity: tableContext.schemaContentIdentity }];
      if (activePath && activeEditor) expectedSources.push({ path: activePath, contentIdentity: activeEditor.snapshot.baseContentIdentity });
      let result: MigrationResult;
      try {
        result = await invoke<MigrationResult>("apply_table_intent", { projectPath: projectRoot, input, expectedSources, dirtyPaths });
      } catch (error) {
        if (error && typeof error === "object" && "diagnostic" in error) throw error;
        const unknown: MigrationResult = { state: "recovery_required", files: [tableContext?.schemaPath ?? ""], diagnostic: { code: "E-MIGRATION-TRANSPORT", message: String(error) } };
        recordRecovery(projectRoot, unknown);
        throw error;
      }
      if (result.state === "recovery_required") recordRecovery(projectRoot, result);
      if (result.state !== "success") throw new Error(result.diagnostic?.message ?? result.state);
      let focusField = input.operation === "add_default" ? "last" : input.newName;
      if (input.operation === "add_default") {
        let serial = tableContext.schema.schema.fields.length + 1;
        while (tableContext.schema.schema.fields.some(field => field.name === `field${serial}`)) serial++;
        focusField = `field${serial}`;
      }
      pendingColumnFocus.current = { table: input.table, field: focusField };
      const paths = [...new Set([...result.files, ...(tableContext?.recordSources.map(source => source.path) ?? [])])];
      await refreshMigrationFiles(projectRoot, paths);
    } finally {
      if (migrationBusyRef.current === projectRoot) {
        migrationBusyRef.current = null;
        setMigrationBusyRoot(null);
      }
    }
  };
  const recheckMigration = async () => {
    if (!projectRoot || !recovery) return;
    try {
      const result = await invoke<MigrationResult | null>("recheck_migration", { projectPath: projectRoot });
      if (!result) await refreshMigrationFiles(projectRoot, recovery.files);
      recordRecovery(projectRoot, result);
    } catch (error) { showNotice(asApiError(error).diagnostic.message); }
  };

  const activeDiagnostics = activeSchemaDraft && schemaDraftIsDirty(activeSchemaDraft) && activeSchemaDraft.previewState === "current" && activeSchemaDraft.preview
    ? activeSchemaDraft.preview.validation.diagnostics
    : activeEditor?.previewState === "current"
    ? activeEditor.preview.validation.diagnostics
    : [];
  const manualDiagnostics = manualValidation.kind === "done" ? manualValidation.value.diagnostics : [];
  const operationDiagnostics = [
    ...(activeSchemaDraft?.previewError ? [activeSchemaDraft.previewError] : []),
    ...(activeSchemaDraft?.saveDiagnostic ? [activeSchemaDraft.saveDiagnostic] : []),
    ...(activeEditor?.saveDiagnostic ? [activeEditor.saveDiagnostic] : []),
    ...(buildState.kind === "error" ? [apiDiagnosticToDiagnostic(buildState.diagnostic)] : []),
    ...(manualValidation.kind === "error" ? [apiDiagnosticToDiagnostic(manualValidation.diagnostic)] : []),
  ];
  const problems = [
    ...activeDiagnostics.map((diagnostic) => ({ diagnostic, origin: "Buffer" })),
    ...manualDiagnostics.map((diagnostic) => ({ diagnostic, origin: "Saved source" })),
    ...operationDiagnostics.map((diagnostic) => ({ diagnostic, origin: "Operation" })),
  ];
  const openCreation = (category: "folder" | "table" | "data" | "value_object", table = "") => {
    setCreationPreset({ category, table });
    setCreationOpen(true);
  };
  const openInlineCreation = (category: Category, table = "") => {
    if (!workspace || mutationBlocked) return;
    setInlineCreation({ root: creationTarget.root || workspace.sourceRoots[0] || "", folder: creationTarget.folder, category, table });
  };
  const openDataCreation = (table: string) => {
    const parent = workspace?.files.find((file) => file.table === table && file.kind === "schema")
      ?? workspace?.files.find((file) => file.table === table && file.kind === "data");
    const relative = parent?.sourceRoot && parent.path.startsWith(`${parent.sourceRoot}/`) ? parent.path.slice(parent.sourceRoot.length + 1) : parent?.path ?? "";
    const root = parent?.sourceRoot ?? workspace?.sourceRoots[0] ?? "";
    const folder = relative.split("/").slice(0, -1).join("/");
    setCreationTarget({ root, folder });
    setInlineCreation({ root, folder, category: "data", table });
  };
  const overviewTable = selectedTable ?? activeFile?.table ?? workspace?.files.find((file) => file.table)?.table ?? null;
  const recordFileActive = activeFile?.kind === "data" || !!activeFile?.hasInlineRecords;
  const activeTableName = activeFile?.table ?? null;
  const activeSchema = activeFile?.kind === "schema"
    ? activeFile
    : workspace?.files.find(file => file.kind === "schema" && file.table === activeTableName);
  const tableEditor = activeSchema && projectRoot && activeTableName ? <TableEditor key={`${projectRoot}:${activeSchema.path}:${tableEpoch}`}
    projectPath={projectRoot} path={activeSchema.path} canWrite={!mutationBlocked} embedded schemaAction={schemaAction} onSchemaActionConsumed={() => setSchemaAction(null)}
    onOverview={() => { setSelectedTable(activeTableName); setSurface("overview"); }}
    onCreateData={() => openDataCreation(activeTableName)}
    dirtyPaths={Object.entries(editors).filter(([,editor]) => editorIsDirty(editor) || editor.saving).map(([path]) => path)}
    beginApply={paths => {
      if (sourceMutationBlocked(projectRoot) || paths.some(path => editorsRef.current[path] && (editorIsDirty(editorsRef.current[path]) || editorsRef.current[path].saving))) return false;
      migrationBusyRef.current = projectRoot; setMigrationBusyRoot(projectRoot); return true;
    }}
    endApply={() => { if (migrationBusyRef.current === projectRoot) { migrationBusyRef.current = null; setMigrationBusyRoot(null); } }}
    onResult={result => migrationResult(projectRoot, result)} /> : null;
  const [collapseTreeSignal, setCollapseTreeSignal] = useState(0);
  const refreshExplorer = async () => {
    if (!projectRoot) return;
    try {
      const next = await invoke<AuthoringWorkspace>("authoring_workspace", { projectPath: projectRoot });
      if (next.project.project_root === projectRoot) setWorkspaceState({ kind: "ready", workspace: next });
    } catch (error) {
      showNotice(asApiError(error).diagnostic.message);
    }
  };

  const schemaDraftActions = activeSchemaDraft && (schemaDraftIsDirty(activeSchemaDraft) || activeSchemaDraft.historyFuture.length > 0 || activeSchemaDraft.saveDiagnostic)
    ? <Dropdown trigger={["click"]} menu={{ items: [
      { key: "undo", label: "Undo schema change", disabled: !activeSchemaDraft.historyPast.length || activeSchemaDraft.saving, onClick: () => undoSchemaDraft(activeSchemaDraft.path) },
      { key: "redo", label: "Redo schema change", disabled: !activeSchemaDraft.historyFuture.length || activeSchemaDraft.saving, onClick: () => undoSchemaDraft(activeSchemaDraft.path, true) },
      ...(activeSchemaDraft.saveStatus === "outcome_unknown" ? [{ key: "recheck", label: "Recheck save", onClick: () => void recheckSchemaDraft(activeSchemaDraft.path) }] : []),
      ...(activeSchemaDraft.saveStatus === "conflict" ? [{ key: "reload", label: "Discard draft and reload schema", onClick: () => void reloadSchemaDraft(activeSchemaDraft.path) }] : []),
    ] }}>
      <Button htmlType="button" size="small" className="schema-status-action"
        aria-label={schemaDraftIsDirty(activeSchemaDraft) ? "Schema unsaved changes and actions" : "Schema redo available"}
        title={activeSchemaDraft.saveDiagnostic?.message}>
        {activeSchemaDraft.saving ? "Saving schema…" : schemaDraftIsDirty(activeSchemaDraft) ? "Schema unsaved" : "Schema redo"}
        <ChevronDown size={12} aria-hidden="true" />
      </Button>
    </Dropdown> : null;

  return (
    <ConfigProvider theme={antThemeConfig}>
      <main className={`app-shell${/Mac/.test(navigator.platform) ? " macos-titlebar" : ""}`}>
      {/* Tauri checks the exact event target for dragging, so passive children
          also carry the region attribute; interactive controls keep their clicks. */}
      <header className="titlebar" data-tauri-drag-region aria-label="Window toolbar">
        <div className="brand-block" data-tauri-drag-region>
          <div className="brand-mark" aria-hidden="true" data-tauri-drag-region><Database size={14} /></div>
          <div data-tauri-drag-region>
            <strong data-tauri-drag-region>masterdata</strong>
            <span data-tauri-drag-region title={workspace?.project.name}>{workspace?.project.name ?? "No project"}</span>
          </div>
        </div>
        <div className="project-open" data-tauri-drag-region>
          <span className="project-location" data-tauri-drag-region title={projectRoot ?? undefined}>{projectRoot ?? "Choose a folder to begin"}</span>
        </div>
        <div className="command-bar" data-tauri-drag-region>
          {totalDirtyCount > 0 && <span className="header-dirty" role="status" data-tauri-drag-region>{totalDirtyCount} unsaved</span>}
          {manualValidation.kind === "loading" && <span className="header-operation" role="status" data-tauri-drag-region>Validating…</span>}
          {buildState.kind === "loading" && <span className="header-operation" role="status" data-tauri-drag-region>Building…</span>}
          <Button
            htmlType="button"
            aria-label="Application Settings"
            icon={<Settings size={14} aria-hidden="true" />}
            onClick={() => setAppSettingsOpen(true)}
          >
            Settings
          </Button>
          <Dropdown menu={{ items: [
            { key: "open", label: projectPickerBusy ? "Opening Project…" : "Open Project", disabled: projectPickerBusy, onClick: () => void openProjectPicker() },
            { key: "create", label: "Create Project", onClick: () => requestAction({ kind: "create" }) },
            ...(workspace ? [{ key: "reload", label: "Reload Project", onClick: () => requestAction({ kind: "reload" }) }] : []),
            ...(workspace ? [
              { key: "settings", label: "Project Settings", onClick: () => setSurface("settings") },
              { key: "delivery", label: "Build & Publish", onClick: () => setSurface("delivery") },
              { key: "validate", label: "Validate", onClick: () => { setSurface("editor"); void validateDisk(); } },
              { key: "build", label: "Build", disabled: mutationBlocked || deliveryBusy, onClick: () => { setSurface("editor"); void runBuild(); } },
            ] : []),
            { type: "divider" },
            { key: "app-settings", label: "Application Settings…", onClick: () => setAppSettingsOpen(true) },
          ] }} trigger={["click"]}>
            <Button htmlType="button" aria-label="Project menu">Project <ChevronDown size={14} aria-hidden="true" /></Button>
          </Dropdown>
        </div>
      </header>

      {surface === "editor" && !workspace && (
        <section className="welcome" aria-label="Welcome">
          <div className="welcome-intro">
            <div className="welcome-mark"><Database size={34} /></div>
            <span className="dialog-kicker">MASTERDATA DESKTOP</span>
            <h1>Start with a Masterdata project</h1>
            <p>Open an existing project folder or create a new project to start editing your master data.</p>
            <div className="welcome-actions">
              <Button type="primary" size="large" icon={<FolderOpen size={17} />} loading={projectPickerBusy} onClick={() => void openProjectPicker()}>
                Open Project
              </Button>
              <Button size="large" onClick={() => requestAction({ kind: "create" })}>Create Project</Button>
              <Button size="large" icon={<Settings size={16} />} onClick={() => setAppSettingsOpen(true)}>Settings</Button>
            </div>

            {workspaceState.kind === "loading" && <p className="welcome-status" role="status">Looking for a configured project…</p>}
            {workspaceState.kind === "error" && !workspaceState.previous && (
              <DiagnosticBanner diagnostic={apiDiagnosticToDiagnostic(workspaceState.diagnostic)} />
            )}
          </div>
          <aside className="welcome-recent" aria-label="Recent Projects">
            <div className="welcome-section-heading">
              <span className="dialog-kicker">RECENT</span>
              <h2>Recent Projects</h2>
            </div>
            {recentProjects.length === 0 ? (
              <p className="recent-empty">Projects you open will appear here.</p>
            ) : (
              <div className="recent-list">
                {recentProjects.map((project) => (
                  <div className="recent-project" key={project.root}>
                    <Button className="recent-project-open" htmlType="button" onClick={() => requestAction({ kind: "open", projectPath: project.root })}>
                      <span><strong>{project.name}</strong><small>{project.root}</small></span>
                      <ArrowRight size={15} aria-hidden="true" />
                    </Button>
                    <Button className="recent-project-remove" type="text" aria-label={`Remove ${project.name} from Recent Projects`} icon={<X size={14} />} onClick={() => removeRecentProject(project.root)} />
                  </div>
                ))}
              </div>
            )}
          </aside>
        </section>
      )}

      <div className="workbench" hidden={!workspace && surface !== "create"}>
      {workspace && surface !== "create" && <aside className="workspace-navigation explorer-pane" aria-label="Explorer">
        <div className="explorer-toolbar">
          <strong>EXPLORER</strong>
          <Dropdown menu={{ items: [
            { key: "table", label: "Table", onClick: () => openInlineCreation("table") },
            { key: "data", label: "Data", onClick: () => openInlineCreation("data") },
            { key: "value_object", label: "Value Object", onClick: () => openInlineCreation("value_object") },
            { key: "enum", label: "Enum", onClick: () => openInlineCreation("enum") },
            { key: "flags", label: "Flags Enum", onClick: () => openInlineCreation("flags") },
            { key: "custom_type", label: "Custom Type", onClick: () => openInlineCreation("custom_type") },
            { type: "divider" },
            { key: "advanced", label: "Advanced…", onClick: () => openCreation("table") },
          ] }} trigger={["click"]}>
            <Button type="text" size="small" aria-label="New source artifact" disabled={mutationBlocked} icon={<FilePlus2 size={15} />} />
          </Dropdown>
          <Button type="text" size="small" aria-label="New folder" disabled={mutationBlocked} icon={<FolderPlus size={15} />} onClick={() => openInlineCreation("folder")} />
          <Button type="text" size="small" aria-label="Refresh Explorer" icon={<RefreshCw size={15} />} onClick={() => void refreshExplorer()} />
          <Button type="text" size="small" aria-label="Collapse folders" icon={<ChevronsUp size={15} />} onClick={() => setCollapseTreeSignal((value) => value + 1)} />
          <Dropdown menu={{ items: [
            { key: "move", label: "Rename or Move Source", disabled: mutationBlocked || !activeFile, onClick: () => openPathMutation() },
          ] }} trigger={["click"]}>
            <Button type="text" size="small" aria-label="More Explorer actions" icon={<MoreHorizontal size={15} />} />
          </Dropdown>
        </div>
        <SourceTree
          workspace={workspace}
          activePath={explorerPath ?? activePath}
          editors={editors}
          loadingPaths={loadingPaths}
          fileOpenErrors={fileOpenErrors}
          onSelect={selectFile}
          onFolderSelect={(root, folder) => setCreationTarget({ root, folder })}
          onRename={(file) => openPathMutation(file)}
          collapseSignal={collapseTreeSignal}
          revealCreated={revealCreated}
          inlineCreation={inlineCreation ? { ...inlineCreation, node: projectRoot && <InlineSourceCreation
            key={`${projectRoot}:${inlineCreation.root}:${inlineCreation.folder}:${inlineCreation.category}`}
            projectPath={projectRoot} sourceRoot={inlineCreation.root} sourceRootIndex={workspace.sourceRoots.indexOf(inlineCreation.root)} folder={inlineCreation.folder}
            category={inlineCreation.category} contextTable={inlineCreation.table} canWrite={!mutationBlocked}
            onCancel={() => { setInlineCreation(null); window.requestAnimationFrame(() => document.querySelector<HTMLElement>(`[data-tree-path="${CSS.escape([inlineCreation.root, inlineCreation.folder].filter(Boolean).join("/"))}"]`)?.focus()); }}
            onAdvanced={(filename, identity) => { setCreationTarget({ root: inlineCreation.root, folder: inlineCreation.folder }); setCreationPreset({ category: inlineCreation.category, table: inlineCreation.table, filename, name: identity ?? "" }); setInlineCreation(null); setCreationOpen(true); }}
            onCreated={async report => { await refreshAfterCreation(report); setInlineCreation(null); }} /> } : null}
        />
      </aside>}
      <section className="surface-layout" hidden={surface !== "overview"}>
        <ProjectOverviewPanel
          // A Table switch must not show the previous saved snapshot under the new Table heading.
          key={JSON.stringify([projectRoot, configRevision, overviewTable])}
          active={surface === "overview"}
          projectRoot={projectRoot}
          workspace={workspace as SurfaceWorkspace | null}
          table={overviewTable}
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
      <section className="surface-layout create-surface" hidden={surface !== "create"}>
        <ProjectCreatePanel
          active={surface === "create"}
          onCancel={() => setSurface(workspace ? createReturnSurface.current : "editor")}
          onCreated={(root) => {
            setSurface("editor");
            void loadWorkspace(root);
          }}
        />
      </section>

      {workspace && <section className="workspace-layout" hidden={surface !== "editor"}>
        <section className="editor-area" aria-label="Editor" data-active-source={activePath} tabIndex={-1}>
          {workspaceState.kind === "error" && workspace && (
            <div className="workspace-error-strip">
              <strong>{workspaceState.diagnostic.code}</strong>
              <span>{workspaceState.diagnostic.message}</span>
            </div>
          )}
          {recovery && <Alert role="alert" type="error" title="Recovery Required — source changes and Build are blocked"
            description={<><p>{recovery.diagnostic?.message}</p><p>{recovery.files.join(", ")}</p>{recovery.fileStates?.map(file => <p key={file.path}>{file.path}: {file.state}</p>)}{recovery.recoveryWorkspace && <p>Recovery workspace: {recovery.recoveryWorkspace}</p>}</>}
            action={<Button onClick={() => void recheckMigration()}>Recheck recovered source</Button>} />}
          {workspace.files.length === 0 && !activeFile && <div className="empty-project">
            <span className="dialog-kicker">NEW PROJECT</span>
            <h2>Start with a Table</h2>
            <p>Create a Table with records in one file, or keep records in separate data files.</p>
            <div><Button type="primary" disabled={mutationBlocked} onClick={() => openInlineCreation("table")}>Create Table</Button><Button disabled={mutationBlocked} onClick={() => openInlineCreation("value_object")}>Create Type</Button><Button disabled={mutationBlocked} onClick={() => openInlineCreation("folder")}>Create Folder</Button></div>
          </div>}
          {workspace.files.length > 0 && !activeFile && (
            <EmptyEditor title="Select a source file" copy="Choose a YAML document from the Workspace Explorer." />
          )}
          {activeFile?.kind === "schema" && !activeFile.hasInlineRecords && !tableContext && !tableContextError && (
            <EmptyEditor title={`Opening ${activeFile.table ?? "Table"}…`} copy="Loading Table context." />
          )}
          {activeFile?.kind === "schema" && !activeFile.hasInlineRecords && tableContextError && (
            <DiagnosticBanner diagnostic={apiDiagnosticToDiagnostic(tableContextError)} />
          )}
          {activeFile?.kind === "schema" && !activeFile.hasInlineRecords && tableContext && !tableContext.selectedRecordSource && (
            <EmptyTableSurface context={tableContext} disabled={mutationBlocked || !!activeSchemaDraft?.saving || activeSchemaDraft?.saveStatus === "outcome_unknown"} onIntent={applyColumnIntent}
              schemaDraftActions={schemaDraftActions}
              onUndoSchema={redo => undoSchemaDraft(tableContext.schemaPath, redo)}
              onCreateData={() => openDataCreation(tableContext.table)} details={tableEditor}
              saveEnabled={!!activeSchemaDraft && schemaDraftIsDirty(activeSchemaDraft)}
              onSave={() => void saveCurrentTableContext(tableContext, null)} />
          )}
          {activeFile?.kind === "type" && projectRoot && <TypeEditor key={`${projectRoot}:${activeFile.path}:${tableEpoch}`}
            projectPath={projectRoot} path={activeFile.path} canWrite={!mutationBlocked}
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
          {recordFileActive && activeLoading && (
            <EmptyEditor title={`Loading ${sourceName(activeFile.path)}…`} copy="Refreshing records through the shared application service." />
          )}
          {recordFileActive && !activeLoading && activeLoadDiagnostic && (
            <section className="placeholder-editor">
              <h2>{sourceName(activeFile.path)} is unavailable</h2>
              <p>The previous clean snapshot is not editable until the source can be loaded safely.</p>
              <DiagnosticBanner diagnostic={apiDiagnosticToDiagnostic(activeLoadDiagnostic)} />
            </section>
          )}
          {recordFileActive && !activeLoading && !activeLoadDiagnostic && !activeEditor && (
            <EmptyEditor title={`Opening ${sourceName(activeFile.path)}…`} copy="Loading records through the shared application service." />
          )}
          {recordFileActive && !activeLoading && !activeLoadDiagnostic && activeEditor && (
            <DataEditor key={`${projectRoot}:${activeFile.path}`}
              schemaDraftActions={schemaDraftActions}
              contextDirty={(!!activeSchemaDraft && schemaDraftIsDirty(activeSchemaDraft))
                || (!!tableContext?.recordSources.some(source => source.path === tableContext.schemaPath && source.inline)
                  && !!editors[tableContext.schemaPath] && editorIsDirty(editors[tableContext.schemaPath]))}
              contextSaving={!!activeSchemaDraft?.saving || !!editors[tableContext?.schemaPath ?? ""]?.saving}
              mutationBlocked={mutationBlocked}
              schemaDraftBlocked={!!activeSchemaDraft?.saving || activeSchemaDraft?.saveStatus === "outcome_unknown"}
              file={activeFile}
              projectRoot={projectRoot!}
              editor={displayEditor!}
              schemaEditor={tableEditor}
              tableContext={tableContext}
              onRecordSourceSelect={path => { setActivePath(path); setExplorerPath(path); if (projectRoot) void openDataFile(projectRoot, path); }}
              onColumnIntent={applyColumnIntent}
              onUndoSchema={redo => { if (tableContext) undoSchemaDraft(tableContext.schemaPath, redo); }}
              onOverview={activeFile.table ? () => { setSelectedTable(activeFile.table); setSurface("overview"); } : undefined}
              onCreateData={activeFile.table ? () => openDataCreation(activeFile.table!) : undefined}
              uiCache={dataEditorUi}
              onCellChange={(recordIndex, field, value, operation) => updateCell(activeFile.path, recordIndex, field, value, operation)}
              onDraftCellChange={(draftId, field, value, operation) => updateDraftCell(activeFile.path, draftId, field, value, operation)}
              onCellFocus={(key) => { historyEditKey.current = key; }}
              onTagsChange={(recordIndex, tags) => updateExistingTags(activeFile.path, recordIndex, tags)}
              onDraftTagsChange={(draftId, tags) => updateDraftTags(activeFile.path, draftId, tags)}
              onBatchApplied={(batch, expectedRevision) => applyBatchPreview(activeFile.path, batch, expectedRevision)}
              onQueryResult={(result) => updateQueryResult(activeFile.path, result)}
              onUndo={() => undoBuffer(activeFile.path)}
              onRedo={() => redoBuffer(activeFile.path)}
              onAddRow={() => addRow(activeFile.path)}
              onInsertRow={position => addRow(activeFile.path, position)}
              onMoveRow={(row, position) => moveRow(activeFile.path, row, position)}
              onDeleteExistingRow={(recordIndex) => deleteExistingRow(activeFile.path, recordIndex)}
              onUndoExistingDelete={(recordIndex) => undoExistingDelete(activeFile.path, recordIndex)}
              onDeleteDraftRow={(draftId) => deleteDraftRow(activeFile.path, draftId)}
              onSave={() => { if (tableContext) void saveCurrentTableContext(tableContext, activeFile.path); }}
              onRecheckSource={() => void recheckSource(activeFile.path)}
              onSwitchView={(view) => switchView(activeFile.path, view)}
              onReloadConflict={() => void reloadConflict(activeFile.path)}
              onOverwriteConflict={() => void overwriteConflict(activeFile.path)}
            />
          )}

          <section className={`problems-panel ${problemsOpen ? "open" : "collapsed"}`}>
            <Button className="problems-header" htmlType="button" onClick={() => setProblemsOpen((value) => !value)}>
              <span>PROBLEMS <b>{problems.length}</b></span>
              <span className="problem-snapshot">
                {activeEditor?.previewState === "pending" && "Buffer validation pending"}
                {activeEditor?.previewState === "unavailable" && "Buffer validation unavailable"}
                {activeEditor?.previewState === "current" && !activeEditor.preview.validation.valid && "Buffer has diagnostics"}
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
      </div>

      {workspace && <footer className="statusbar">
        <span>{surface === "settings" ? "Project Settings" : surface === "delivery" ? "Build & Publish" : surface === "overview" ? `Table ${selectedTable ?? ""}` : activePath ?? "No source selected"}</span>
        <span>{surface === "editor" && activeEditor ? `${activeEditor.snapshot.rows.length + activeEditor.addedRecords.length} records · ${activeEditor.snapshot.columns.length} fields` : ""}</span>
        <span>{totalDirtyCount > 0 ? `${totalDirtyCount} dirty` : ""}</span>
      </footer>}

      {creationOpen && workspace && projectRoot && <SourceCreation key={projectRoot}
        projectPath={projectRoot}
        initialRootIndex={Math.max(0, workspace.sourceRoots.indexOf(creationTarget.root))}
        initialFolder={creationTarget.folder}
        initialCategory={creationPreset.category}
        initialTable={creationPreset.table}
        initialFilename={creationPreset.filename}
        initialName={creationPreset.name}
        canWrite={!mutationBlocked}
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

      <Modal
        open={pathMutationTarget !== null}
        title={pathMutationPhase === "dirty" ? "Save changes before moving?" : pathMutationPhase === "result" ? "Source move result" : "Rename or move source"}
        closable={!pathMutationBusy}
        keyboard={!pathMutationBusy}
        mask={{ closable: false }}
        onCancel={closePathMutation}
        footer={pathMutationPhase === "dirty" ? [
          <Button key="cancel" disabled={pathMutationBusy} onClick={closePathMutation}>Cancel</Button>,
          <Button key="discard" disabled={pathMutationBusy} onClick={() => void discardDirtyPathTargetAndMove()}>Don&apos;t Save</Button>,
          <Button key="save" type="primary" loading={pathMutationBusy} disabled={mutationBlocked} onClick={() => void saveDirtyPathTargetAndMove()}>Save</Button>,
        ] : pathMutationPhase === "result" ? [
          <Button key="close" type="primary" disabled={pathMutationBusy} onClick={closePathMutation}>Close</Button>,
          ...(pathMutationResult?.kind === "report" && pathMutationResult.report.status === "conflict"
            ? [<Button key="change" disabled={pathMutationBusy} onClick={() => {
                setPathMutationResult(null);
                setPathMutationPhase("form");
              }}>Change destination</Button>]
            : []),
          ...(pathMutationResult?.kind === "report" && pathMutationResult.report.status === "outcome_unknown"
            ? [<Button key="recheck" loading={pathMutationBusy} onClick={() => void recheckPathMutation()}>Recheck path state</Button>]
            : []),
        ] : [
          <Button key="cancel" disabled={pathMutationBusy} onClick={closePathMutation}>Cancel</Button>,
          <Button key="move" type="primary" loading={pathMutationBusy} disabled={mutationBlocked || !pathMutationTarget?.destinationPath.trim() || pathMutationTarget?.destinationPath === pathMutationTarget?.sourcePath} onClick={() => void submitPathMutation()}>Move</Button>,
        ]}
      >
        {pathMutationTarget && pathMutationPhase === "form" && (
          <>
            <p>Move an existing source file inside the same configured source root. The destination must be a new <code>.yaml</code> or <code>.yml</code> path.</p>
            <Input
              aria-label="Source destination path"
              value={pathMutationTarget.destinationPath}
              onChange={(event) => {
                setPathMutationTarget((current) => current ? { ...current, destinationPath: event.target.value } : current);
                setPathMutationResult(null);
              }}
              placeholder="sources/data/renamed.yaml"
              disabled={pathMutationBusy}
            />
          </>
        )}
        {pathMutationTarget && pathMutationPhase === "dirty" && (
          <p>{sourceName(pathMutationTarget.sourcePath)} has unsaved changes. Choose Save, Don&apos;t Save, or Cancel before the source mutation.</p>
        )}
        {pathMutationPhase === "result" && pathMutationResult?.kind === "report" && (
          <div className="path-mutation-result" role="status">
            <strong>{pathMutationResult.report.status === "success" ? "Move succeeded." : pathMutationResult.report.status === "conflict" ? "Move conflicted; no overwrite was performed." : pathMutationResult.report.status === "outcome_unknown" ? "Move outcome is unknown. Recheck the old and new paths before retrying." : "Move failed."}</strong>
            {pathMutationResult.report.diagnostic && <DiagnosticBanner diagnostic={pathMutationResult.report.diagnostic} />}
          </div>
        )}
        {pathMutationPhase === "result" && pathMutationResult?.kind === "state" && (
          <div className="path-mutation-result" role="status">
            <strong>Current path state</strong>
            <span>{pathMutationResult.state.source.path}: {pathMutationResult.state.source.exists ? "present" : "missing"}</span>
            <span>{pathMutationResult.state.destination.path}: {pathMutationResult.state.destination.exists ? "present" : "missing"}</span>
          </div>
        )}
        {pathMutationPhase === "result" && pathMutationResult?.kind === "error" && (
          <DiagnosticBanner diagnostic={apiDiagnosticToDiagnostic(pathMutationResult.diagnostic)} />
        )}
      </Modal>

      <Modal open={tableSaveFailure !== null} title="Save incomplete" footer={<Button onClick={() => setTableSaveFailure(null)}>Close</Button>}
        onCancel={() => setTableSaveFailure(null)}>
        <p>Some current Table sources were not saved. Unsaved drafts remain available.</p>
        <ul>{tableSaveFailure?.files.map(file => <li key={file.path}>
          <strong>{file.path}</strong>: {file.status === "success" ? "saved" : file.status === "unchanged" ? "unchanged" : file.status === "not_attempted" ? "not attempted" : file.status.replace("_", " ")}
          {file.diagnostic && <> — {file.diagnostic.message}</>}
        </li>)}</ul>
      </Modal>

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
        <p>{deliveryBusy && "A Build or Publish operation is still running. Wait for it to finish before continuing. "}{totalDirtyCount > 0 && <>Unsaved changes in {[dirtyCount > 0 ? `${dirtyCount} source file${dirtyCount === 1 ? "" : "s"}` : null, settingsDirty ? "Project Settings (masterdata.toml)" : null].filter(Boolean).join(" and ")}. Continuing without saving will discard these changes.</>}</p>
      </Modal>

      {notice && <Alert className="toast" title={notice} type="info" showIcon role="status" />}

      <ApplicationSettingsModal
        open={appSettingsOpen}
        onClose={() => setAppSettingsOpen(false)}
        themePreference={themePreference}
        effectiveTheme={effectiveTheme}
        onThemePreferenceChange={handleThemePreferenceChange}
      />
    </main>
  </ConfigProvider>
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
  onRename,
  collapseSignal,
  revealCreated,
  inlineCreation,
}: {
  workspace: AuthoringWorkspace;
  activePath: string | null;
  editors: Record<string, EditorState>;
  loadingPaths: Set<string>;
  fileOpenErrors: Record<string, ApiDiagnostic>;
  onSelect: (file: WorkspaceSourceFile) => void;
  onFolderSelect: (root: string, folder: string) => void;
  onRename: (file: WorkspaceSourceFile) => void;
  collapseSignal: number;
  revealCreated: { path: string; root: string } | null;
  inlineCreation: { root: string; folder: string; category: Category; node: React.ReactNode } | null;
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
    if (collapseSignal === 0) return;
    const keys = new Set<string>();
    const visit = (nodes: SourceTreeNode[]) => {
      for (const node of nodes) if (node.kind === "folder") { keys.add(node.key); visit(node.children); }
    };
    for (const group of groups) visit(group.children);
    setCollapsed(keys);
  }, [collapseSignal]);

  useEffect(() => {
    if (!revealCreated) return;
    setCollapsed(current => new Set([...current].filter(key => key !== revealCreated.path && !revealCreated.path.startsWith(`${key}/`))));
    const frame = window.requestAnimationFrame(() => document.querySelector<HTMLElement>(`[data-tree-path="${CSS.escape(revealCreated.path)}"]`)?.focus());
    return () => window.cancelAnimationFrame(frame);
  }, [revealCreated, workspace]);

  useEffect(() => {
    if (!inlineCreation) return;
    const target = inlineCreation.folder ? `${inlineCreation.root}/${inlineCreation.folder}` : inlineCreation.root;
    setCollapsed(current => new Set([...current].filter(key => key !== target && !target.startsWith(`${key}/`))));
  }, [inlineCreation?.root, inlineCreation?.folder]);

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
          {expanded && <div role="group">
            {inlineCreation?.root === node.sourceRoot && inlineCreation.folder === (node.sourceRoot ? node.key.slice(node.sourceRoot.length + 1) : node.key) && inlineCreation.node}
            {node.children.map(renderNode)}
          </div>}
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
      <Dropdown key={node.key} menu={{ items: [{ key: "rename", label: "Rename or Move", onClick: () => onRename(node.file) }] }} trigger={["contextMenu"]}>
      <Button
        htmlType="button"
        role="treeitem"
        data-depth={node.depth}
        data-tree-path={node.file.path}
        aria-selected={activePath === node.file.path}
        aria-label={`${node.file.path}${stateLabels.length ? `, ${stateLabels.join(", ")}` : ""}`}
        className={`tree-file ${activePath === node.file.path ? "active" : ""}`}
        style={{ paddingLeft: `${10 + node.depth * 14}px` }}
        onClick={() => onSelect(node.file)}
        onKeyDown={(event) => {
          if (event.key === "F2") { event.preventDefault(); onRename(node.file); return; }
          if (event.key === "Enter") {
            event.preventDefault();
            onSelect(node.file);
            window.requestAnimationFrame(() => {
              const editor = document.querySelector<HTMLElement>(".editor-area");
              (editor?.querySelector<HTMLElement>("[data-cell]") ?? editor?.querySelector<HTMLElement>("button:not([disabled])") ?? editor)?.focus();
            });
            return;
          }
          handleTreeKey(event);
        }}
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
      </Dropdown>
    );
  };

  return <div className="source-tree" role="tree" aria-label="Project source files">
    {groups.map(({ root, children }) => {
      return (
        <div className="source-root" key={root}>
          <Button htmlType="button" role="treeitem" data-depth="0" className="tree-root-label"
            onFocus={() => onFolderSelect(root, "")} onClick={() => onFolderSelect(root, "")}
            onKeyDown={(event) => handleTreeKey(event)}
            aria-label={`${root || "."} source root`}>{root || "."}</Button>
          <div role="group" aria-label={`${root || "."} source files`}>
            {inlineCreation?.root === root && inlineCreation.folder === "" && inlineCreation.node}
            {children.map(renderNode)}
          </div>
        </div>
      );
    })}
    {workspace.files.length === 0 && !inlineCreation && <div className="pane-message">No YAML source documents.</div>}
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

// Fixed-height rows keep the scroll window independent of complex value shape.
// EVIDENCE: GUI-DATA-LAYOUT-007, GUI-DATA-VIRTUAL-001.
const GRID_ROW_HEIGHT = 32;
const GRID_OVERSCAN = 12;

type DataEditorUiState = {
  selectedRange: GridRange | null;
  batchText: string;
  querySearch: string;
  queryField: string;
  queryValue: string;
  querySortField: string;
  queryOperator: string;
  querySortDirection: string;
  queryAdvancedOpen: boolean;
  batchToolsOpen: boolean;
};

function ColumnHeader({ field, table, fieldTypes, fieldIndex, fieldCount, nextField, disabled, onIntent, onUndoSchema }: {
  field: TableField;
  table: string;
  fieldTypes: string[];
  fieldIndex: number;
  fieldCount: number;
  nextField?: string;
  disabled: boolean;
  onIntent: (intent: ColumnIntent) => Promise<void>;
  onUndoSchema: (redo: boolean) => void;
}) {
  const [name, setName] = useState(field.name);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const pointerCleanup = useRef<(() => void) | null>(null);
  useEffect(() => () => pointerCleanup.current?.(), []);
  useEffect(() => { pointerCleanup.current?.(); }, [disabled, fieldIndex, fieldCount, field, table]);
  const cancelBlur = useRef(false);
  useEffect(() => { setName(field.name); setError(null); }, [field.name]);
  const run = async (intent: ColumnIntent) => {
    if (busy || disabled) return;
    setBusy(true); setError(null);
    try { await onIntent(intent); }
    catch (cause) { setError(asApiError(cause).diagnostic.message); }
    finally { setBusy(false); }
  };
  const rename = () => {
    if (cancelBlur.current) { cancelBlur.current = false; return; }
    const next = name.trim();
    if (next && next !== field.name) void run({ operation: "rename", table, field: field.name, newName: next });
    else setName(field.name);
  };
  return <div className="unified-column-header"
    onContextMenu={event => { if ((event.target as HTMLElement).closest(".column-menu")) return; event.preventDefault(); event.currentTarget.querySelector<HTMLButtonElement>(".column-menu")?.click(); }}
    onKeyDown={event => { if (event.key !== "ContextMenu" && !(event.shiftKey && event.key === "F10")) return;
      event.preventDefault(); event.currentTarget.querySelector<HTMLButtonElement>(".column-menu")?.click(); }}
  >
    <div className="unified-column-actions">
      <button type="button" className="column-grab" aria-label={`Drag column ${field.name} to reorder`} disabled={disabled || busy}
        onPointerDown={event => {
          if (event.button != null && event.button !== 0) return;
          const root = event.currentTarget.closest("table");
          if (!root) return;
          pointerCleanup.current?.();
          pointerCleanup.current = startGridReorder(event, { root, axis: "x", sourceIndex: fieldIndex, itemCount: fieldCount,
            onDrop: destination => { void run({ operation: "reorder", table, field: field.name, newIndex: destination }); } });
        }}><GripVertical size={13} /></button>
      <Dropdown trigger={["click", "contextMenu"]} menu={{ items: [
        { key: "insert-left", label: "Insert column left", disabled, onClick: () => void run({ operation: "add_default", table, beforeField: field.name }) },
        { key: "insert-right", label: "Insert column right", disabled, onClick: () => void run({ operation: "add_default", table, beforeField: nextField }) },
        { type: "divider" },
        { key: "move-left", label: "Move column left", disabled: disabled || fieldIndex === 0, onClick: () => void run({ operation: "reorder", table, field: field.name, newIndex: fieldIndex - 1 }) },
        { key: "move-right", label: "Move column right", disabled: disabled || fieldIndex === fieldCount - 1, onClick: () => void run({ operation: "reorder", table, field: field.name, newIndex: fieldIndex + 1 }) },
      ] }}><Button type="text" size="small" className="column-menu" aria-label={`Actions for column ${field.name}`} icon={<MoreHorizontal size={13} />} /></Dropdown>
    </div>
    <input className="unified-column-name" data-column-field={field.name} aria-label={`Field name ${field.name}`} value={name} disabled={disabled || busy}
      onChange={event => setName(event.target.value)} onBlur={rename}
      onKeyDown={event => { if (event.key === "Enter") { event.preventDefault(); event.currentTarget.blur(); } else if (event.key === "Escape") { cancelBlur.current = true; setName(field.name); event.currentTarget.blur(); } }} />
    <div className="unified-column-type-row" onKeyDown={event => {
      if ((event.metaKey || event.ctrlKey) && (event.key.toLowerCase() === "z" || event.key.toLowerCase() === "y")) {
        event.preventDefault(); event.stopPropagation();
        onUndoSchema(event.shiftKey || event.key.toLowerCase() === "y");
      }
    }}>
      <Select size="small" aria-label={`Type of ${field.name}`} value={field.type} disabled={disabled || busy}
        options={fieldTypes.map(value => ({ value, label: value }))}
        onChange={type => void run({ operation: "change_declaration", table, field: field.name, type, nullable: field.nullable, array: field.array })} />
      <Button type="text" size="small" aria-label={`Nullable ${field.name}`} aria-pressed={field.nullable} disabled={disabled || busy}
        onClick={() => void run({ operation: "change_declaration", table, field: field.name, type: field.type, nullable: !field.nullable, array: false })}>?</Button>
      <Button type="text" size="small" aria-label={`Array ${field.name}`} aria-pressed={field.array} disabled={disabled || busy}
        onClick={() => void run({ operation: "change_declaration", table, field: field.name, type: field.type, nullable: false, array: !field.array })}>[]</Button>
    </div>
    {error && <span className="unified-column-error" role="alert" title={error}>{error}</span>}
  </div>;
}

function EmptyTableSurface({ context, disabled, onIntent, onUndoSchema, onCreateData, details, schemaDraftActions, saveEnabled, onSave }: {
  context: TableContext;
  disabled: boolean;
  onIntent: (intent: ColumnIntent) => Promise<void>;
  onUndoSchema: (redo: boolean) => void;
  onCreateData: () => void;
  details: React.ReactNode;
  schemaDraftActions: React.ReactNode;
  saveEnabled: boolean;
  onSave: () => void;
}) {
  const [error, setError] = useState<string | null>(null);
  const [detailsOpen, setDetailsOpen] = useState(false);
  return <section className="data-editor unified-empty-table">
    <header className="editor-tabs"><div className="document-tab"><strong>{context.table}</strong></div>
      <div className="editor-actions">{schemaDraftActions}<Button size="small" disabled={disabled || !saveEnabled} onClick={onSave}>Save</Button><Button type="text" size="small" onClick={() => setDetailsOpen(open => !open)}>Table details</Button></div></header>
    {error && <Alert type="error" title={error} closable onClose={() => setError(null)} />}
    {detailsOpen && details}
    <div className="grid-scroll"><table className="record-grid" role="grid" aria-rowcount={1} aria-colcount={context.schema.schema.fields.length + 2}>
      <thead><tr><th className="row-number">#</th>
        {context.schema.schema.fields.map((field, fieldIndex) => <th key={field.name} data-column-index={fieldIndex}><ColumnHeader field={field} table={context.table} fieldIndex={fieldIndex} fieldCount={context.schema.schema.fields.length} nextField={context.schema.schema.fields[fieldIndex + 1]?.name}
          fieldTypes={context.schema.fieldTypes} disabled={disabled} onIntent={onIntent} onUndoSchema={onUndoSchema} /></th>)}
        <th className="tag-column"><Button type="text" size="small" aria-label="Add column" disabled={disabled}
          onClick={() => void onIntent({ operation: "add_default", table: context.table }).catch(cause => setError(asApiError(cause).diagnostic.message))}>＋</Button></th>
      </tr></thead><tbody /></table>
      <div className="empty-grid"><p>No record source yet.</p><Button onClick={onCreateData} disabled={disabled}>Create record source</Button></div>
    </div>
  </section>;
}

function DataEditor({
  schemaDraftActions,
  contextDirty,
  contextSaving,
  mutationBlocked,
  schemaDraftBlocked,
  file,
  projectRoot,
  editor,
  schemaEditor,
  tableContext,
  onRecordSourceSelect,
  onColumnIntent,
  onUndoSchema,
  onOverview,
  onCreateData,
  uiCache,
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
  onInsertRow,
  onMoveRow,
  onDeleteExistingRow,
  onUndoExistingDelete,
  onDeleteDraftRow,
  onSave,
  onRecheckSource,
  onSwitchView,
  onReloadConflict,
  onOverwriteConflict,
}: {
  schemaDraftActions: React.ReactNode;
  contextDirty: boolean;
  contextSaving: boolean;
  mutationBlocked: boolean;
  schemaDraftBlocked: boolean;
  file: WorkspaceSourceFile;
  projectRoot: string;
  editor: EditorState;
  schemaEditor?: React.ReactNode;
  tableContext: TableContext | null;
  onRecordSourceSelect: (path: string) => void;
  onColumnIntent: (intent: ColumnIntent) => Promise<void>;
  onUndoSchema: (redo: boolean) => void;
  onOverview?: () => void;
  onCreateData?: () => void;
  uiCache: React.MutableRefObject<Map<string, DataEditorUiState>>;
  onCellChange: (recordIndex: number, field: string, value: AuthoringValue, operation?: boolean) => void;
  onDraftCellChange: (draftId: string, field: string, value: AuthoringValue, operation?: boolean) => void;
  onCellFocus: (key: string) => void;
  onTagsChange: (recordIndex: number, tags: string[]) => void;
  onDraftTagsChange: (draftId: string, tags: string[]) => void;
  onBatchApplied: (batch: AuthoringBatchPreview, expectedRevision: number) => void;
  onQueryResult: (result: DataFileQueryResult | null) => void;
  onUndo: () => void;
  onRedo: () => void;
  onAddRow: () => void;
  onInsertRow: (position: number) => void;
  onMoveRow: (row: EditorRowRef, position: number) => void;
  onDeleteExistingRow: (recordIndex: number) => void;
  onUndoExistingDelete: (recordIndex: number) => void;
  onDeleteDraftRow: (draftId: string) => void;
  onSave: () => void;
  onRecheckSource: () => void;
  onSwitchView: (view: EditorState["view"]) => void;
  onReloadConflict: () => void;
  onOverwriteConflict: () => void;
}) {
  const dirty = editorIsDirty(editor);
  const diagnostics = editor.previewState === "current" ? editor.preview.validation.diagnostics : [];
  const diagnosticsByCell = useMemo(() => {
    const result = new Map<string, Diagnostic[]>();
    if (editor.previewState !== "current" || editor.preview.validation.diagnostics.length === 0) return result;
    const pending = new Set(editor.pendingDeletes);
    const sourceOrder = (editor.rowOrder ?? defaultRowOrder(editor)).filter(row => row.kind === "added" || !pending.has(row.recordIndex));
    const source = normalizePath(`${projectRoot}/${file.path}`);
    for (const diagnostic of editor.preview.validation.diagnostics) {
      if (!diagnostic.source || normalizePath(diagnostic.source) !== source) continue;
      const field = diagnosticField(diagnostic);
      const index = diagnosticRecordIndex(diagnostic);
      if (!field || index === null) continue;
      const row = sourceOrder[index];
      const key = row?.kind === "existing" ? cellKey(row.recordIndex, field) : row?.kind === "added" ? draftCellKey(row.draftId, field) : null;
      if (!key) continue;
      const cellDiagnostics = result.get(key) ?? [];
      cellDiagnostics.push(diagnostic);
      result.set(key, cellDiagnostics);
    }
    return result;
  }, [editor.previewState, editor.preview.validation.diagnostics, editor.pendingDeletes, editor.snapshot.rows, editor.addedRecords, editor.rowOrder, projectRoot, file.path]);
  const uiKey = `${projectRoot}:${file.path}`;
  const rememberedUi = uiCache.current.get(uiKey);
  const lastFocusedCell = useRef<string | null>(null);
  const [selectedRange, setSelectedRange] = useState<GridRange | null>(rememberedUi?.selectedRange ?? null);
  const [editingCell, setEditingCell] = useState<{
    key: string; value: AuthoringValue; rowIndex: number; columnIndex: number; selectAll: boolean; baseIdentity: string;
    target: { kind: "existing"; recordIndex: number; field: string } | { kind: "added"; draftId: string; field: string };
  } | null>(null);
  const pendingGridFocus = useRef<string | null>(null);
  const pendingRowFocus = useRef<string | null>(null);
  const draggingSelection = useRef(false);
  const rowPointerCleanup = useRef<(() => void) | null>(null);
  useEffect(() => () => rowPointerCleanup.current?.(), []);
  const [batchText, setBatchText] = useState(rememberedUi?.batchText ?? "");
  const [batchPreview, setBatchPreview] = useState<AuthoringBatchPreview | null>(null);
  const [batchBusy, setBatchBusy] = useState(false);
  const [copyBusy, setCopyBusy] = useState(false);
  const [querySearch, setQuerySearch] = useState(rememberedUi?.querySearch ?? "");
  const [queryField, setQueryField] = useState(rememberedUi?.queryField ?? "");
  const [queryValue, setQueryValue] = useState(rememberedUi?.queryValue ?? "");
  const [querySortField, setQuerySortField] = useState(rememberedUi?.querySortField ?? "");
  const [queryBusy, setQueryBusy] = useState(false);
  const [queryError, setQueryError] = useState<ApiDiagnostic | null>(null);
  const [queryNotice, setQueryNotice] = useState<string | null>(null);
  const [queryOperator, setQueryOperator] = useState(rememberedUi?.queryOperator ?? "contains");
  const [querySortDirection, setQuerySortDirection] = useState(rememberedUi?.querySortDirection ?? "ascending");
  const [queryAdvancedOpen, setQueryAdvancedOpen] = useState(rememberedUi?.queryAdvancedOpen ?? false);
  const [batchToolsOpen, setBatchToolsOpen] = useState(rememberedUi?.batchToolsOpen ?? false);
  const [schemaOpen, setSchemaOpen] = useState(false);
  const filterButtonRef = useRef<HTMLButtonElement>(null);
  const moreButtonRef = useRef<HTMLButtonElement>(null);
  const gridScrollRef = useRef<HTMLDivElement>(null);
  const [gridScrollTop, setGridScrollTop] = useState(0);
  const [gridViewportHeight, setGridViewportHeight] = useState(640);
  useEffect(() => {
    // Query and batch drafts are file-local UI state; switching sources must not
    // leave an applied query showing controls that belong to another file.
    uiCache.current.set(uiKey, { selectedRange, batchText, querySearch, queryField, queryValue, querySortField, queryOperator, querySortDirection, queryAdvancedOpen, batchToolsOpen });
  }, [uiCache, uiKey, selectedRange, batchText, querySearch, queryField, queryValue, querySortField, queryOperator, querySortDirection, queryAdvancedOpen, batchToolsOpen]);
  const queryRequestSequence = useRef(0);
  const previousAddedCount = useRef(editor.addedRecords.length);
  const [batchContext, setBatchContext] = useState<{ revision: number; selectionKey: string } | null>(null);
  const capability = addCapability(editor.snapshot);
  const rowsByRecordIndex = useMemo(() => new Map(editor.snapshot.rows.map((row) => [row.recordIndex, row])), [editor.snapshot.rows]);
  const draftsById = useMemo(() => new Map(editor.addedRecords.map(draft => [draft.draftId, draft])), [editor.addedRecords]);
  const queryOrder = editor.queryResult?.orderedRecordIndices;
  const gridRows: GridRow[] = useMemo(() => {
    const pending = new Set(editor.pendingDeletes);
    const localOrder = editor.rowOrder ?? defaultRowOrder(editor);
    const candidateOrder = localOrder.filter(row => row.kind === "added" || !pending.has(row.recordIndex));
    const toGridRow = (row: EditorRowRef): GridRow[] => row.kind === "existing"
      ? rowsByRecordIndex.has(row.recordIndex) ? [{ kind: "existing", recordIndex: row.recordIndex, pendingDelete: pending.has(row.recordIndex) }] : []
      : draftsById.has(row.draftId) ? [{ kind: "added", draft: draftsById.get(row.draftId)! }] : [];
    return queryOrder
    ? queryOrder.flatMap<GridRow>(recordIndex => candidateOrder[recordIndex] ? toGridRow(candidateOrder[recordIndex]) : [])
    : localOrder.flatMap(toGridRow);
  }, [queryOrder, rowsByRecordIndex, draftsById, editor.snapshot.rows, editor.pendingDeletes, editor.addedRecords, editor.rowOrder]);
  const rowPositionEnabled = !mutationBlocked && !editor.saving && !editor.queryResult && editor.pendingDeletes.length === 0;
  useEffect(() => { rowPointerCleanup.current?.(); }, [editor.snapshot.baseContentIdentity, editor.rowOrder, gridRows, rowPositionEnabled, editor.view]);
  const rowPositionReason = editor.queryResult ? "Clear search, filter, or sort to edit source row positions." : editor.pendingDeletes.length ? "Undo or save pending deletions before moving rows." : "Row positions are unavailable while source changes are blocked.";
  const refForRow = (row: GridRow): EditorRowRef => row.kind === "existing"
    ? { kind: "existing", recordIndex: row.recordIndex }
    : { kind: "added", draftId: row.draft.draftId };
  const moveGridRow = (row: GridRow, destination: number) => {
    if (!rowPositionEnabled || destination < 0 || destination >= gridRows.length) return;
    pendingRowFocus.current = row.kind === "existing" ? `existing:${row.recordIndex}` : `added:${row.draft.draftId}`;
    revealGridRow(destination);
    setSelectedRange(null);
    onMoveRow(refForRow(row), destination);
  };
  const gridCellKeyAt = (rowIndex: number, columnIndex: number) => {
    const row = gridRows[rowIndex];
    const column = editor.snapshot.columns[columnIndex];
    if (!row || !column) return null;
    return row.kind === "existing" ? cellKey(row.recordIndex, column.name) : draftCellKey(row.draft.draftId, column.name);
  };
  const firstGridRow = Math.min(Math.max(0, gridRows.length - 1), Math.max(0, Math.floor(gridScrollTop / GRID_ROW_HEIGHT) - GRID_OVERSCAN));
  const lastGridRow = Math.min(gridRows.length, Math.ceil((gridScrollTop + gridViewportHeight) / GRID_ROW_HEIGHT) + GRID_OVERSCAN);
  const visibleGridRows = gridRows.slice(firstGridRow, lastGridRow).map((row, offset) => ({ row, rowIndex: firstGridRow + offset }));
  useLayoutEffect(() => {
    const element = gridScrollRef.current;
    if (!element) return;
    const measure = () => setGridViewportHeight(element.clientHeight || 640);
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [editor.view]);
  const revealGridRow = (rowIndex: number) => {
    const element = gridScrollRef.current;
    if (!element) return;
    const top = rowIndex * GRID_ROW_HEIGHT;
    const bottom = top + GRID_ROW_HEIGHT;
    const height = element.clientHeight || gridViewportHeight;
    if (top < element.scrollTop) element.scrollTop = top;
    else if (bottom > element.scrollTop + height) element.scrollTop = bottom - height;
    else return;
    setGridScrollTop(element.scrollTop);
  };
  const focusGridCell = (rowIndex: number, columnIndex: number) => {
    const key = gridCellKeyAt(rowIndex, columnIndex);
    if (!key) return;
    pendingGridFocus.current = key;
    revealGridRow(rowIndex);
    setSelectedRange({ startRow: rowIndex, startColumn: columnIndex, endRow: rowIndex, endColumn: columnIndex });
  };
  const finishCellEdit = (commit: boolean, nextRow?: number, nextColumn?: number, restoreFocus = true) => {
    if (!editingCell) return;
    const shape = editor.snapshot.columns[editingCell.columnIndex]?.shape;
    const complex = shape?.modifier === "array" || shape?.shape.kind === "custom" || shape?.shape.kind === "flags";
    if (commit && !complex && !mutationBlocked && !editor.saving && editor.snapshot.baseContentIdentity === editingCell.baseIdentity) {
      if (editingCell.target.kind === "existing") onCellChange(editingCell.target.recordIndex, editingCell.target.field, editingCell.value);
      else onDraftCellChange(editingCell.target.draftId, editingCell.target.field, editingCell.value);
    } else if (commit && !complex) setQueryNotice("Source changed or editing became unavailable. This cell change was not applied; reopen the cell to edit the current source.");
    const { rowIndex, columnIndex, key } = editingCell;
    setEditingCell(null);
    if (nextRow !== undefined && nextColumn !== undefined) focusGridCell(nextRow, nextColumn);
    else if (restoreFocus) pendingGridFocus.current = key;
  };
  const commitComplexOperation = (key: string, value: AuthoringValue) => {
    if (!editingCell || editingCell.key !== key || mutationBlocked || editor.saving) return;
    if (editor.snapshot.baseContentIdentity !== editingCell.baseIdentity) {
      setQueryNotice("Source changed. Reopen the cell to edit the current source.");
      return;
    }
    if (editingCell.target.kind === "existing") onCellChange(editingCell.target.recordIndex, editingCell.target.field, value, true);
    else onDraftCellChange(editingCell.target.draftId, editingCell.target.field, value, true);
  };
  useEffect(() => {
    if (!editingCell) return;
    const closeOutside = (event: MouseEvent) => {
      const target = event.target;
      if (!(target instanceof Element)) return;
      // Nested controls and their Ant Design portals remain part of this edit.
      if (target.closest(".complex-cell-popover, .ant-select-dropdown, .ant-dropdown")) return;
      if (target.closest(`[data-cell="${CSS.escape(editingCell.key)}"]`)) return;
      finishCellEdit(false, undefined, undefined, false);
    };
    // click runs after a nested text input's blur has committed its draft.
    document.addEventListener("click", closeOutside, true);
    return () => document.removeEventListener("click", closeOutside, true);
  }, [editingCell]);
  const handleGridScroll = (top: number) => {
    if (editingCell) {
      const first = Math.max(0, Math.floor(top / GRID_ROW_HEIGHT) - GRID_OVERSCAN);
      const last = Math.min(gridRows.length, Math.ceil((top + gridViewportHeight) / GRID_ROW_HEIGHT) + GRID_OVERSCAN);
      if (editingCell.rowIndex < first || editingCell.rowIndex >= last) finishCellEdit(true, undefined, undefined, false);
    }
    setGridScrollTop(top);
  };
  const beginCellEdit = (key: string, value: AuthoringValue, rowIndex: number, columnIndex: number, selectAll = false) => {
    const row = gridRows[rowIndex];
    const column = editor.snapshot.columns[columnIndex];
    if (!row || !column) return;
    // A query may reorder visible rows while a cell is open. Commit to its source identity.
    const target = row.kind === "existing" ? { kind: "existing" as const, recordIndex: row.recordIndex, field: column.name }
      : { kind: "added" as const, draftId: row.draft.draftId, field: column.name };
    setEditingCell({ key, value, rowIndex, columnIndex, selectAll, baseIdentity: editor.snapshot.baseContentIdentity, target });
  };
  useEffect(() => {
    const focusRequestedValue = (event: Event) => {
      const { cell, valuePath } = (event as CustomEvent<{ cell: string; valuePath: string }>).detail;
      const columnIndex = editor.snapshot.columns.findIndex((column) => {
        const index = gridRows.findIndex((row) => row.kind === "existing"
          ? cellKey(row.recordIndex, column.name) === cell
          : draftCellKey(row.draft.draftId, column.name) === cell);
        return index >= 0;
      });
      const rowIndex = columnIndex < 0 ? -1 : gridRows.findIndex((row) => {
        const field = editor.snapshot.columns[columnIndex].name;
        return row.kind === "existing" ? cellKey(row.recordIndex, field) === cell : draftCellKey(row.draft.draftId, field) === cell;
      });
      const row = gridRows[rowIndex];
      const column = editor.snapshot.columns[columnIndex];
      if (!row || !column || !column.shape) return;
      const value = row.kind === "existing" ? currentCellValue(editor, row.recordIndex, column.name) : row.draft.values[column.name] ?? nullAuthoringValue();
      revealGridRow(rowIndex);
      beginCellEdit(cell, value, rowIndex, columnIndex);
      window.requestAnimationFrame(() => window.requestAnimationFrame(() => {
        focusElement(document.querySelector<HTMLElement>(`[data-value-path="${CSS.escape(valuePath)}"]`));
      }));
    };
    document.addEventListener("masterdata:focus-value", focusRequestedValue);
    return () => document.removeEventListener("masterdata:focus-value", focusRequestedValue);
  }, [editor, gridRows]);
  const commitAndSave = () => {
    if (editingCell) {
      finishCellEdit(true, undefined, undefined, false);
      window.setTimeout(onSave, 0);
    } else onSave();
  };
  useLayoutEffect(() => {
    if (!editingCell) return;
    const focusInput = () => {
      const input = document.querySelector<HTMLElement>(`[data-edit-cell="${CSS.escape(editingCell.key)}"] input:not([disabled]), [data-edit-cell="${CSS.escape(editingCell.key)}"] button:not(.array-item-grab):not([disabled]), [data-edit-cell="${CSS.escape(editingCell.key)}"] [tabindex="0"]`);
      input?.focus();
      if (editingCell.selectAll && input instanceof HTMLInputElement) input.select();
      return Boolean(input);
    };
    if (focusInput()) return;
    // Ant Design mounts the popover portal after the cell commit. Focus when it appears.
    const observer = new MutationObserver(() => { if (focusInput()) observer.disconnect(); });
    observer.observe(document.body, { childList: true, subtree: true });
    return () => observer.disconnect();
  }, [editingCell?.key]);
  useLayoutEffect(() => {
    const key = pendingGridFocus.current;
    if (!key) return;
    const target = document.querySelector<HTMLElement>(`[data-cell="${CSS.escape(key)}"]`);
    if (target) { target.focus(); pendingGridFocus.current = null; }
  }, [editingCell, selectedRange, firstGridRow, lastGridRow]);
  useEffect(() => {
    const key = pendingRowFocus.current;
    if (!key || editor.previewState === "pending") return;
    const timer = window.setTimeout(() => {
      const target = document.querySelector<HTMLElement>(`[data-row-grab="${CSS.escape(key)}"]`);
      if (target) { target.focus({ preventScroll: true }); pendingRowFocus.current = null; }
    }, 0);
    return () => window.clearTimeout(timer);
  }, [editor.rowOrder, editor.previewState, firstGridRow, lastGridRow]);

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
  const latestPasteContext = useRef({ revision: editor.revision, selectionContextKey });
  latestPasteContext.current = { revision: editor.revision, selectionContextKey };
  const batchIsStale = batchPreview !== null
    && batchContext !== null
    && (batchContext.revision !== editor.revision || batchContext.selectionKey !== selectionContextKey);
  const selectedCellCount = selectedRange
    ? (Math.abs(selectedRange.endRow - selectedRange.startRow) + 1) * (Math.abs(selectedRange.endColumn - selectedRange.startColumn) + 1)
    : 1;

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

  const prepareBatch = async (fill: boolean, clipboardText: string) => {
    const targets = fill ? selectedTargets() : await pasteTargets(clipboardText);
    if (targets.length === 0) return null;
    return invoke<AuthoringBatchPreview>("preview_data_file_batch", {
      projectPath: projectRoot,
      relativePath: file.path,
      baseSource: editor.snapshot.baseSource,
      currentMutation: mutationForEditor(editor),
      request: { targets, clipboardText, fill },
    });
  };
  const previewBatch = async (fill: boolean, clipboardText = batchText) => {
    const requestSelectionKey = selectionContextKey;
    setBatchBusy(true);
    setQueryError(null);
    try {
      const result = await prepareBatch(fill, clipboardText);
      if (!result) return;
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
  const pasteClipboard = async () => {
    if (batchBusy || mutationBlocked || editor.saving) return;
    const requestRevision = editor.revision;
    const requestSelectionKey = selectionContextKey;
    setBatchBusy(true);
    setQueryError(null);
    try {
      const clipboardText = await navigator.clipboard.readText();
      const result = await prepareBatch(false, clipboardText);
      if (!result) return;
      if (latestPasteContext.current.revision !== requestRevision
        || latestPasteContext.current.selectionContextKey !== requestSelectionKey) {
        setQueryNotice("Paste was not applied because the buffer or selection changed. Paste again at the current cell.");
        return;
      }
      onBatchApplied(result, requestRevision);
    } catch (error) {
      setQueryError(asApiError(error).diagnostic);
    } finally {
      setBatchBusy(false);
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
      const newest = editor.addedRecords.at(-1);
      const rowIndex = gridRows.findIndex(row => row.kind === "added" && row.draft.draftId === newest?.draftId);
      if (rowIndex >= 0) {
        setSelectedRange({ startRow: rowIndex, endRow: rowIndex, startColumn: 0, endColumn: 0 });
        const field = editor.snapshot.columns[0]?.name;
        if (field) {
          const key = draftCellKey(newest!.draftId, field);
          pendingGridFocus.current = key;
          window.requestAnimationFrame(() => {
            // Let the new virtual row extend the scrollable height before revealing it.
            revealGridRow(rowIndex);
            window.requestAnimationFrame(() => {
              const target = document.querySelector<HTMLElement>(`[data-cell="${CSS.escape(key)}"]`);
              if (target) { target.focus({ preventScroll: true }); pendingGridFocus.current = null; }
            });
          });
        }
      }
      setQuerySearch("");
      setQueryField("");
      setQueryValue("");
      setQueryOperator("contains");
      setQuerySortField("");
      setQuerySortDirection("ascending");
      setQueryNotice(null);
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
          <strong>{tableContext?.table ?? editor.snapshot.table}</strong>
          {tableContext && tableContext.recordSources.length > 1 && <Select size="small" aria-label="Record set" value={file.path}
            options={tableContext.recordSources.map(source => ({ value: source.path, label: sourceName(source.path) }))}
            onChange={onRecordSourceSelect} />}
          {tableContext && tableContext.recordSources.length === 1 && <small className="record-source-label">{sourceName(file.path)}</small>}
          {dirty && <span className="tab-dirty">●</span>}
        </div>
        <Tabs className="view-tabs" size="small" activeKey={editor.view}
          onChange={(key) => onSwitchView(key as EditorState["view"])}
          items={[{ key: "grid", label: "Data" }, { key: "diff", label: "Diff" },
            ...(editor.conflict ? [{ key: "compare", label: "Conflict" }] : [])]} />
        <div className="editor-actions">
          {schemaDraftActions}
          {(editor.previewState !== "current" || !editor.preview.validation.valid) && <span className={`validation-state ${editor.previewState}`}>{validationLabel(editor)}</span>}
          <Button htmlType="button" onClick={commitAndSave} disabled={mutationBlocked || (!dirty && !contextDirty && !editingCell) || editor.saving || contextSaving || editor.saveStatus === "outcome_unknown" || schemaDraftBlocked}>{editor.saving || contextSaving ? "Saving…" : "Save"}</Button>
          <Dropdown trigger={["click"]} menu={{ items: [
            { key: "add", label: "Add Row", disabled: mutationBlocked || !capability.supported || editor.saving, onClick: onAddRow },
            { key: "undo", label: "Undo", disabled: mutationBlocked || editor.saving || editor.historyPast.length === 0, onClick: onUndo },
            { key: "redo", label: "Redo", disabled: mutationBlocked || editor.saving || editor.historyFuture.length === 0, onClick: onRedo },
            ...(schemaEditor ? [{ key: "schema", label: "Table details", onClick: () => setSchemaOpen(open => !open) }] : []),
            ...(onOverview ? [{ key: "overview", label: "Table Overview", onClick: onOverview }] : []),
            ...(onCreateData ? [{ key: "data", label: "New data file", onClick: onCreateData }] : []),
            { key: "batch", label: "Batch tools", onClick: () => { setQueryAdvancedOpen(false); setBatchToolsOpen(open => !open); } },
          ] }}>
            <Button ref={moreButtonRef} htmlType="button" aria-label="More data actions" icon={<MoreHorizontal size={16} />} />
          </Dropdown>
        </div>
      </header>

      {schemaEditor && schemaOpen && <section id="data-table-structure" className="data-table-structure" aria-label="Table structure">{schemaEditor}</section>}

      <div className="authoring-tools-shell">
      <div className="authoring-toolbar" aria-label="Data authoring tools">
        <label htmlFor="data-search-input" className="visually-hidden">Data search</label>
        <Input.Search id="data-search-input" placeholder="Search current buffer" value={querySearch} onChange={(event) => setQuerySearch(event.target.value)} onSearch={() => void runQuery()} loading={queryBusy}
          enterButton={<Button aria-label="Search data" icon={<Search size={14} aria-hidden="true" />} />} />
        <Button ref={filterButtonRef} htmlType="button" aria-expanded={queryAdvancedOpen} aria-controls="data-query-options" onClick={() => { setBatchToolsOpen(false); setQueryAdvancedOpen((open) => !open); }}>Filter &amp; sort{queryField || querySortField ? " · set" : ""} <ChevronDown size={13} aria-hidden="true" /></Button>
        {editor.queryResult && <span className="query-result">{editor.queryResult.displayedCount} / {editor.queryResult.totalCount} rows</span>}
      </div>
      <div className="authoring-toolbar authoring-options" id="data-query-options" hidden={!queryAdvancedOpen} aria-label="Filter and sort options" onKeyDown={event => {
        if (event.key === "Escape") { event.stopPropagation(); setQueryAdvancedOpen(false); filterButtonRef.current?.focus(); }
      }}>
        <Select aria-label="Data filter field" allowClear placeholder="Filter field" value={queryField || undefined} onChange={(value) => setQueryField(value ?? "")} options={editor.snapshot.columns.map((column) => ({ value: column.name, label: column.name }))} />
        <Select aria-label="Data filter operator" value={queryOperator} onChange={setQueryOperator} options={[{ value: "contains", label: "contains" }, { value: "equals", label: "equals" }, { value: "not-equals", label: "not equals" }, { value: "less-than", label: "<" }, { value: "greater-than", label: ">" }, { value: "is-null", label: "is null" }, { value: "is-invalid", label: "is invalid" }]} />
        <Input aria-label="Data filter value" placeholder="Filter contains" value={queryValue} onChange={(event) => setQueryValue(event.target.value)} />
        <Select aria-label="Data sort field" allowClear placeholder="Sort by" value={querySortField || undefined} onChange={(value) => setQuerySortField(value ?? "")} options={editor.snapshot.columns.map((column) => ({ value: column.name, label: column.name }))} />
        <Select aria-label="Data sort direction" value={querySortDirection} onChange={setQuerySortDirection} options={[{ value: "ascending", label: "A→Z" }, { value: "descending", label: "Z→A" }]} />
        <Button htmlType="button" onClick={() => void runQuery()} loading={queryBusy}>Apply filter &amp; sort</Button>
      </div>
      <div className="authoring-toolbar authoring-options" id="data-batch-tools" hidden={!batchToolsOpen} aria-label="Batch tools" onKeyDown={event => {
        if (event.key === "Escape") { event.stopPropagation(); setBatchToolsOpen(false); moreButtonRef.current?.focus(); }
      }}>
        <Input.TextArea aria-label="Clipboard TSV" rows={1} placeholder="Paste TSV for the selected scalar range" value={batchText} onChange={(event) => setBatchText(event.target.value)} />
        <Button htmlType="button" onClick={() => void previewBatch(false)} disabled={batchBusy}>Paste preview</Button>
        <Button htmlType="button" onClick={() => void previewBatch(true)} disabled={batchBusy}>Fill preview</Button>
        <Button htmlType="button" onClick={() => void copySelection()} loading={copyBusy} disabled={batchBusy || copyBusy}>Copy selection</Button>
        <Button htmlType="button" onClick={() => void readClipboardAndPreview()}>Read clipboard & preview</Button>
      </div>
      </div>
      <div className={`selection-status ${!batchIsStale && selectedCellCount <= 1 ? "visually-hidden" : ""}`} role="status" aria-live="polite">
        {selectedRange ? `${selectedCellCount} cells selected` : "One cell active"}
        {batchIsStale && " · batch preview expired; review again"}
      </div>
      {(queryNotice || queryError) && <div className="editor-feedback">
        {queryNotice && <Alert type="info" showIcon closable onClose={() => setQueryNotice(null)} title={queryNotice} />}
        {queryError && <Alert type="error" showIcon title={queryError.code} description={queryError.message} />}
      </div>}

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
        <div className="grid-scroll" ref={gridScrollRef} onScroll={(event) => handleGridScroll(event.currentTarget.scrollTop)}
        >
          <table className="record-grid" role="grid" aria-rowcount={gridRows.length + 1} aria-colcount={editor.snapshot.columns.length + 2}>
            <thead>
              <tr>
                <th className="row-number">#</th>
                {editor.snapshot.columns.map((column, fieldIndex) => (
                  <th key={column.name} data-column-index={fieldIndex}>
                    {tableContext?.schema.schema.fields.find(field => field.name === column.name)
                      ? <ColumnHeader field={tableContext.schema.schema.fields.find(field => field.name === column.name)!}
                          table={tableContext.table} fieldTypes={tableContext.schema.fieldTypes} fieldIndex={fieldIndex} fieldCount={editor.snapshot.columns.length} nextField={editor.snapshot.columns[fieldIndex + 1]?.name}
                          disabled={mutationBlocked || schemaDraftBlocked} onIntent={onColumnIntent} onUndoSchema={onUndoSchema} />
                      : <div className="column-heading"><strong>{column.name}</strong><span>{column.typeName}</span></div>}
                  </th>
                ))}
                <th className="tag-column"><span>$tags</span>{tableContext && <Button type="text" size="small" aria-label="Add column" disabled={mutationBlocked}
                  onClick={() => void onColumnIntent({ operation: "add_default", table: tableContext.table }).catch(error => setQueryError(asApiError(error).diagnostic))}>＋</Button>}</th>
              </tr>
            </thead>
            <tbody>
              {firstGridRow > 0 && <tr className="virtual-spacer" role="presentation" aria-hidden="true"><td colSpan={editor.snapshot.columns.length + 2} style={{ height: firstGridRow * GRID_ROW_HEIGHT }} /></tr>}
              {visibleGridRows.map(({ row: gridRow, rowIndex: gridRowIndex }) => (
                <tr
                  key={gridRow.kind === "existing" ? `record-${gridRow.recordIndex}` : gridRow.draft.draftId}
                  aria-rowindex={gridRowIndex + 2}
                  data-grid-row-index={gridRowIndex}
                  className={gridRow.kind === "existing" && gridRow.pendingDelete ? "pending-delete" : ""}
                >
                  <th className="row-number" onContextMenu={event => { if ((event.target as HTMLElement).closest(".row-menu")) return;
                    event.preventDefault(); event.currentTarget.querySelector<HTMLButtonElement>(".row-menu")?.click(); }}
                    onKeyDown={event => { if (event.key !== "ContextMenu" && !(event.shiftKey && event.key === "F10")) return;
                      event.preventDefault(); event.currentTarget.querySelector<HTMLButtonElement>(".row-menu")?.click(); }}>
                    <div className="row-number-content">
                    <button className="row-grab" type="button" data-row-grab={gridRow.kind === "existing" ? `existing:${gridRow.recordIndex}` : `added:${gridRow.draft.draftId}`} aria-label={`Drag row ${gridRowIndex + 1} to reorder`} title={!rowPositionEnabled ? rowPositionReason : "Drag to reorder"}
                      disabled={!rowPositionEnabled}
                      onPointerDown={event => {
                        if (event.button != null && event.button !== 0) return;
                        const root = event.currentTarget.closest("table");
                        if (!root) return;
                        rowPointerCleanup.current?.();
                        rowPointerCleanup.current = startGridReorder(event, { root, axis: "y", sourceIndex: gridRowIndex,
                          itemCount: gridRows.length, rowHeight: GRID_ROW_HEIGHT, onDrop: destination => moveGridRow(gridRow, destination) });
                      }}><GripVertical size={12} /></button>
                    <span title={gridRow.kind === "existing" && gridRow.pendingDelete ? "Pending delete" : gridRow.kind === "added" ? "New draft" : `Source occurrence ${gridRow.recordIndex + 1}`}>{gridRow.kind === "added" ? "+" : gridRowIndex + 1}</span>
                    <Dropdown menu={{ items: [
                      { key: "insert-above", label: "Insert row above", disabled: !rowPositionEnabled || !capability.supported, onClick: () => onInsertRow(gridRowIndex) },
                      { key: "insert-below", label: "Insert row below", disabled: !rowPositionEnabled || !capability.supported, onClick: () => onInsertRow(gridRowIndex + 1) },
                      { type: "divider" },
                      { key: "move-up", label: "Move row up", disabled: !rowPositionEnabled || gridRowIndex === 0, onClick: () => moveGridRow(gridRow, gridRowIndex - 1) },
                      { key: "move-down", label: "Move row down", disabled: !rowPositionEnabled || gridRowIndex === gridRows.length - 1, onClick: () => moveGridRow(gridRow, gridRowIndex + 1) },
                      { type: "divider" },
                      gridRow.kind === "existing"
                        ? { key: "delete", label: gridRow.pendingDelete ? "Undo Delete" : "Delete Record", danger: !gridRow.pendingDelete, disabled: mutationBlocked || editor.saving, onClick: () => gridRow.pendingDelete ? onUndoExistingDelete(gridRow.recordIndex) : onDeleteExistingRow(gridRow.recordIndex) }
                        : { key: "delete", label: "Delete New Row", danger: true, disabled: mutationBlocked || editor.saving, onClick: () => onDeleteDraftRow(gridRow.draft.draftId) },
                    ] }} trigger={["click", "contextMenu"]}>
                      <Button type="text" size="small" className="row-menu" aria-label={`Actions for record ${gridRowIndex + 1}`} icon={<MoreHorizontal size={13} />} />
                    </Dropdown>
                    </div>
                  </th>
                  {editor.snapshot.columns.map((column, columnIndex) => {
                    const key = gridCellKeyAt(gridRowIndex, columnIndex)!;
                    const snapshotCell = gridRow.kind === "existing"
                      ? rowsByRecordIndex.get(gridRow.recordIndex)
                        ?.cells.find((cell) => cell.field === column.name)
                      : undefined;
                    const value = gridRow.kind === "existing"
                      ? editor.edits[key]?.value ?? snapshotCell?.value ?? nullAuthoringValue()
                      : gridRow.draft.values[column.name] ?? nullAuthoringValue();
                    const cellDiagnostics = diagnosticsByCell.get(key) ?? [];
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
                      : snapshotCell?.readOnlyReason ?? column.readOnlyReason ?? undefined;
                    const complex = column.shape?.modifier === "array" || column.shape?.shape.kind === "custom" || column.shape?.shape.kind === "flags";
                    const isEditing = editingCell?.key === key;
                    const label = `${gridRow.kind === "added" ? "new record" : `record ${gridRow.recordIndex + 1}`} ${column.name}`;
                    return (
                      <td key={column.name} data-column-index={columnIndex} className={`${changed ? "changed" : ""} ${hasDiagnostic ? "invalid" : ""} ${isSelected ? "selected" : ""}`}>
                        <Popover open={Boolean(isEditing && complex)} trigger={["click"]} onOpenChange={(open) => { if (!open && isEditing) finishCellEdit(false, undefined, undefined, false); }}
                          afterOpenChange={(open) => {
                            if (!open || !isEditing) return;
                            const container = document.querySelector<HTMLElement>(`[data-edit-cell="${CSS.escape(key)}"]`);
                            if (container?.contains(document.activeElement)) return;
                            focusElement(container);
                          }} placement="bottomLeft" overlayClassName="complex-cell-popover"
                          content={isEditing && column.shape ? <div data-edit-cell={key} onKeyDown={(event) => { if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") { event.preventDefault(); event.stopPropagation(); commitAndSave(); } else if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); finishCellEdit(false); } }}>
                            <ValueEditor field={column.shape} value={value} label={label} cellKey={key} editable={editable} invalidPaths={invalidPaths}
                              bufferedText onChange={(next) => commitComplexOperation(key, next)} />
                            <div className="complex-cell-actions"><Button size="small" onClick={() => finishCellEdit(false)}>Close</Button></div>
                          </div> : null}>
                        <div
                          className="cell-wrap"
                          data-cell={key}
                          data-edit-cell={isEditing && !complex ? key : undefined}
                          tabIndex={selectedRange
                            ? (selectedRange.endRow === gridRowIndex && selectedRange.endColumn === columnIndex ? 0 : -1)
                            : (gridRowIndex === 0 && columnIndex === 0 ? 0 : -1)}
                          role="gridcell"
                          aria-label={`${label}: ${authoringValueSummary(value)}${readOnlyReason ? `, ${readOnlyReason}` : ""}`}
                          onMouseDown={(event) => {
                            if (event.button !== 0) return;
                            if (editingCell && editingCell.key !== key) {
                              const previousShape = editor.snapshot.columns[editingCell.columnIndex]?.shape;
                              const previousComplex = previousShape?.modifier === "array" || previousShape?.shape.kind === "custom" || previousShape?.shape.kind === "flags";
                              finishCellEdit(!previousComplex, undefined, undefined, false);
                            }
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
                          onDoubleClick={() => { if (editable) beginCellEdit(key, value, gridRowIndex, columnIndex); }}
                          onFocusCapture={() => {
                            onCellFocus(key);
                            lastFocusedCell.current = key;
                          }}
                          onBlurCapture={(event) => {
                            if (!isEditing || complex) return;
                            if (event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget)) return;
                            finishCellEdit(true, undefined, undefined, false);
                          }}
                          onKeyDown={(event) => {
                            if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s" && isEditing) { event.preventDefault(); event.stopPropagation(); commitAndSave(); return; }
                            const input = event.target instanceof HTMLInputElement ? event.target : null;
                            const textSelectionActive = input?.selectionStart != null
                              && input.selectionEnd != null
                              && input.selectionStart !== input.selectionEnd;
                            if (!isEditing && (event.metaKey || event.ctrlKey)
                              && !event.altKey
                              && !textSelectionActive
                              && (event.key.toLowerCase() === "v" || event.key.toLowerCase() === "c")) {
                              event.preventDefault();
                              if (event.key.toLowerCase() === "c") void copySelection();
                              else void pasteClipboard();
                              return;
                            }
                            if (isEditing) {
                              if (event.key === "Escape") { event.preventDefault(); finishCellEdit(false); return; }
                              if (!complex && (event.key === "Enter" || event.key === "Tab")) {
                                event.preventDefault();
                                const nextRow = event.key === "Enter" ? gridRowIndex + (event.shiftKey ? -1 : 1) : gridRowIndex;
                                const nextColumn = event.key === "Tab" ? columnIndex + (event.shiftKey ? -1 : 1) : columnIndex;
                                finishCellEdit(true, Math.max(0, Math.min(gridRows.length - 1, nextRow)), Math.max(0, Math.min(editor.snapshot.columns.length - 1, nextColumn)));
                              }
                              return;
                            }
                            if ((event.key === "Enter" || event.key === "F2") && editable) { event.preventDefault(); beginCellEdit(key, value, gridRowIndex, columnIndex, event.key === "Enter"); return; }
                            if (event.key === "Escape" && selectedRange) { event.preventDefault(); setSelectedRange({ startRow: gridRowIndex, startColumn: columnIndex, endRow: gridRowIndex, endColumn: columnIndex }); return; }
                            const nextRow = gridRowIndex + (event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0);
                            const nextColumn = columnIndex + (event.key === "ArrowRight" || event.key === "Tab" && !event.shiftKey ? 1 : event.key === "ArrowLeft" || event.key === "Tab" && event.shiftKey ? -1 : 0);
                            if (nextRow !== gridRowIndex || nextColumn !== columnIndex) {
                              event.preventDefault();
                              const row = Math.max(0, Math.min(gridRows.length - 1, nextRow));
                              const col = Math.max(0, Math.min(editor.snapshot.columns.length - 1, nextColumn));
                              if (event.shiftKey && event.key.startsWith("Arrow")) setSelectedRange((current) => ({ startRow: current?.startRow ?? gridRowIndex, startColumn: current?.startColumn ?? columnIndex, endRow: row, endColumn: col }));
                              else focusGridCell(row, col);
                            }
                          }}
                        >
                          {isEditing && !complex && column.shape ? (
                            <ValueEditor
                              field={column.shape}
                              value={editingCell.value}
                              label={label}
                              cellKey={key}
                              editable={editable}
                              invalidPaths={invalidPaths}
                              onChange={(next) => setEditingCell((current) => current?.key === key ? { ...current, value: next } : current)}
                              onOperation={(next) => { commitComplexOperation(key, next); finishCellEdit(false); }}
                            />
                          ) : (
                            <output className="cell-summary" data-value-path={key} title={readOnlyReason ?? authoringValueSummary(value)}>{authoringValueSummary(value)}</output>
                          )}
                          {hasDiagnostic && <span className="cell-error" title="Validation diagnostic">!</span>}
                        </div>
                        </Popover>
                      </td>
                    );
                  })}
                  <td className="tag-cell">
                    {(() => {
                      const snapshotRow = gridRow.kind === "existing" ? rowsByRecordIndex.get(gridRow.recordIndex) : undefined;
                      const tags = gridRow.kind === "existing"
                        ? editor.tagEdits[String(gridRow.recordIndex)] ?? snapshotRow?.tags ?? []
                        : gridRow.draft.tags;
                      const editable = !mutationBlocked && !editor.saving && (gridRow.kind === "added"
                        ? true
                        : snapshotRow?.tagsEditable !== false && !gridRow.pendingDelete);
                      return <Popover trigger="click" placement="bottomLeft" overlayClassName="tags-cell-popover" content={<RecordTagEditor
                        tags={tags}
                        suggestions={editor.snapshot.tagCandidates ?? []}
                        suggestionsComplete={editor.snapshot.tagCandidatesComplete !== false}
                        editable={editable}
                        reason={snapshotRow?.tagsReadOnlyReason ?? undefined}
                        label={`${gridRow.kind === "added" ? "new record" : `record ${gridRow.recordIndex + 1}`} tags`}
                        onChange={(next) => gridRow.kind === "added"
                          ? onDraftTagsChange(gridRow.draft.draftId, next)
                          : onTagsChange(gridRow.recordIndex, next)}
                      />}><Button type="text" size="small" className="tag-summary" aria-label={`Edit tags for ${gridRow.kind === "added" ? "new record" : `record ${gridRow.recordIndex + 1}`}`}>{tags.length ? tags.join(", ") : "—"}</Button></Popover>;
                    })()}
                  </td>
                </tr>
              ))}
              {lastGridRow < gridRows.length && <tr className="virtual-spacer" role="presentation" aria-hidden="true"><td colSpan={editor.snapshot.columns.length + 2} style={{ height: (gridRows.length - lastGridRow) * GRID_ROW_HEIGHT }} /></tr>}
            </tbody>
          </table>
          <Button className="add-record-inline" htmlType="button" onClick={onAddRow} disabled={mutationBlocked || !capability.supported || editor.saving}>＋ Add Row</Button>
          {!capability.supported && <div className="add-row-reason" role="status">
            <strong>Add Row unavailable</strong>
            <span>{capability.reason ?? "This Table is outside the initial Required Primitive scope."}</span>
          </div>}
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
          if (batchPreview && batchContext && !batchIsStale) onBatchApplied(batchPreview, batchContext.revision);
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
