import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type ProjectInfo = {
  project_root: string;
  config_path: string;
  project_id: string;
  name: string;
  version: string;
  source_roots: string[];
  build_output: string;
  build_binary_output: string | null;
  build_cache: string;
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

type LoadState =
  | { kind: "loading" }
  | { kind: "loaded"; project: ProjectInfo }
  | { kind: "error"; diagnostic: Diagnostic };

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

function App() {
  const [state, setState] = useState<LoadState>({ kind: "loading" });

  const loadProject = useCallback(async () => {
    setState({ kind: "loading" });
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
          <button type="button" disabled>
            Validate
          </button>
          <button type="button" disabled>
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
          {state.kind === "loaded" && <ProjectCard project={state.project} />}
        </section>

        <aside className="inspector">
          <span className="section-label">Inspector</span>
          <p className="inspector-copy">Select a table or record to inspect details.</p>
          <div className="status-pill">Shell ready</div>
        </aside>
      </section>
    </main>
  );
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
      </dl>
    </article>
  );
}

export default App;
