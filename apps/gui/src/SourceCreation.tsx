import { useEffect, useRef, useState } from "react";
import { Alert, Button, Checkbox, Form, Input, InputNumber, Modal, Select, Space, Spin } from "antd";
import { ArrowDown, ArrowUp, Plus, Trash2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

type Field = { key: number | null; name: string; type: string; nullable: boolean; array: boolean };
type Member = { name: string; value: string };
type Secondary = { fields: string[]; nonUnique: boolean };
type Category = "folder" | "table" | "data" | "value_object" | "enum" | "flags" | "custom_type";
type Context = { roots: { index: number; label: string; folders: string[] }[]; choices: { fieldTypes: string[]; tables: string[]; valueObjectUnderlyings: string[]; enumUnderlyings: string[] } };
type CreationRequest = { sourceRoot: string; destination: string; artifact: Record<string, unknown> };
export type CreationReport = { status: "success" | "conflict" | "failure" | "outcome_unknown"; path: string; folder: boolean; diagnostic: { code: string; message: string; schema_path?: string; schemaPath?: string } | null };
// Keep uncertain commits across dialog Cancel/reopen within this workspace session.
// Otherwise closing a dialog would bypass GUI-CREATE-INT-011 before recheck.
const uncertainCreations = new Map<string, CreationRequest>();
const categories: { value: Category; label: string }[] = [
  { value: "folder", label: "Folder" }, { value: "table", label: "Table" }, { value: "data", label: "Data" },
  { value: "value_object", label: "Value Object" }, { value: "enum", label: "Enum" }, { value: "flags", label: "Flags Enum" }, { value: "custom_type", label: "Custom Type" },
];
const options = (values: string[]) => values.map(value => ({ value, label: value || "/" }));
const initialField = (): Field => ({ key: 0, name: "id", type: "int", nullable: false, array: false });
function move<T>(rows: T[], index: number, direction: number): T[] {
  const next = [...rows]; [next[index], next[index + direction]] = [next[index + direction], next[index]]; return next;
}
function diagnosticOf(error: unknown) {
  if (error && typeof error === "object" && "diagnostic" in error) return (error as { diagnostic: NonNullable<CreationReport["diagnostic"]> }).diagnostic;
  return { code: "E-CREATION-REQUEST", message: String(error) };
}

export default function SourceCreation({ projectPath, initialRootIndex, initialFolder, canWrite, onCancel, onCreated }: {
  projectPath: string; initialRootIndex: number; initialFolder: string; canWrite: boolean;
  onCancel: () => void; onCreated: (report: CreationReport) => Promise<void>;
}) {
  const [context, setContext] = useState<Context | null>(null);
  const [category, setCategory] = useState<Category>("table");
  const [root, setRoot] = useState("");
  const [folder, setFolder] = useState(initialFolder);
  const [filename, setFilename] = useState("new.yaml");
  const [name, setName] = useState("");
  const [table, setTable] = useState("");
  const [csharpName, setCsharpName] = useState("");
  const [underlying, setUnderlying] = useState("int");
  const [fromImplicit, setFromImplicit] = useState(false);
  const [toImplicit, setToImplicit] = useState(false);
  const [fields, setFields] = useState<Field[]>([initialField()]);
  const [primaryKey, setPrimaryKey] = useState<string[]>(["id"]);
  const [secondaryKeys, setSecondaryKeys] = useState<Secondary[]>([]);
  const [members, setMembers] = useState<Member[]>([{ name: "", value: "" }]);
  const [busy, setBusy] = useState(false);
  const inFlight = useRef(false);
  const [error, setError] = useState<CreationReport["diagnostic"]>(null);
  const [status, setStatus] = useState<CreationReport["status"] | null>(() => uncertainCreations.has(projectPath) ? "outcome_unknown" : null);
  const unknownRequest = useRef<CreationRequest | null>(uncertainCreations.get(projectPath) ?? null);
  const [recheckMessage, setRecheckMessage] = useState("");
  const [created, setCreated] = useState<CreationReport | null>(null);
  useEffect(() => {
    let disposed = false;
    void invoke<Context>("creation_context", { projectPath }).then(value => {
      if (disposed) return;
      setContext(value); setRoot((value.roots.find(root => root.index === initialRootIndex) ?? value.roots[0])?.label ?? "");
      window.requestAnimationFrame(() => document.getElementById("creation-category")?.focus());
    }).catch(error => { if (!disposed) setError(diagnosticOf(error)); });
    return () => { disposed = true; };
  }, [projectPath, initialRootIndex]);

  const request = (): CreationRequest => {
    let artifact: Record<string, unknown> = { category };
    if (category === "table") artifact = { category, table: name, csharpName: csharpName || null, fields, primaryKey: { fields: primaryKey }, secondaryKeys };
    if (category === "data") artifact = { category, table };
    if (category === "value_object") artifact = { category, name, underlying, conversions: { fromUnderlyingImplicit: fromImplicit, toUnderlyingImplicit: toImplicit } };
    if (category === "enum" || category === "flags") artifact = { category, name, underlying, members };
    if (category === "custom_type") artifact = { category, name, fields };
    return { sourceRoot: root, destination: folder ? `${folder}/${filename}` : filename, artifact };
  };
  const focusError = (diagnostic: NonNullable<CreationReport["diagnostic"]>) => {
    let id = "creation-name";
    if (/PATH|EXTENSION|CONFLICT|IO/.test(diagnostic.code)) id = "creation-filename";
    const fieldName = diagnostic.message.match(/field `([^`]+)`/)?.[1];
    const field = fields.findIndex(field => field.name === fieldName);
    if (field >= 0) id = `creation-field-${field}`;
    const member = (diagnostic.schema_path ?? diagnostic.schemaPath)?.match(/members\[(\d+)\]/)?.[1];
    if (member) id = `creation-member-value-${member}`;
    window.requestAnimationFrame(() => document.getElementById(id)?.focus());
  };
  const submit = async () => {
    if (inFlight.current || !canWrite || !context || unknownRequest.current || created) return;
    inFlight.current = true; setBusy(true); setError(null); setRecheckMessage("");
    const submitted = request();
    try {
      const result = await invoke<CreationReport>("create_source", { projectPath, request: submitted });
      setStatus(result.status);
      if (result.status === "success") { setCreated(result); await onCreated(result); }
      else {
        if (result.status === "outcome_unknown") { unknownRequest.current = submitted; uncertainCreations.set(projectPath, submitted); }
        setError(result.diagnostic);
        if (result.diagnostic) focusError(result.diagnostic);
      }
    } catch (error) {
      const diagnostic = diagnosticOf(error); setError(diagnostic); focusError(diagnostic);
      // A structured backend rejection is preflight-only. Transport failure may
      // have followed commit: require a recheck rather than blindly resubmit.
      if (!(error && typeof error === "object" && "diagnostic" in error)) { unknownRequest.current = submitted; uncertainCreations.set(projectPath, submitted); setStatus("outcome_unknown"); }
      else setStatus("failure");
    } finally { inFlight.current = false; setBusy(false); }
  };
  const recheck = async () => {
    if (inFlight.current) return;
    inFlight.current = true; setBusy(true);
    try {
      if (created) { await onCreated(created); return; }
      const actual = await invoke<{ exists: boolean; folder: boolean; source: string | null }>("recheck_creation", { projectPath, request: unknownRequest.current ?? request() });
      const refreshed = await invoke<Context>("creation_context", { projectPath });
      setContext(refreshed); unknownRequest.current = null; uncertainCreations.delete(projectPath); setStatus(actual.exists ? "conflict" : null); setError(null);
      setRecheckMessage(actual.exists ? "Destination exists. It will not be overwritten; choose another destination or Cancel." : "Destination is absent. You can review the inputs and Create again.");
    } catch (error) { setError(diagnosticOf(error)); }
    finally { inFlight.current = false; setBusy(false); }
  };
  return <Modal open width={920} focusable={{ focusTriggerAfterClose: false }} title="New source artifact" onCancel={onCancel} closable={!busy} keyboard={!busy} mask={{ closable: false }}
    footer={<Space><Button onClick={onCancel} disabled={busy}>Cancel</Button>
      {(status === "outcome_unknown" || created) && <Button onClick={() => void recheck()} loading={busy}>Recheck Workspace</Button>}
      <Button type="primary" onClick={() => void submit()} loading={busy} disabled={!context || !canWrite || status === "outcome_unknown" || !!created}>Create</Button></Space>}>
    {!context ? <Spin tip="Loading source context"><div style={{ minHeight: 60 }} /></Spin> :
      <Form layout="vertical" className="creation-form" disabled={busy || !!created}>
        <div className="creation-destination">
          <Form.Item label="Artifact type" htmlFor="creation-category"><Select id="creation-category" aria-label="Artifact type" value={category} options={categories} onChange={value => {
            setCategory(value); setError(null);
            setFilename(value === "folder" ? "new-folder" : "new.yaml");
            if (value === "flags") setMembers([{ name: "None", value: "0" }]);
          }} /></Form.Item>
          <Form.Item label="Source root"><Select aria-label="Source root" value={root} options={context.roots.map(root => ({ value: root.label, label: root.label }))} onChange={value => { setRoot(value); setFolder(""); }} /></Form.Item>
          <Form.Item label="Parent folder"><Select aria-label="Parent folder" value={folder} options={options(context.roots.find(candidate => candidate.label === root)?.folders ?? [])} onChange={setFolder} /></Form.Item>
          <Form.Item label={category === "folder" ? "Folder name" : "Filename (.yaml / .yml)"} htmlFor="creation-filename" required><Input id="creation-filename" value={filename} onChange={event => setFilename(event.target.value)} /></Form.Item>
        </div>
        <p className="creation-hint">Destination and domain identity are independent. Existing files are never overwritten.</p>
        {category !== "folder" && category !== "data" && <Form.Item label={category === "table" ? "Table identity" : "Type name"} htmlFor="creation-name" required><Input id="creation-name" value={name} onChange={event => setName(event.target.value)} /></Form.Item>}
        {category === "data" && <Form.Item label="Existing Table" required><Select aria-label="Existing Table" value={table || undefined} options={options(context.choices.tables)} onChange={setTable} placeholder="Select a Table" /><p>New Data documents start with empty records.</p></Form.Item>}
        {category === "table" && <Form.Item label="C# name (optional)"><Input aria-label="C# name" value={csharpName} onChange={event => setCsharpName(event.target.value)} /></Form.Item>}
        {(category === "table" || category === "custom_type") && <>
          <h3>Fields</h3>
          {fields.map((field, index) => <div className="creation-field" key={index}>
            <Form.Item label="MessagePack key"><InputNumber aria-label={`Field ${index + 1} key`} value={field.key} onChange={value => setFields(fields.map((field, i) => i === index ? { ...field, key: value } : field))} /></Form.Item>
            <Form.Item label="Name"><Input id={`creation-field-${index}`} aria-label={`Field ${index + 1} name`} value={field.name} onChange={event => setFields(fields.map((field, i) => i === index ? { ...field, name: event.target.value } : field))} /></Form.Item>
            <Form.Item label="Base type"><Select aria-label={`Field ${index + 1} type`} showSearch value={field.type} options={options(context.choices.fieldTypes)} onChange={value => setFields(fields.map((field, i) => i === index ? { ...field, type: value } : field))} /></Form.Item>
            <Form.Item label="Modifier"><Select aria-label={`Field ${index + 1} modifier`} value={field.array ? "array" : field.nullable ? "nullable" : "required"} options={options(["required", "nullable", "array"])} onChange={value => setFields(fields.map((field, i) => i === index ? { ...field, nullable: value === "nullable", array: value === "array" } : field))} /></Form.Item>
            <RowActions label={`Field ${index + 1}`} index={index} count={fields.length} reorder={direction => setFields(move(fields, index, direction))} remove={() => setFields(fields.filter((_, i) => i !== index))} />
          </div>)}
          <Button icon={<Plus size={14} />} onClick={() => setFields([...fields, { ...initialField(), key: fields.length ? Math.max(...fields.map(field => field.key ?? -1)) + 1 : 0, name: "" }])}>Add field</Button>
        </>}
        {category === "table" && <>
          <Form.Item label="Primary Key (ordered)" required><Select mode="multiple" aria-label="Primary Key" value={primaryKey} options={options(fields.map(field => field.name).filter(Boolean))} onChange={setPrimaryKey} /></Form.Item>
          <p className="creation-hint">Key order follows selection order. Remove and reselect a field to change its position.</p>
          <h3>Secondary Keys</h3>
          {secondaryKeys.map((key, index) => <div className="creation-secondary" key={index}>
            <Select mode="multiple" aria-label={`Secondary Key ${index + 1} fields`} value={key.fields} options={options(fields.map(field => field.name).filter(Boolean))} onChange={value => setSecondaryKeys(secondaryKeys.map((key, i) => i === index ? { ...key, fields: value } : key))} />
            <Checkbox checked={key.nonUnique} onChange={event => setSecondaryKeys(secondaryKeys.map((key, i) => i === index ? { ...key, nonUnique: event.target.checked } : key))}>Non-unique</Checkbox>
            <RowActions label={`Secondary Key ${index + 1}`} index={index} count={secondaryKeys.length} reorder={direction => setSecondaryKeys(move(secondaryKeys, index, direction))} remove={() => setSecondaryKeys(secondaryKeys.filter((_, i) => i !== index))} />
          </div>)}
          <Button onClick={() => setSecondaryKeys([...secondaryKeys, { fields: [], nonUnique: false }])}>Add secondary key</Button>
        </>}
        {(category === "value_object" || category === "enum" || category === "flags") && <Form.Item label="Underlying" required><Select aria-label="Underlying" value={underlying} options={options(category === "value_object" ? context.choices.valueObjectUnderlyings : context.choices.enumUnderlyings)} onChange={setUnderlying} /></Form.Item>}
        {category === "value_object" && <Space direction="vertical"><Checkbox checked={fromImplicit} onChange={event => setFromImplicit(event.target.checked)}>fromUnderlyingImplicit</Checkbox><Checkbox checked={toImplicit} onChange={event => setToImplicit(event.target.checked)}>toUnderlyingImplicit</Checkbox></Space>}
        {(category === "enum" || category === "flags") && <>
          <h3>Members · explicit integer values</h3>
          {category === "flags" && <p>Flags require None = 0 and valid atomic bits. Shared validation checks the declaration.</p>}
          {members.map((member, index) => <div className="creation-member" key={index}>
            <Input aria-label={`Member ${index + 1} name`} value={member.name} placeholder="Member name" onChange={event => setMembers(members.map((member, i) => i === index ? { ...member, name: event.target.value } : member))} />
            <Input id={`creation-member-value-${index}`} aria-label={`Member ${index + 1} value`} value={member.value} placeholder="Explicit integer" onChange={event => setMembers(members.map((member, i) => i === index ? { ...member, value: event.target.value } : member))} />
            <RowActions label={`Member ${index + 1}`} index={index} count={members.length} reorder={direction => setMembers(move(members, index, direction))} remove={() => setMembers(members.filter((_, i) => i !== index))} />
          </div>)}
          <Button onClick={() => setMembers([...members, { name: "", value: "" }])}>Add member</Button>
        </>}
      </Form>}
    {!canWrite && <Alert type="warning" title="Workspace write capability is unavailable." />}
    {error && <Alert role="alert" type="error" showIcon title={`${status ?? "failure"}: ${error.code}`} description={error.message} action={<Button onClick={() => focusError(error)}>Go to input</Button>} />}
    {created && <Alert type="info" title="Source created. Refresh the workspace to select it; do not submit again." />}
    {recheckMessage && <Alert role="status" type="info" title={recheckMessage} />}
  </Modal>;
}
function RowActions({ label, index, count, reorder, remove }: { label: string; index: number; count: number; reorder: (direction: number) => void; remove: () => void }) {
  return <Space.Compact className="creation-row-actions"><Button aria-label={`${label} up`} icon={<ArrowUp size={14} />} disabled={index === 0} onClick={() => reorder(-1)} /><Button aria-label={`${label} down`} icon={<ArrowDown size={14} />} disabled={index === count - 1} onClick={() => reorder(1)} /><Button aria-label={`Remove ${label}`} icon={<Trash2 size={14} />} onClick={remove} /></Space.Compact>;
}
