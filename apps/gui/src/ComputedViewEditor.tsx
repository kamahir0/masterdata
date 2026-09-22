import { useEffect, useState } from "react";
import { Alert, Button, Form, Input, Modal, Space, Spin, Tag } from "antd";
import { Plus, Trash2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

type Diagnostic = { code: string; message: string; source?: string | null; line?: number | null; column?: number | null };
type Column = { name: string; expression: string };
type Snapshot = {
  path: string;
  baseSource: string;
  baseContentIdentity: string;
  name: string;
  table: string;
  columns: Column[];
  diagnostics: Diagnostic[];
};
type Preview = {
  candidateSource: string;
  candidateContentIdentity: string;
  changed: boolean;
  validation: { valid: boolean; diagnostics: Diagnostic[] };
};
type SaveReport = {
  status: "success" | "conflict" | "failure" | "outcome_unknown";
  snapshot: Snapshot | null;
  current: { source: string; contentIdentity: string } | null;
  diagnostic: Diagnostic | null;
};

export default function ComputedViewEditor({ projectPath, path, canWrite, onSaved }: {
  projectPath: string;
  path: string;
  canWrite: boolean;
  onSaved: () => Promise<void> | void;
}) {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [name, setName] = useState("");
  const [table, setTable] = useState("");
  const [columns, setColumns] = useState<Column[]>([]);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [diagnostic, setDiagnostic] = useState<Diagnostic | null>(null);
  const [busy, setBusy] = useState(false);

  const install = (next: Snapshot) => {
    setSnapshot(next);
    setName(next.name);
    setTable(next.table);
    setColumns(next.columns);
    setPreview(null);
    setDiagnostic(next.diagnostics[0] ?? null);
  };

  useEffect(() => {
    let disposed = false;
    setSnapshot(null);
    setDiagnostic(null);
    void invoke<Snapshot>("open_computed_view", { projectPath, relativePath: path }).then(next => {
      if (!disposed) install(next);
    }).catch(error => {
      if (!disposed) setDiagnostic(errorDiagnostic(error));
    });
    return () => { disposed = true; };
  }, [projectPath, path]);

  const request = () => ({ name: name.trim(), table: table.trim(), columns });
  const previewEdit = async () => {
    if (!snapshot) return;
    setBusy(true); setDiagnostic(null);
    try {
      const next = await invoke<Preview>("preview_computed_view", {
        projectPath, relativePath: path, baseSource: snapshot.baseSource, request: request(),
      });
      setPreview(next);
      setDiagnostic(next.validation.diagnostics[0] ?? null);
    } catch (error) { setDiagnostic(errorDiagnostic(error)); }
    finally { setBusy(false); }
  };
  const save = async () => {
    if (!snapshot || !canWrite) return;
    setBusy(true); setDiagnostic(null);
    try {
      const result = await invoke<SaveReport>("save_computed_view", {
        projectPath, relativePath: path, baseSource: snapshot.baseSource,
        baseContentIdentity: snapshot.baseContentIdentity, request: request(),
      });
      if (result.status === "success" && result.snapshot) {
        install(result.snapshot); await onSaved();
      } else setDiagnostic(result.diagnostic);
    } catch (error) { setDiagnostic(errorDiagnostic(error)); }
    finally { setBusy(false); }
  };
  const remove = async () => {
    if (!snapshot || !canWrite || !window.confirm(`Remove Computed View ${snapshot.name}?`)) return;
    setBusy(true); setDiagnostic(null);
    try {
      const result = await invoke<{ status: string; diagnostic: Diagnostic | null }>("remove_computed_view", {
        projectPath, relativePath: path, baseSource: snapshot.baseSource,
        baseContentIdentity: snapshot.baseContentIdentity, confirmed: true,
      });
      if (result.status === "success") await onSaved();
      else setDiagnostic(result.diagnostic);
    } catch (error) { setDiagnostic(errorDiagnostic(error)); }
    finally { setBusy(false); }
  };

  if (!snapshot) return <section className="placeholder-editor"><Spin tip="Opening Computed View"><div style={{ minHeight: 80 }} /></Spin>{diagnostic && <Diagnostic diagnostic={diagnostic} />}</section>;
  return <section className="source-editor computed-view-editor" aria-label="Computed View editor">
    <div className="editor-heading"><div><h2>{snapshot.name}</h2><Tag>authoring-only</Tag><Tag>{snapshot.table}</Tag></div><Space>
      <Button onClick={() => void previewEdit()} disabled={busy}>Preview</Button>
      <Button type="primary" onClick={() => void save()} loading={busy} disabled={!canWrite}>Save</Button>
      <Button danger icon={<Trash2 size={14} />} onClick={() => void remove()} loading={busy} disabled={!canWrite}>Remove</Button>
    </Space></div>
    <Form layout="vertical" disabled={busy}>
      <Form.Item label="View name"><Input value={name} onChange={event => setName(event.target.value)} /></Form.Item>
      <Form.Item label="Target Table"><Input value={table} onChange={event => setTable(event.target.value)} /></Form.Item>
      <h3>Computed columns</h3>
      {columns.map((column, index) => <div className="computed-view-column" key={index}>
        <Form.Item label="Name"><Input aria-label={`Computed column ${index + 1} name`} value={column.name} onChange={event => setColumns(columns.map((item, i) => i === index ? { ...item, name: event.target.value } : item))} /></Form.Item>
        <Form.Item label="Expression"><Input.TextArea aria-label={`Computed column ${index + 1} expression`} value={column.expression} onChange={event => setColumns(columns.map((item, i) => i === index ? { ...item, expression: event.target.value } : item))} autoSize={{ minRows: 1, maxRows: 5 }} /></Form.Item>
        <Button aria-label={`Remove computed column ${index + 1}`} icon={<Trash2 size={14} />} onClick={() => setColumns(columns.filter((_, i) => i !== index))}>Remove column</Button>
      </div>)}
      <Button icon={<Plus size={14} />} onClick={() => setColumns([...columns, { name: "", expression: "" }])}>Add computed column</Button>
    </Form>
    {preview && <section className="preview-panel" aria-label="Computed View preview"><h3>Candidate source</h3><pre>{preview.candidateSource}</pre></section>}
    {diagnostic && <Diagnostic diagnostic={diagnostic} />}
  </section>;
}

function errorDiagnostic(error: unknown): Diagnostic {
  if (error && typeof error === "object" && "diagnostic" in error) return (error as { diagnostic: Diagnostic }).diagnostic;
  return { code: "E-VIEW-EDITOR", message: String(error) };
}

function Diagnostic({ diagnostic }: { diagnostic: Diagnostic }) {
  return <Alert role="alert" type="error" showIcon title={`${diagnostic.code}${diagnostic.line ? ` @ ${diagnostic.line}:${diagnostic.column ?? 0}` : ""}`} description={diagnostic.message} />;
}
