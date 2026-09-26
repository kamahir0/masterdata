import { useEffect, useRef, useState } from "react";
import { Button, Checkbox, Select } from "antd";
import { invoke } from "@tauri-apps/api/core";
import { type Category, type Context, type CreationReport, type CreationRequest, diagnosticOf, uncertainCreations } from "./SourceCreation";

type Proposal = { request: CreationRequest; identity: string | null };
type Diagnostic = NonNullable<CreationReport["diagnostic"]>;
const labels: Record<Category, string> = { folder: "Folder", table: "Table", data: "Data", value_object: "Value Object", enum: "Enum", flags: "Flags Enum", custom_type: "Custom Type" };

export default function InlineSourceCreation({ projectPath, sourceRoot, sourceRootIndex, folder, category, contextTable, canWrite, onCancel, onAdvanced, onCreated }: {
  projectPath: string; sourceRoot: string; sourceRootIndex: number; folder: string; category: Category; contextTable: string;
  canWrite: boolean; onCancel: () => void; onAdvanced: (filename: string, identity: string | null) => void; onCreated: (report: CreationReport) => Promise<void>;
}) {
  const [filename, setFilename] = useState(category === "folder" ? "new-folder" : "new.yaml");
  const [table, setTable] = useState(contextTable);
  const [inlineRecords, setInlineRecords] = useState(true);
  const [context, setContext] = useState<Context | null>(null);
  const [proposal, setProposal] = useState<Proposal | null>(null);
  const [error, setError] = useState<Diagnostic | null>(null);
  const [busy, setBusy] = useState(false);
  const [uncertain, setUncertain] = useState(() => uncertainCreations.has(projectPath));
  const [created, setCreated] = useState<CreationReport | null>(null);
  const [recheckMessage, setRecheckMessage] = useState("");
  const revision = useRef(0);
  const inFlight = useRef(false);
  const input = useRef<HTMLInputElement>(null);
  const destination = folder ? `${folder}/${filename}` : filename;
  const resolvedRoot = context?.roots.find(root => root.index === sourceRootIndex)?.label ?? "";
  const intent = { sourceRoot: resolvedRoot, destination, category, table: table || null, inlineRecords };

  useEffect(() => {
    let active = true;
    void invoke<Context>("creation_context", { projectPath }).then(value => {
      if (!active) return;
      setContext(value);
      if (category === "data" && !contextTable && value.choices.tables.length === 1) setTable(value.choices.tables[0]);
    }).catch(reason => { if (active) setError(diagnosticOf(reason)); });
    input.current?.focus();
    input.current?.select();
    return () => { active = false; };
  }, [projectPath, category, contextTable]);

  useEffect(() => {
    const current = ++revision.current;
    setProposal(null);
    if (!resolvedRoot) return;
    if (category === "data" && !table) return;
    void invoke<Proposal>("default_creation_proposal", { intent }).then(value => {
      if (revision.current === current) { setProposal(value); setError(null); }
    }).catch(reason => { if (revision.current === current) setError(diagnosticOf(reason)); });
  }, [resolvedRoot, destination, category, table, inlineRecords]);

  const submit = async () => {
    if (inFlight.current || !canWrite || !resolvedRoot || uncertain || created || (category === "data" && !table)) return;
    inFlight.current = true; setBusy(true); setError(null); setRecheckMessage("");
    let submitted: CreationRequest | null = null;
    try {
      const candidate = await invoke<Proposal>("default_creation_proposal", { intent });
      submitted = candidate.request;
      const result = await invoke<CreationReport>("create_source", { projectPath, request: submitted });
      if (result.status === "success") { setCreated(result); await onCreated(result); }
      else if (result.status === "outcome_unknown") { uncertainCreations.set(projectPath, submitted); setUncertain(true); setError(result.diagnostic); }
      else setError(result.diagnostic);
    } catch (reason) {
      setError(diagnosticOf(reason));
      // A transport failure after submit may mean the exclusive commit completed.
      if (submitted && !(reason && typeof reason === "object" && "diagnostic" in reason)) {
        uncertainCreations.set(projectPath, submitted); setUncertain(true);
      }
    } finally { inFlight.current = false; setBusy(false); }
  };

  const recheck = async () => {
    if (inFlight.current) return;
    inFlight.current = true; setBusy(true);
    try {
      if (created) { await onCreated(created); return; }
      const request = uncertainCreations.get(projectPath);
      if (!request) return;
      const actual = await invoke<{ exists: boolean }>("recheck_creation", { projectPath, request });
      uncertainCreations.delete(projectPath); setUncertain(false); setError(null);
      setRecheckMessage(actual.exists ? "Destination exists. Review it in Explorer or choose another filename." : "Destination is absent. Creation can be retried.");
    } catch (reason) { setError(diagnosticOf(reason)); }
    finally { inFlight.current = false; setBusy(false); }
  };

  return <div className="inline-source-creation" aria-label={`New ${labels[category]}`}>
    <div className="inline-source-name">
      <span aria-hidden="true">{category === "folder" ? "▸" : "+"}</span>
      <input ref={input} aria-label={category === "folder" ? "New folder name" : "New source filename"} value={filename}
        onChange={event => setFilename(event.target.value)} disabled={busy || uncertain || !!created}
        onKeyDown={event => {
          event.stopPropagation();
          if (event.key === "Enter") { event.preventDefault(); void submit(); }
          if (event.key === "Escape" && !busy) { event.preventDefault(); onCancel(); }
        }} />
    </div>
    <div className="inline-source-details">
      <span>{labels[category]} · {sourceRoot}{folder ? `/${folder}` : ""}</span>
      {category !== "folder" && <span>Identity: {proposal?.identity ?? (category === "data" ? table || "Select a Table" : "…")}</span>}
      {category === "data" && <Select aria-label="Existing Table" size="small" value={table || undefined} placeholder="Select Table"
        options={(context?.choices.tables ?? []).map(value => ({ value, label: value }))} onChange={setTable} disabled={busy || uncertain} />}
      {category === "table" && <Checkbox checked={inlineRecords} onChange={event => setInlineRecords(event.target.checked)} disabled={busy || uncertain}>Records in this file</Checkbox>}
      {error && <span role="alert" className="inline-source-error">{error.code}: {error.message}</span>}
      {recheckMessage && <span role="status">{recheckMessage}</span>}
      <div className="inline-source-actions">
        <Button size="small" type="primary" onClick={() => void submit()} disabled={!canWrite || !proposal || uncertain || !!created} loading={busy}>Create</Button>
        {(uncertain || created) && <Button size="small" onClick={() => void recheck()} disabled={busy}>Recheck</Button>}
        <Button size="small" onClick={() => onAdvanced(filename, proposal?.identity ?? null)} disabled={busy || uncertain || !!created}>Advanced…</Button>
        <Button size="small" onClick={onCancel} disabled={busy}>Cancel</Button>
      </div>
    </div>
  </div>;
}
