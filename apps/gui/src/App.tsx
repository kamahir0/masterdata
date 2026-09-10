import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
  conflict: SourceContentState | null;
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
    conflict: null,
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
  const previewTimers = useRef(new Map<string, number>());
  const editorsRef = useRef(editors);
  const workspaceStateRef = useRef(workspaceState);
  const cellFocusStart = useRef(new Map<string, string>());

  useEffect(() => {
    editorsRef.current = editors;
  }, [editors]);

  useEffect(() => {
    workspaceStateRef.current = workspaceState;
  }, [workspaceState]);

  const workspace = workspaceState.kind === "ready"
    ? workspaceState.workspace
    : workspaceState.previous;
  const projectRoot = workspace?.project.project_root ?? null;
  const activeFile = workspace?.files.find((file) => file.path === activePath) ?? null;
  const activeEditor = activePath ? editors[activePath] ?? null : null;
  const dirtyCount = Object.values(editors).filter(editorIsDirty).length;

  const showNotice = useCallback((message: string) => {
    setNotice(message);
    window.setTimeout(() => setNotice(null), 2600);
  }, []);

  const openDataFile = useCallback(async (root: string, path: string, force = false) => {
    setActivePath(path);
    if (!force && editorsRef.current[path]) {
      return;
    }
    try {
      const snapshot = await invoke<DataFileSnapshot>("open_data_file", {
        projectPath: root,
        relativePath: path,
      });
      setEditors((current) => ({ ...current, [path]: editorFromSnapshot(snapshot) }));
    } catch (error) {
      const diagnostic = asApiError(error).diagnostic;
      setWorkspaceState((current) => ({
        kind: "error",
        diagnostic,
        previous: current.kind === "ready" ? current.workspace : current.previous,
      }));
    }
  }, []);

  const loadWorkspace = useCallback(async (requestedProject: string | null) => {
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
      setWorkspaceState({ kind: "ready", workspace: next });
      setProjectPathInput(next.project.project_root);
      setEditors({});
      const first = next.files.find((file) => file.kind === "data") ?? next.files[0] ?? null;
      setActivePath(first?.path ?? null);
      if (first?.kind === "data") {
        await openDataFile(next.project.project_root, first.path, true);
      }
    } catch (error) {
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
    const revision = editor.revision;
    const timer = window.setTimeout(async () => {
      previewTimers.current.delete(path);
      try {
        const preview = await invoke<SourceEditPreview>("preview_data_file", {
          projectPath: root,
          relativePath: path,
          baseSource: editor.snapshot.baseSource,
          edits: Object.values(editor.edits),
        });
        setEditors((current) => {
          const latest = current[path];
          if (!latest || latest.revision !== revision) {
            return current;
          }
          return {
            ...current,
            [path]: {
              ...latest,
              preview,
              previewState: "current",
              previewError: null,
            },
          };
        });
      } catch (error) {
        const diagnostic = asApiError(error).diagnostic;
        setEditors((current) => {
          const latest = current[path];
          if (!latest || latest.revision !== revision) {
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
            saveDiagnostic: report.diagnostic,
            view: report.status === "conflict" ? "compare" : latest.view,
          },
        };
      });
      return false;
    } catch (error) {
      const diagnostic = asApiError(error).diagnostic;
      setEditors((current) => current[path]
        ? {
            ...current,
            [path]: {
              ...current[path],
              saving: false,
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
      for (const [path, editor] of Object.entries(editorsRef.current)) {
        if (editor.saving) continue;
        void invoke<SourceContentState>("source_content", {
          projectPath: root,
          relativePath: path,
        }).then((current) => {
          const latest = editorsRef.current[path];
          if (!latest || latest.saving) return;
          if (current.contentIdentity === latest.snapshot.baseContentIdentity) {
            if (latest.conflict) {
              setEditors((all) => all[path]
                ? { ...all, [path]: { ...all[path], conflict: null } }
                : all);
            }
            return;
          }
          if (editorIsDirty(latest)) {
            setEditors((all) => all[path]
              ? { ...all, [path]: { ...all[path], conflict: current } }
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
    setManualValidation({ kind: "loading" });
    try {
      const report = await invoke<ValidationReport>("validate", { projectPath: projectRoot });
      setManualValidation({ kind: "done", value: report });
      setProblemsOpen(true);
    } catch (error) {
      setManualValidation({ kind: "error", diagnostic: asApiError(error).diagnostic });
    }
  }, [projectRoot, workspace]);

  const runBuild = useCallback(async () => {
    if (!workspace?.capabilities.build || !projectRoot || buildState.kind === "loading") return;
    if (dirtyCount > 0) {
      showNotice("Build uses saved source only; unsaved changes are not included.");
    }
    setBuildState({ kind: "loading" });
    try {
      const response = await invoke<BuildResponse>("build", {
        projectPath: projectRoot,
        dryRun: false,
      });
      setBuildState({ kind: "done", value: response });
      showNotice("Build complete");
    } catch (error) {
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

  const switchView = useCallback((path: string, view: EditorState["view"]) => {
    setEditors((current) => current[path]
      ? { ...current, [path]: { ...current[path], view } }
      : current);
  }, []);

  const focusDiagnostic = useCallback(async (diagnostic: Diagnostic) => {
    if (!workspace) return;
    const source = diagnostic.source ? normalizePath(diagnostic.source) : null;
    const file = source
      ? workspace.files.find((candidate) => source.endsWith(normalizePath(candidate.path)))
      : null;
    if (!file) return;
    selectFile(file);
    if (file.kind !== "data") return;
    const record = diagnosticRecordIndex(diagnostic);
    const field = diagnosticField(diagnostic);
    if (record === null || !field) return;
    window.setTimeout(() => {
      document.querySelector<HTMLInputElement>(`[data-cell="${CSS.escape(cellKey(record, field))}"]`)?.focus();
    }, 120);
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
  const problems = [...activeDiagnostics, ...manualDiagnostics, ...operationDiagnostics];

  return (
    <main className="app-shell">
      <header className="titlebar">
        <div className="brand-block">
          <div className="brand-mark">M</div>
          <div>
            <strong>masterdata</strong>
            <span>{workspace?.project.name ?? "No project"}</span>
          </div>
        </div>
        <div className="project-open">
          <input
            aria-label="Project path"
            value={projectPathInput}
            onChange={(event) => setProjectPathInput(event.target.value)}
            placeholder="Project folder path"
          />
          <button
            type="button"
            onClick={() => projectPathInput.trim() && requestAction({ kind: "open", projectPath: projectPathInput.trim() })}
          >
            Open Project
          </button>
        </div>
        <div className="command-bar">
          <button type="button" onClick={() => requestAction({ kind: "reload" })} disabled={!workspace}>Reload</button>
          <button type="button" onClick={() => activePath && void saveFile(activePath)} disabled={!activeEditor || !editorIsDirty(activeEditor) || activeEditor.saving || !workspace?.capabilities.workspaceWrite}>
            {activeEditor?.saving ? "Saving…" : "Save"}
          </button>
          <button type="button" onClick={() => void validateDisk()} disabled={!workspace?.capabilities.validate || manualValidation.kind === "loading"}>
            {manualValidation.kind === "loading" ? "Validating…" : "Validate"}
          </button>
          <button className="primary" type="button" onClick={() => void runBuild()} disabled={!workspace?.capabilities.build || buildState.kind === "loading"}>
            {buildState.kind === "loading" ? "Building…" : "Build"}
          </button>
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
          {activeFile?.kind === "data" && !activeEditor && (
            <EmptyEditor title={`Opening ${sourceName(activeFile.path)}…`} copy="Loading records through the shared application service." />
          )}
          {activeFile?.kind === "data" && activeEditor && (
            <DataEditor
              file={activeFile}
              editor={activeEditor}
              onCellChange={(recordIndex, field, value) => updateCell(activeFile.path, recordIndex, field, value)}
              onSave={() => void saveFile(activeFile.path)}
              onSwitchView={(view) => switchView(activeFile.path, view)}
              onReloadConflict={() => void reloadConflict(activeFile.path)}
              onOverwriteConflict={() => void overwriteConflict(activeFile.path)}
              cellFocusStart={cellFocusStart}
            />
          )}

          <section className={`problems-panel ${problemsOpen ? "open" : "collapsed"}`}>
            <button className="problems-header" type="button" onClick={() => setProblemsOpen((value) => !value)}>
              <span>PROBLEMS <b>{problems.length}</b></span>
              <span className="problem-snapshot">
                {activeEditor?.previewState === "pending" && "Buffer validation pending"}
                {activeEditor?.previewState === "unavailable" && "Buffer validation unavailable"}
                {activeEditor?.previewState === "current" && (activeEditor.preview.validation.valid ? "Buffer valid" : "Buffer has diagnostics")}
                {manualValidation.kind === "done" && ` · Disk ${manualValidation.value.valid ? "valid" : "invalid"}`}
              </span>
            </button>
            {problemsOpen && (
              <div className="problems-body">
                {activeEditor?.previewError && <DiagnosticBanner diagnostic={apiDiagnosticToDiagnostic(activeEditor.previewError)} />}
                {problems.length === 0 ? (
                  <div className="no-problems">No diagnostics for the current buffer.</div>
                ) : (
                  problems.map((diagnostic, index) => (
                    <button className="problem-row" type="button" key={`${diagnostic.code}-${index}`} onClick={() => void focusDiagnostic(diagnostic)}>
                      <span className="problem-icon">!</span>
                      <strong>{diagnostic.code}</strong>
                      <span>{diagnostic.message}</span>
                      <small>{formatDiagnosticLocation(diagnostic)}</small>
                    </button>
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

      {pendingAction && (
        <div className="modal-backdrop" role="presentation">
          <section className="save-all-dialog" role="dialog" aria-modal="true" aria-labelledby="save-all-title">
            <span className="dialog-kicker">UNSAVED CHANGES</span>
            <h2 id="save-all-title">Save changes before continuing?</h2>
            <p>{dirtyCount} source file{dirtyCount === 1 ? " has" : "s have"} unsaved changes. The requested action would discard the current buffers.</p>
            <div className="dialog-actions">
              <button type="button" onClick={() => setPendingAction(null)} disabled={pendingActionBusy}>Cancel</button>
              <button type="button" onClick={() => void performAction(pendingAction)} disabled={pendingActionBusy}>Don't Save</button>
              <button className="primary" type="button" disabled={pendingActionBusy} onClick={async () => {
                setPendingActionBusy(true);
                const action = pendingAction;
                const ok = await saveAll();
                setPendingActionBusy(false);
                if (ok) await performAction(action);
              }}>
                {pendingActionBusy ? "Saving…" : "Save All"}
              </button>
            </div>
          </section>
        </div>
      )}

      {notice && <div className="toast">{notice}</div>}
    </main>
  );
}

function SourceTree({
  workspace,
  activePath,
  editors,
  onSelect,
}: {
  workspace: AuthoringWorkspace;
  activePath: string | null;
  editors: Record<string, EditorState>;
  onSelect: (file: WorkspaceSourceFile) => void;
}) {
  const groups = useMemo(() => {
    const result = new Map<string, WorkspaceSourceFile[]>();
    for (const root of workspace.sourceRoots) result.set(root, []);
    for (const file of workspace.files) {
      const items = result.get(file.sourceRoot) ?? [];
      items.push(file);
      result.set(file.sourceRoot, items);
    }
    return [...result.entries()];
  }, [workspace]);

  return <div className="source-tree">
    {groups.map(([root, files]) => (
      <div className="source-root" key={root}>
        <div className="tree-root-label"><span>▾</span>{root}</div>
        {files.map((file) => {
          const relative = file.path.startsWith(`${root}/`) ? file.path.slice(root.length + 1) : file.path;
          const depth = Math.max(0, relative.split("/").length - 1);
          const editor = editors[file.path];
          return (
            <button
              key={file.path}
              type="button"
              className={`tree-file ${activePath === file.path ? "active" : ""}`}
              style={{ paddingLeft: `${20 + depth * 14}px` }}
              onClick={() => onSelect(file)}
              title={file.path}
            >
              <span className={`file-kind ${file.kind}`}>{file.kind === "data" ? "▦" : file.kind === "schema" ? "T" : file.kind === "type" ? "◇" : "!"}</span>
              <span className="tree-file-name">{sourceName(file.path)}</span>
              {editorIsDirtySafe(editor) && <span className="dirty-dot" title="Unsaved changes">●</span>}
              {editor?.conflict && <span className="conflict-badge" title="External change conflict">!</span>}
            </button>
          );
        })}
      </div>
    ))}
    {workspace.files.length === 0 && <div className="pane-message">No YAML source documents.</div>}
  </div>;
}

function editorIsDirtySafe(editor: EditorState | undefined): boolean {
  return editor ? editorIsDirty(editor) : false;
}

function DataEditor({
  file,
  editor,
  onCellChange,
  onSave,
  onSwitchView,
  onReloadConflict,
  onOverwriteConflict,
  cellFocusStart,
}: {
  file: WorkspaceSourceFile;
  editor: EditorState;
  onCellChange: (recordIndex: number, field: string, value: string) => void;
  onSave: () => void;
  onSwitchView: (view: EditorState["view"]) => void;
  onReloadConflict: () => void;
  onOverwriteConflict: () => void;
  cellFocusStart: React.MutableRefObject<Map<string, string>>;
}) {
  const dirty = editorIsDirty(editor);
  const diagnostics = editor.previewState === "current" ? editor.preview.validation.diagnostics : [];

  return (
    <section className="data-editor">
      <header className="editor-tabs">
        <div className="document-tab">
          <span className="file-kind data">▦</span>
          <strong>{sourceName(file.path)}</strong>
          {dirty && <span className="tab-dirty">●</span>}
        </div>
        <div className="view-tabs" role="tablist">
          <button className={editor.view === "grid" ? "selected" : ""} type="button" onClick={() => onSwitchView("grid")}>Data</button>
          <button className={editor.view === "diff" ? "selected" : ""} type="button" onClick={() => onSwitchView("diff")}>Diff</button>
          {editor.conflict && <button className={editor.view === "compare" ? "selected conflict" : "conflict"} type="button" onClick={() => onSwitchView("compare")}>Conflict</button>}
        </div>
        <div className="editor-actions">
          <span className={`validation-state ${editor.previewState}`}>{validationLabel(editor)}</span>
          <button type="button" onClick={onSave} disabled={!dirty || editor.saving}>{editor.saving ? "Saving…" : "Save"}</button>
        </div>
      </header>

      {editor.conflict && (
        <div className="conflict-strip">
          <div>
            <strong>File changed outside masterdata.</strong>
            <span>Your unsaved buffer is preserved. Normal Save will not overwrite the external version.</span>
          </div>
          <div>
            <button type="button" onClick={() => onSwitchView("compare")}>Compare</button>
            <button type="button" onClick={onReloadConflict}>Reload</button>
            <button className="danger" type="button" onClick={onOverwriteConflict}>Overwrite</button>
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
                      diagnosticRecordIndex(diagnostic) === row.recordIndex && diagnosticField(diagnostic) === column.name);
                    const changed = key in editor.edits;
                    return (
                      <td key={column.name} className={`${changed ? "changed" : ""} ${hasDiagnostic ? "invalid" : ""}`}>
                        <div className="cell-wrap">
                          <input
                            data-cell={key}
                            aria-label={`record ${row.recordIndex + 1} ${column.name}`}
                            value={value}
                            readOnly={!column.editable || editor.saving}
                            onFocus={() => cellFocusStart.current.set(key, value)}
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
  return <section className="empty-editor"><div className="empty-symbol">▦</div><h2>{title}</h2><p>{copy}</p></section>;
}

function DiagnosticBanner({ diagnostic }: { diagnostic: Diagnostic }) {
  return (
    <div className="diagnostic-banner">
      <strong>{diagnostic.code}</strong>
      <span>{diagnostic.message}</span>
      {diagnostic.suggestion && <small>{diagnostic.suggestion}</small>}
    </div>
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
