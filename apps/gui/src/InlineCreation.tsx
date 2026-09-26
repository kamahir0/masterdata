import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Alert, Button, Spin } from "antd";
import { Check, X } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import type { Category, CreationReport } from "./SourceCreation";

type CreationContext = {
  choices: { tables: string[] };
};

type CreationRequest = {
  sourceRoot: string;
  destination: string;
  artifact: Record<string, unknown>;
};

type Diagnostic = { code: string; message: string } | null;

const uncertainCreations = new Map<string, CreationRequest>();

export type InlineCreationState = {
  root: string;
  folder: string;
  category: Category;
  initialTable?: string;
};

function diagnosticOf(error: unknown): { code: string; message: string } {
  if (typeof error === "object" && error !== null && "diagnostic" in error) {
    const diagnostic = (error as { diagnostic?: { code?: string; message?: string } }).diagnostic;
    if (diagnostic) {
      return { code: diagnostic.code ?? "E-SOURCE-CREATE", message: diagnostic.message ?? String(error) };
    }
  }
  return { code: "E-SOURCE-CREATE", message: String(error) };
}

function filenameWithExtension(value: string, category: Category): string {
  const trimmed = value.trim();
  if (category === "folder" || trimmed.length === 0 || /\.ya?ml$/i.test(trimmed)) return trimmed;
  return `${trimmed}.yaml`;
}

function requestFor({ state, filename, table }: { state: InlineCreationState; filename: string; table: string }): CreationRequest {
  const destination = state.folder ? `${state.folder}/${filename}` : filename;
  if (state.category === "folder") {
    return { sourceRoot: state.root, destination, artifact: { category: "folder" } };
  }
  return {
    sourceRoot: state.root,
    destination,
    artifact: {
      category: "starter",
      kind: state.category,
      ...(state.category === "data" && table ? { table } : {}),
    },
  };
}

