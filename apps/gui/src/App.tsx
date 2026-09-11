import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Alert, Button, Empty, Input, Modal, Tabs } from "antd";
import { Database, FolderOpen, Save, RotateCw, ShieldCheck, Play } from "lucide-react";
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
};

type Diagnostic = {
  code: string;
  kind: string;
  message: string;
  source?: string | null;
  line?: number | null;
  column?: number | null;
  schema_path?: string | null;
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
  capabilities: AuthoringCapabilities;
};

type DataEditorColumn = {
  name: string;
  typeName: string;
  editable: boolean;
  keyField: boolean;
};

type DataEditorCell = {
  field: string;
  text: string;
  editable: boolean;
};

type DataEditorRow = {
  recordIndex: number;
  cells: DataEditorCell[];
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
  validation: ValidationReport;
};

type AuthoringEdit = {
  recordIndex: number;
  field: string;
  value: string;
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
};

type WorkspaceState =
  | { kind: "loading"; previous: AuthoringWorkspace | null }
  | { kind: "ready"; workspace: AuthoringWorkspace }
  | { kind: "error"; diagnostic: ApiDiagnostic; previous: AuthoringWorkspace | null };

type OperationState<T> =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "done"; value: T }
  | { kind: "error"; diagnostic: ApiDiagnostic };

