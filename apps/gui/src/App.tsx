import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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
  publish_targets: { kind: "csharp" | "binary"; path: string; resolved_path: string }[];
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

type Diagnostic = {
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

type ApiError = { diagnostic: Diagnostic };

// Keep the shared ValidationReport serialization at the Tauri boundary so the
// GUI does not invent a second validation result schema.
type ValidationDiagnostic = {
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

type ValidationReport = {
  valid: boolean;
  files_scanned: number;
  schema_documents: number;
  data_documents: number;
  type_documents: number;
  tables: string[];
  types: string[];
  diagnostics: ValidationDiagnostic[];
};

type LoadState =
  | { kind: "loading" }
  | { kind: "loaded"; project: ProjectInfo }
  | { kind: "error"; diagnostic: Diagnostic };

type ValidationState =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "loaded"; report: ValidationReport }
  | { kind: "error"; diagnostic: Diagnostic };

type BuildState =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "loaded"; response: BuildResponse }
  | { kind: "error"; diagnostic: Diagnostic };

type DisplayDiagnostic = {
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

function normalizeDiagnostic(
  diagnostic: Diagnostic | ValidationDiagnostic,
): DisplayDiagnostic {
  // Command errors use the existing camelCase DTO; reports preserve the core
  // diagnostic field names. Normalize only for presentation.
  const usesCamelCaseFields = "schemaPath" in diagnostic;
  return {
    code: diagnostic.code,
    kind: diagnostic.kind,
    message: diagnostic.message,
    source: diagnostic.source ?? null,
    line: diagnostic.line ?? null,
    column: diagnostic.column ?? null,
    schemaPath: usesCamelCaseFields
      ? diagnostic.schemaPath ?? null
      : diagnostic.schema_path ?? null,
    recordIdentity: usesCamelCaseFields
      ? diagnostic.recordIdentity ?? null
      : diagnostic.record_identity ?? null,
    suggestion: diagnostic.suggestion ?? null,
    relatedRequirements: usesCamelCaseFields
      ? diagnostic.relatedRequirements ?? []
      : diagnostic.related_requirements ?? [],
  };
}

function App() {
  const [state, setState] = useState<LoadState>({ kind: "loading" });
  const [validationState, setValidationState] = useState<ValidationState>({
    kind: "idle",
  });
  const [buildState, setBuildState] = useState<BuildState>({ kind: "idle" });

  const loadProject = useCallback(async () => {
    setState({ kind: "loading" });
    setValidationState({ kind: "idle" });
    setBuildState({ kind: "idle" });
    try {
      // The Rust Tauri command resolves the project through masterdata-app
      // and masterdata-core.
      // The frontend does not inspect the filesystem or invoke the CLI.
      const project = await invoke<ProjectInfo>("project_info", {
        projectPath: null,
      });
      setState({ kind: "loaded", project });
    } catch (error) {
      setState({ kind: "error", diagnostic: asApiError(error).diagnostic });
    }
  }, []);

  useEffect(() => {
    void loadProject();
  }, [loadProject]);

  const validateProject = useCallback(async () => {
    if (state.kind !== "loaded") {
      return;
    }

    setValidationState({ kind: "loading" });
    try {
      const report = await invoke<ValidationReport>("validate", {
        projectPath: state.project.project_root,
      });
      setValidationState({ kind: "loaded", report });
    } catch (error) {
      setValidationState({
        kind: "error",
        diagnostic: asApiError(error).diagnostic,
      });
    }
  }, [state]);

  const validationStatus = validationStatusLabel(validationState);
  const buildStatus = buildStatusLabel(buildState);

  const buildProject = useCallback(async () => {
    if (state.kind !== "loaded" || buildState.kind === "loading") {
      return;
    }

    setBuildState({ kind: "loading" });
    try {
      const response = await invoke<BuildResponse>("build", {
        projectPath: state.project.project_root,
        dryRun: false,
      });
      setBuildState({ kind: "loaded", response });
    } catch (error) {
      setBuildState({
        kind: "error",
        diagnostic: asApiError(error).diagnostic,
      });
    }
  }, [buildState.kind, state]);

  return (
    <main className="shell">
      <header className="topbar">
        <div>
          <p className="eyebrow">LOCAL-FIRST MASTER DATA</p>
          <h1>masterdata</h1>
        </div>
        <div className="actions">
          <button type="button" onClick={() => void loadProject()}>
            Reload project
          </button>
          <button
            type="button"
            onClick={() => void validateProject()}
            disabled={state.kind !== "loaded" || validationState.kind === "loading"}
          >
            Validate
          </button>
          <button
            type="button"
            onClick={() => void buildProject()}
            disabled={state.kind !== "loaded" || buildState.kind === "loading"}
          >
            Build
          </button>
        </div>
      </header>

      <section className="workspace" aria-label="masterdata workspace">
        <aside className="sidebar">
          <span className="section-label">Navigation</span>
          <div className="nav-item active">Project overview</div>
          <div className="nav-item muted">Tables (coming soon)</div>
          <div className="nav-item muted">Types (coming soon)</div>
        </aside>

        <section className="content">
          <span className="section-label">Current project</span>
          {state.kind === "loading" && <p className="message">Loading project through Rust application service…</p>}
          {state.kind === "error" && (
            <div className="error-card">
              <h2>Project could not be opened</h2>
              <p>{state.diagnostic.message}</p>
              <code>{state.diagnostic.code}</code>
              {state.diagnostic.source && <p>{state.diagnostic.source}</p>}
              {state.diagnostic.suggestion && <p>{state.diagnostic.suggestion}</p>}
              <p className="hint">Run <code>cargo xtask dev-reset</code> and start the GUI again.</p>
            </div>
          )}
          {state.kind === "loaded" && (
            <>
              <ProjectCard project={state.project} />
              <ValidationPanel state={validationState} />
              <BuildPanel state={buildState} />
            </>
          )}
        </section>

        <aside className="inspector">
          <span className="section-label">Inspector</span>
          <p className="inspector-copy">Select a table or record to inspect details.</p>
          <div className="status-stack">
            <div className="status-pill">{validationStatus}</div>
            <div className="status-pill">{buildStatus}</div>
          </div>
        </aside>
      </section>
    </main>
  );
}

function validationStatusLabel(state: ValidationState): string {
  switch (state.kind) {
    case "idle":
      return "Validation not run";
    case "loading":
      return "Validating…";
    case "error":
      return "Validation unavailable";
    case "loaded":
      return state.report.valid ? "Validation passed" : "Validation failed";
  }
}

function buildStatusLabel(state: BuildState): string {
  switch (state.kind) {
    case "idle":
      return "Build not run";
    case "loading":
      return "Building…";
    case "error":
      return "Build failed";
    case "loaded":
      return "Build complete";
  }
}

function BuildPanel({ state }: { state: BuildState }) {
  return (
    <section className="build-panel" aria-live="polite">
      <div className="build-heading">
        <div>
          <p className="card-kicker">Canonical build</p>
          <h2>Build result</h2>
        </div>
        {state.kind === "loaded" && (
          <span className="validation-badge is-valid">Complete</span>
        )}
      </div>

      {state.kind === "idle" && (
        <p className="build-message">Run a full build to create the current canonical artifact set.</p>
      )}
      {state.kind === "loading" && (
        <p className="build-message">Building canonical artifacts through Rust and .NET…</p>
      )}
      {state.kind === "error" && (
        <DiagnosticCard
          heading="Build could not be completed"
          diagnostic={normalizeDiagnostic(state.diagnostic)}
        />
      )}
      {state.kind === "loaded" && <BuildResult response={state.response} />}
    </section>
  );
}

function BuildResult({ response }: { response: BuildResponse }) {
  return (
    <>
      <p className="build-success">
        {response.dryRun
          ? "Dry run completed without publishing artifacts."
          : "Canonical artifact set created successfully."}
      </p>
      <dl className="build-details">
        <div>
          <dt>Artifact root</dt>
          <dd>{response.artifactRoot}</dd>
        </div>
        <div>
          <dt>Canonical C#</dt>
          <dd>{response.csharpOutput}</dd>
        </div>
        <div>
          <dt>Canonical binary</dt>
          <dd>{response.binaryOutput}</dd>
        </div>
        <div>
          <dt>Build cache</dt>
          <dd>{response.cache}</dd>
        </div>
        <div>
          <dt>Schema source hash</dt>
          <dd>{response.schemaSourceContentHash}</dd>
        </div>
      </dl>

      <div className="build-files">
        <h3>Generated C# ({response.generatedFiles.length})</h3>
        {response.generatedFiles.length > 0 ? (
          <ul>
            {response.generatedFiles.map((file) => (
              <li key={file}><code>{file}</code></li>
            ))}
          </ul>
        ) : (
          <p>No C# files were generated.</p>
        )}
      </div>
    </>
  );
}

function ValidationPanel({ state }: { state: ValidationState }) {
  return (
    <section className="validation-panel" aria-live="polite">
      <div className="validation-heading">
        <div>
          <p className="card-kicker">Source validation</p>
          <h2>Validation result</h2>
        </div>
        {state.kind === "loaded" && (
          <span className={`validation-badge ${state.report.valid ? "is-valid" : "is-invalid"}`}>
            {state.report.valid ? "Valid" : "Needs attention"}
          </span>
        )}
      </div>

      {state.kind === "idle" && (
        <p className="validation-message">Run validation to inspect the current project sources.</p>
      )}
      {state.kind === "loading" && (
        <p className="validation-message">Validating current project sources through Rust…</p>
      )}
      {state.kind === "error" && (
        <DiagnosticCard
          heading="Validation could not be completed"
          diagnostic={normalizeDiagnostic(state.diagnostic)}
        />
      )}
      {state.kind === "loaded" && <ValidationResult report={state.report} />}
    </section>
  );
}

function ValidationResult({ report }: { report: ValidationReport }) {
  return (
    <>
      <dl className="validation-summary">
        <div>
          <dt>Files scanned</dt>
          <dd>{report.files_scanned}</dd>
        </div>
        <div>
          <dt>Schemas</dt>
          <dd>{report.schema_documents}</dd>
        </div>
        <div>
          <dt>Types</dt>
          <dd>{report.type_documents}</dd>
        </div>
        <div>
          <dt>Data documents</dt>
          <dd>{report.data_documents}</dd>
        </div>
      </dl>

      <div className="validation-collections">
        <div>
          <span className="collection-label">Tables</span>
          <span>{report.tables.length > 0 ? report.tables.join(", ") : "None"}</span>
        </div>
        <div>
          <span className="collection-label">Types</span>
          <span>{report.types.length > 0 ? report.types.join(", ") : "None"}</span>
        </div>
      </div>

      {report.diagnostics.length === 0 ? (
        <p className="validation-success">No diagnostics. The project sources are valid.</p>
      ) : (
        <div>
          <h3 className="diagnostics-heading">Diagnostics ({report.diagnostics.length})</h3>
          <ol className="diagnostic-list">
            {report.diagnostics.map((diagnostic, index) => (
              <li key={`${diagnostic.code}-${index}`}>
                <DiagnosticCard diagnostic={normalizeDiagnostic(diagnostic)} />
              </li>
            ))}
          </ol>
        </div>
      )}
    </>
  );
}

function DiagnosticCard({
  heading,
  diagnostic,
}: {
  heading?: string;
  diagnostic: DisplayDiagnostic;
}) {
  const location = formatDiagnosticLocation(diagnostic);

  return (
    <article className="diagnostic-card">
      {heading && <h3>{heading}</h3>}
      <div className="diagnostic-title">
        <code>{diagnostic.code}</code>
        <span>{diagnostic.kind}</span>
      </div>
      <p>{diagnostic.message}</p>
      {location && <p className="diagnostic-location">{location}</p>}
      {diagnostic.schemaPath && <p>Schema path: <code>{diagnostic.schemaPath}</code></p>}
      {diagnostic.recordIdentity && <p>Record: <code>{diagnostic.recordIdentity}</code></p>}
      {diagnostic.suggestion && <p>Suggestion: {diagnostic.suggestion}</p>}
      {diagnostic.relatedRequirements.length > 0 && (
        <p>Requirements: {diagnostic.relatedRequirements.join(", ")}</p>
      )}
    </article>
  );
}

function formatDiagnosticLocation(diagnostic: DisplayDiagnostic): string | null {
  const position = [diagnostic.line, diagnostic.column]
    .filter((value): value is number => value !== null)
    .join(":");

  if (!diagnostic.source) {
    return position || null;
  }

  return position ? `${diagnostic.source}:${position}` : diagnostic.source;
}

function ProjectCard({ project }: { project: ProjectInfo }) {
  return (
    <article className="project-card">
      <div className="project-heading">
        <div>
          <p className="card-kicker">Project identity</p>
          <h2>{project.name}</h2>
        </div>
        <span className="version">v{project.version}</span>
      </div>
      <dl className="details">
        <div>
          <dt>Project ID</dt>
          <dd>{project.project_id}</dd>
        </div>
        <div>
          <dt>Root</dt>
          <dd>{project.project_root}</dd>
        </div>
        <div>
          <dt>Config</dt>
          <dd>{project.config_path}</dd>
        </div>
        <div>
          <dt>Source roots</dt>
          <dd>{project.source_roots.join(", ")}</dd>
        </div>
        <div>
          <dt>Canonical artifacts</dt>
          <dd>{project.artifact_root}</dd>
        </div>
        <div>
          <dt>Canonical C#</dt>
          <dd>{project.csharp_output}</dd>
        </div>
        <div>
          <dt>Canonical binary</dt>
          <dd>{project.binary_output}</dd>
        </div>
        <div>
          <dt>Build cache</dt>
          <dd>{project.cache}</dd>
        </div>
      </dl>
    </article>
  );
}

export default App;