export function InlineCreationRow({
  projectPath,
  state,
  canWrite,
  onCommit,
  onCancel,
}: {
  projectPath: string;
  state: InlineCreationState;
  canWrite: boolean;
  onCommit: (report: CreationReport) => Promise<void>;
  onCancel: () => void;
}) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [filename, setFilename] = useState(state.category === "folder" ? "new-folder" : "new.yaml");
  const [table, setTable] = useState(state.initialTable ?? "");
  const [tables, setTables] = useState<string[]>([]);
  const [contextLoading, setContextLoading] = useState(state.category === "data");
  const [busy, setBusy] = useState(false);
  const [diagnostic, setDiagnostic] = useState<Diagnostic>(null);
  const [status, setStatus] = useState<CreationReport["status"] | null>(() => {
    return uncertainCreations.has(projectPath) ? "outcome_unknown" : null;
  });
  const submittedRequest = useRef<CreationRequest | null>(uncertainCreations.get(projectPath) ?? null);

  useLayoutEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  useEffect(() => {
    if (state.category !== "data") return;
    let disposed = false;
    setContextLoading(true);
    void invoke<CreationContext>("creation_context", { projectPath })
      .then((context) => {
        if (!disposed) {
          setTables(context.choices.tables);
          if (!table && context.choices.tables.length === 1) setTable(context.choices.tables[0]);
        }
      })
      .catch((error) => {
        if (!disposed) setDiagnostic(diagnosticOf(error));
      })
      .finally(() => {
        if (!disposed) setContextLoading(false);
      });
    return () => { disposed = true; };
  }, [projectPath, state.category]);

  const submit = async () => {
    if (busy || !canWrite || status === "outcome_unknown") return;
    const normalized = filenameWithExtension(filename, state.category);
    if (!normalized || (state.category === "data" && !table)) return;
    const request = requestFor({ state, filename: normalized, table });
    setBusy(true);
    setDiagnostic(null);
    try {
      const report = await invoke<CreationReport>("create_source", { projectPath, request });
      setStatus(report.status);
      if (report.status === "success") {
        await onCommit(report);
      } else {
        if (report.status === "outcome_unknown") {
          submittedRequest.current = request;
          uncertainCreations.set(projectPath, request);
        }
        setDiagnostic(report.diagnostic);
        setFilename(normalized);
      }
    } catch (error) {
      const next = diagnosticOf(error);
      setDiagnostic(next);
      if (error && typeof error === "object" && "diagnostic" in error) {
        // Structured preflight rejection is safe to correct and retry; only an
        // unstructured transport failure makes the commit outcome unknown.
        setStatus("failure");
      } else {
        setStatus("outcome_unknown");
        submittedRequest.current = request;
        uncertainCreations.set(projectPath, request);
      }
    } finally {
      setBusy(false);
    }
  };

  const recheck = async () => {
    const request = submittedRequest.current;
    if (busy || !request) return;
    setBusy(true);
    try {
      const actual = await invoke<{ exists: boolean }>("recheck_creation", { projectPath, request });
      if (actual.exists) {
        setStatus("conflict");
        setDiagnostic({ code: "E-SOURCE-CREATE-CONFLICT", message: "Destination exists. It will not be overwritten." });
        submittedRequest.current = null;
        uncertainCreations.delete(projectPath);
      } else {
        setStatus(null);
        setDiagnostic(null);
        submittedRequest.current = null;
        uncertainCreations.delete(projectPath);
      }
    } catch (error) {
      setDiagnostic(diagnosticOf(error));
    } finally {
      setBusy(false);
    }
  };

  const onKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    // IME Enter is composition confirmation, not an Explorer commit.
    if (event.nativeEvent.isComposing || event.keyCode === 229) return;
    if (event.key === "Enter") {
      event.preventDefault();
      void submit();
    } else if (event.key === "Escape") {
      event.preventDefault();
      if (!busy) onCancel();
    }
  };

  const categoryLabel = state.category === "custom_type" ? "Custom Type" : state.category === "value_object" ? "Value Object" : state.category === "flags" ? "Flags Enum" : state.category[0].toUpperCase() + state.category.slice(1);
  const inputId = `inline-creation-${state.root}-${state.folder}-${state.category}`.replace(/[^a-zA-Z0-9_-]/g, "-");
  const disabled = busy || !canWrite || status === "outcome_unknown" || (state.category === "data" && contextLoading);

  return (
    <div className="tree-inline-creation" role="treeitem" aria-label={`New ${categoryLabel}`} data-inline-creation="true">
      <span className={`file-kind ${state.category}`}>{state.category === "folder" ? "▸" : "＋"}</span>
      <div className="inline-creation-fields">
        <label className="sr-only" htmlFor={inputId}>{state.category === "folder" ? "Folder name" : state.category === "table" ? "Table identity" : state.category === "data" ? "Data filename" : "Type identity"}</label>
        <label className="inline-creation-label" htmlFor={inputId}>{categoryLabel}</label>
        <input
          ref={inputRef}
          id={inputId}
          aria-label={state.category === "folder" ? "Folder name" : "Filename (.yaml / .yml)"}
          value={filename}
          disabled={disabled}
          onChange={(event) => { setFilename(event.target.value); setDiagnostic(null); setStatus(null); }}
          onKeyDown={onKeyDown}
          onBlur={() => { if (!busy && filename.trim().length === 0) onCancel(); }}
        />
        {state.category === "data" && (
          <select aria-label="Existing Table" value={table} disabled={disabled} onChange={(event) => setTable(event.target.value)}>
            <option value="">Select Table…</option>
            {tables.map((candidate) => <option key={candidate} value={candidate}>{candidate}</option>)}
          </select>
        )}
      </div>
      <Button type="text" size="small" aria-label="Create" disabled={disabled || !filename.trim() || (state.category === "data" && !table)} loading={busy} icon={<Check size={14} />} onClick={() => void submit()} />
      <Button type="text" size="small" aria-label="Cancel" disabled={busy} icon={<X size={14} />} onClick={onCancel} />
      {contextLoading && <Spin size="small" />}
      {diagnostic && <Alert className="inline-creation-error" type="error" showIcon title={status ?? "failure"} description={`${diagnostic.code}: ${diagnostic.message}`} />}
      {status === "outcome_unknown" && <Button size="small" onClick={() => void recheck()} disabled={busy}>Recheck Workspace</Button>}
    </div>
  );
}