type EditorState = {
  snapshot: DataFileSnapshot;
  edits: Record<string, AuthoringEdit>;
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

type PendingAction =
  | { kind: "reload" }
  | { kind: "open"; projectPath: string }
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

function baseCellText(snapshot: DataFileSnapshot, recordIndex: number, field: string): string {
  return snapshot.rows
    .find((row) => row.recordIndex === recordIndex)
    ?.cells.find((cell) => cell.field === field)?.text ?? "";
}

function currentCellText(editor: EditorState, recordIndex: number, field: string): string {
  return editor.edits[cellKey(recordIndex, field)]?.value ?? baseCellText(editor.snapshot, recordIndex, field);
}

function editorIsDirty(editor: EditorState): boolean {
  return Object.keys(editor.edits).length > 0;
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

function App() {
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
  const cellFocusStart = useRef(new Map<string, string>());
  const pendingCellFocus = useRef<string | null>(null);

  useEffect(() => {
    editorsRef.current = editors;
  }, [editors]);

  useEffect(() => {
    workspaceStateRef.current = workspaceState;
  }, [workspaceState]);

  useEffect(() => {
    const key = pendingCellFocus.current;
    if (!key || !activePath || loadingPaths.has(activePath)) return;
    const cell = document.querySelector<HTMLInputElement>(`[data-cell="${CSS.escape(key)}"]`);
    if (cell) {
      cell.focus();
      pendingCellFocus.current = null;
    }
  }, [activePath, editors, loadingPaths]);

  const workspace = workspaceState.kind === "ready"
    ? workspaceState.workspace
    : workspaceState.kind === "error"
      ? workspaceState.previous
      : null;
  const projectRoot = workspace?.project.project_root ?? null;
  const activeFile = workspace?.files.find((file) => file.path === activePath) ?? null;
  const activeEditor = activePath ? editors[activePath] ?? null : null;
  const activeLoading = activePath ? loadingPaths.has(activePath) : false;
  const activeLoadDiagnostic = activeEditor && editorIsDirty(activeEditor)
    ? null
    : activeEditor?.loadError ?? (activePath ? fileOpenErrors[activePath] ?? null : null);
  const dirtyCount = Object.values(editors).filter(editorIsDirty).length;

  const showNotice = useCallback((message: string) => {
    setNotice(message);
    window.setTimeout(() => setNotice(null), 2600);
  }, []);

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
    for (const timer of previewTimers.current.values()) window.clearTimeout(timer);
    previewTimers.current.clear();
    setLoadingPaths(new Set());
    setFileOpenErrors({});
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
      if (workspaceGeneration.current !== generation) return;
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
  }, [openDataFile]);

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
          edits: Object.values(editor.edits),
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

  const updateCell = useCallback((path: string, recordIndex: number, field: string, value: string) => {
    if (!projectRoot) return;
    setEditors((current) => {
      const editor = current[path];
      if (!editor || editor.saving) return current;
      const nextEdits = { ...editor.edits };
      const key = cellKey(recordIndex, field);
      if (value === baseCellText(editor.snapshot, recordIndex, field)) {
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
      };
      schedulePreview(projectRoot, path, next);
      return { ...current, [path]: next };
    });
  }, [projectRoot, schedulePreview]);

  const saveFile = useCallback(async (path: string, overwriteExpectedIdentity?: string): Promise<boolean> => {
    const state = workspaceStateRef.current;
    const root = state.kind === "ready" ? state.workspace.project.project_root : state.previous?.project.project_root;
    const editor = editorsRef.current[path];
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
        edits: Object.values(editor.edits),
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
  }, [showNotice]);

  const saveAll = useCallback(async (): Promise<boolean> => {
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
  }, [saveFile]);

  const performAction = useCallback(async (action: PendingAction) => {
    setPendingAction(null);
    if (action.kind === "close") {
      await getCurrentWindow().destroy();
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
    const hasDirty = Object.values(editorsRef.current).some(editorIsDirty);
    if (hasDirty) {
      setPendingAction(action);
    } else {
      void performAction(action);
    }
  }, [performAction]);

  useEffect(() => {
    const listener = getCurrentWindow().onCloseRequested((event) => {
      if (Object.values(editorsRef.current).some(editorIsDirty)) {
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
        if (activePath) void saveFile(activePath);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [activePath, saveFile]);

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
    if (!workspace?.capabilities.build || !projectRoot || buildState.kind === "loading") return;
    if (dirtyCount > 0) {
      showNotice("Build uses saved source only; unsaved changes are not included.");
    }
    const generation = workspaceGeneration.current;
    setBuildState({ kind: "loading" });
    try {
      const response = await invoke<BuildResponse>("build", {
        projectPath: projectRoot,
        dryRun: false,
      });
      if (workspaceGeneration.current !== generation) return;
      setBuildState({ kind: "done", value: response });
      showNotice("Build complete");
    } catch (error) {
      if (workspaceGeneration.current !== generation) return;
      setBuildState({ kind: "error", diagnostic: asApiError(error).diagnostic });
      setProblemsOpen(true);
    }
  }, [buildState.kind, dirtyCount, projectRoot, showNotice, workspace]);

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
      pendingCellFocus.current = cellKey(record, field);
    }
    selectFile(file);
    if (pendingCellFocus.current) {
      window.requestAnimationFrame(() => {
        const key = pendingCellFocus.current;
        if (!key) return;
        const cell = document.querySelector<HTMLInputElement>(`[data-cell="${CSS.escape(key)}"]`);
        if (cell) {
          cell.focus();
          pendingCellFocus.current = null;
        }
      });
    }
  }, [selectFile, workspace]);

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
          <Button htmlType="button" icon={<RotateCw size={15} />} onClick={() => requestAction({ kind: "reload" })} disabled={!workspace}>Reload</Button>
          <Button htmlType="button" icon={<Save size={15} />} onClick={() => activePath && void saveFile(activePath)} disabled={!activeEditor || !editorIsDirty(activeEditor) || activeEditor.saving || activeEditor.saveStatus === "outcome_unknown" || !workspace?.capabilities.workspaceWrite}>
            {activeEditor?.saving ? "Saving…" : "Save"}
          </Button>
          <Button htmlType="button" icon={<ShieldCheck size={15} />} onClick={() => void validateDisk()} disabled={!workspace?.capabilities.validate || manualValidation.kind === "loading"}>
            {manualValidation.kind === "loading" ? "Validating…" : "Validate"}
          </Button>
          <Button type="primary" htmlType="button" icon={<Play size={15} />} onClick={() => void runBuild()} disabled={!workspace?.capabilities.build || buildState.kind === "loading"}>
            {buildState.kind === "loading" ? "Building…" : "Build"}
          </Button>
        </div>
      </header>

      {dirtyCount > 0 && (
        <div className="unsaved-build-note">
          {dirtyCount} unsaved file{dirtyCount === 1 ? "" : "s"}. Build uses saved source only.
        </div>
      )}

      <section className="workspace-layout">
        <aside className="explorer" aria-label="Workspace Explorer">
          <div className="pane-heading">
            <span>EXPLORER</span>
            <small>{workspace?.project.project_id ?? "project not open"}</small>
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
              onSelect={selectFile}
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
          {workspace && !activeFile && (
            <EmptyEditor title="Select a source file" copy="Choose a YAML document from the Workspace Explorer." />
          )}
          {activeFile && activeFile.kind !== "data" && (
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
              file={activeFile}
              projectRoot={projectRoot!}
              editor={activeEditor}
              onCellChange={(recordIndex, field, value) => updateCell(activeFile.path, recordIndex, field, value)}
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
                  <div className="build-result-line">Build complete · {buildState.value.generatedFiles.length} generated C# files · saved sources only</div>
                )}
              </div>
            )}
          </section>
        </section>
      </section>

      <footer className="statusbar">
        <span>{activePath ?? "No source selected"}</span>
        <span>{activeEditor ? `${activeEditor.snapshot.rows.length} records · ${activeEditor.snapshot.columns.length} fields` : ""}</span>
        <span>{dirtyCount > 0 ? `${dirtyCount} dirty` : "Saved"}</span>
      </footer>

      <Modal open={pendingAction !== null} title="Save changes before continuing?"
        closable={!pendingActionBusy} keyboard={!pendingActionBusy} mask={{ closable: false }}
        onCancel={() => !pendingActionBusy && setPendingAction(null)}
        footer={[
          <Button key="cancel" disabled={pendingActionBusy} onClick={() => setPendingAction(null)}>Cancel</Button>,
          <Button key="discard" disabled={pendingActionBusy} onClick={() => pendingAction && void performAction(pendingAction)}>Don't Save</Button>,
          <Button key="save" type="primary" loading={pendingActionBusy} onClick={async () => {
            if (!pendingAction) return;
            setPendingActionBusy(true);
            const action = pendingAction;
            const ok = await saveAll();
            setPendingActionBusy(false);
            if (ok) await performAction(action);
          }}>Save All</Button>,
        ]}>
        <p>{dirtyCount} source file{dirtyCount === 1 ? " has" : "s have"} unsaved changes. The requested action would discard the current buffers.</p>
      </Modal>

      {notice && <Alert className="toast" title={notice} type="info" showIcon role="status" />}
    </main>
  );
}

type SourceTreeFolder = {
  kind: "folder";
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
}: {
  workspace: AuthoringWorkspace;
  activePath: string | null;
  editors: Record<string, EditorState>;
  loadingPaths: Set<string>;
  fileOpenErrors: Record<string, ApiDiagnostic>;
  onSelect: (file: WorkspaceSourceFile) => void;
}) {
  const [collapsed, setCollapsed] = useState<Set<string>>(() => new Set());
  const groups = useMemo(() => workspace.sourceRoots.map((root) => {
    const children: SourceTreeNode[] = [];
    for (const file of workspace.files.filter((candidate) => candidate.sourceRoot === root)) {
      const relative = file.path.startsWith(`${root}/`) ? file.path.slice(root.length + 1) : file.path;
      const parts = relative.split("/").filter(Boolean);
      let level = children;
      let prefix = root;
      for (const [index, segment] of parts.slice(0, -1).entries()) {
        prefix = `${prefix}/${segment}`;
        let folder = level.find(
          (node): node is SourceTreeFolder => node.kind === "folder" && node.name === segment,
        );
        if (!folder) {
          folder = { kind: "folder", name: segment, key: prefix, depth: index + 1, children: [] };
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
            aria-expanded={expanded}
            className="tree-folder"
            style={{ paddingLeft: `${10 + node.depth * 14}px` }}
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
            aria-expanded={expanded}
            className="tree-root-label"
            onClick={() => toggleFolder(key)}
            onKeyDown={(event) => handleTreeKey(event, key, expanded)}
          >
            <span className="folder-chevron" aria-hidden="true">{expanded ? "▾" : "▸"}</span>
            {root}
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

function DataEditor({
  file,
  projectRoot,
  editor,
  onCellChange,
  onSave,
  onRecheckSource,
  onSwitchView,
  onReloadConflict,
  onOverwriteConflict,
  cellFocusStart,
}: {
  file: WorkspaceSourceFile;
  projectRoot: string;
  editor: EditorState;
  onCellChange: (recordIndex: number, field: string, value: string) => void;
  onSave: () => void;
  onRecheckSource: () => void;
  onSwitchView: (view: EditorState["view"]) => void;
  onReloadConflict: () => void;
  onOverwriteConflict: () => void;
  cellFocusStart: React.MutableRefObject<Map<string, string>>;
}) {
  const dirty = editorIsDirty(editor);
  const diagnostics = editor.previewState === "current" ? editor.preview.validation.diagnostics : [];
  const lastFocusedCell = useRef<string | null>(null);

  useEffect(() => {
    lastFocusedCell.current = null;
  }, [file.path]);

  useEffect(() => {
    if (editor.view !== "grid" || !lastFocusedCell.current) return;
    const key = lastFocusedCell.current;
    window.requestAnimationFrame(() => {
      document.querySelector<HTMLInputElement>(`[data-cell="${CSS.escape(key)}"]`)?.focus();
    });
  }, [editor.view]);

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
          <Button htmlType="button" onClick={onSave} disabled={!dirty || editor.saving || editor.saveStatus === "outcome_unknown"}>{editor.saving ? "Saving…" : "Save"}</Button>
        </div>
      </header>

      {(editor.saveStatus === "failure" || editor.saveStatus === "outcome_unknown") && (
        <div className="conflict-strip save-recovery-strip">
          <div>
            <strong>{editor.saveStatus === "outcome_unknown" ? "Previous save outcome is unknown." : "Save failed."}</strong>
            <span>Local changes are preserved. Recheck the workspace source before continuing recovery.</span>
          </div>
          <div>
            <Button htmlType="button" onClick={onRecheckSource}>Recheck Source</Button>
            {editor.saveStatus === "failure" && <Button htmlType="button" onClick={onSave}>Retry Save</Button>}
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
            <Button danger htmlType="button" onClick={onOverwriteConflict}>Overwrite</Button>
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
                      {!column.editable && !column.keyField && <em>READ ONLY</em>}
                    </div>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {editor.snapshot.rows.map((row) => (
                <tr key={row.recordIndex}>
                  <th className="row-number">{row.recordIndex + 1}</th>
                  {editor.snapshot.columns.map((column, columnIndex) => {
                    const value = currentCellText(editor, row.recordIndex, column.name);
                    const key = cellKey(row.recordIndex, column.name);
                    const hasDiagnostic = diagnostics.some((diagnostic) =>
                      diagnostic.source != null &&
                      normalizePath(diagnostic.source) === normalizePath(`${projectRoot}/${file.path}`) &&
                      diagnosticRecordIndex(diagnostic) === row.recordIndex && diagnosticField(diagnostic) === column.name);
                    const changed = key in editor.edits;
                    return (
                      <td key={column.name} className={`${changed ? "changed" : ""} ${hasDiagnostic ? "invalid" : ""}`}>
                        <div className="cell-wrap">
                          <Input
                            data-cell={key}
                            aria-label={`record ${row.recordIndex + 1} ${column.name}`}
                            value={value}
                            readOnly={!column.editable || editor.saving}
                            onFocus={() => {
                              cellFocusStart.current.set(key, value);
                              lastFocusedCell.current = key;
                            }}
                            onChange={(event) => onCellChange(row.recordIndex, column.name, event.target.value)}
                            onKeyDown={(event) => handleGridKey(event, row.recordIndex, columnIndex, editor.snapshot, () => {
                              const initial = cellFocusStart.current.get(key);
                              if (initial !== undefined) onCellChange(row.recordIndex, column.name, initial);
                            })}
                          />
                          {hasDiagnostic && <span className="cell-error" title="Validation diagnostic">!</span>}
                        </div>
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
          {editor.snapshot.rows.length === 0 && <div className="empty-grid">This data file has no records.</div>}
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
    </section>
  );
}

function validationLabel(editor: EditorState): string {
  if (editor.previewState === "pending") return "Validating buffer…";
  if (editor.previewState === "unavailable") return "Validation unavailable";
  return editor.preview.validation.valid ? "Buffer valid" : `${editor.preview.validation.diagnostics.length} problems`;
}

function handleGridKey(
  event: React.KeyboardEvent<HTMLInputElement>,
  rowIndex: number,
  columnIndex: number,
  snapshot: DataFileSnapshot,
  cancel: () => void,
) {
  const input = event.currentTarget;
  if (event.key === "Escape") {
    event.preventDefault();
    cancel();
    input.blur();
    return;
  }
  if (event.key === "F2") {
    event.preventDefault();
    input.select();
    return;
  }
  let nextRow = rowIndex;
  let nextColumn = columnIndex;
  if (event.key === "ArrowUp") nextRow -= 1;
  else if (event.key === "ArrowDown" || event.key === "Enter") nextRow += 1;
  else if (event.key === "ArrowLeft" && input.selectionStart === 0) nextColumn -= 1;
  else if (event.key === "ArrowRight" && input.selectionStart === input.value.length) nextColumn += 1;
  else return;
  if (nextRow < 0 || nextRow >= snapshot.rows.length || nextColumn < 0 || nextColumn >= snapshot.columns.length) return;
  event.preventDefault();
  const next = cellKey(snapshot.rows[nextRow].recordIndex, snapshot.columns[nextColumn].name);
  document.querySelector<HTMLInputElement>(`[data-cell="${CSS.escape(next)}"]`)?.focus();
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
