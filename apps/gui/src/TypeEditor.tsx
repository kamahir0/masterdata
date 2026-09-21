import { useEffect, useRef, useState } from "react";
import { Alert, Button, Checkbox, Form, Input, InputNumber, Select, Space, Spin, Table, Tabs, Tag } from "antd";
import { invoke } from "@tauri-apps/api/core";
import type { MigrationResult } from "./TableEditor";
import { TypedInitializer, initializerJson, resetInitializer } from "./TypedInitializer";
import type { AuthoringValue, ResolvedAuthoringType } from "./data-editor-types";

type Field = { key: number; name: string; type: string; nullable: boolean; array: boolean };
type Snapshot = { path: string; name: string; category: string; underlying: string | null; conversions: { fromUnderlyingImplicit: boolean; toUnderlyingImplicit: boolean } | null; members: { name: string; value: string }[]; fields: Field[]; fieldTypes: string[]; initializerShapes: Record<string, ResolvedAuthoringType> };
type Diagnostic = { code: string; message: string; source?: string; schemaPath?: string; schema_path?: string };
type Plan = { token: string; target: string; operation: string; selector: string; destructive: boolean; affectedOccurrenceCount: number; files: { path: string; before: string; after: string }[]; diagnostics: Diagnostic[] };
type Operation = "conversions" | "add" | "rename" | "drop";
const diagnosticText = (d: Diagnostic) => [d.source, d.schemaPath ?? d.schema_path, `${d.code}: ${d.message}`].filter(Boolean).join(" · ");
const message = (error: unknown) => error && typeof error === "object" && "diagnostic" in error ? diagnosticText((error as { diagnostic: Diagnostic }).diagnostic) : String(error);
export default function TypeEditor({ projectPath, path, canWrite, dirtyPaths, beginApply, onResult, endApply }: {
  projectPath: string; path: string; canWrite: boolean; dirtyPaths: string[];
  beginApply: (paths: string[]) => boolean; onResult: (result: MigrationResult) => Promise<void>; endApply: () => void;
}) {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [selected, setSelected] = useState("");
  const [operation, setOperation] = useState<Operation | null>(null);
  const [name, setName] = useState("");
  const [value, setValue] = useState("");
  const [key, setKey] = useState<number | null>(0);
  const [type, setType] = useState("int");
  const [modifier, setModifier] = useState("required");
  const [from, setFrom] = useState(false);
  const [to, setTo] = useState(false);
  const [hasInitializer, setHasInitializer] = useState(false);
  const [initializer, setInitializer] = useState<AuthoringValue>(resetInitializer());
  const [plan, setPlan] = useState<Plan | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<MigrationResult | null>(null);
  const revision = useRef(0);
  const inFlight = useRef(false);
  const mounted = useRef(true);
  useEffect(() => { if (plan) document.getElementById("type-plan-summary")?.focus(); }, [plan]);
  useEffect(() => {
    mounted.current = true;
    let disposed = false;
    setSnapshot(null); setOperation(null); setPlan(null); setResult(null); setError(null);
    void invoke<Snapshot>("open_type", { projectPath, relativePath: path }).then(next => {
      if (!disposed) { setSnapshot(next); setSelected(next.members[0]?.name ?? next.fields[0]?.name ?? ""); }
    }).catch(error => { if (!disposed) setError(message(error)); });
    return () => { disposed = true; mounted.current = false; revision.current += 1; };
  }, [projectPath, path]);
  const change = (fn: () => void) => { revision.current += 1; fn(); setPlan(null); setConfirmed(false); setError(null); setResult(null); };
  const custom = snapshot?.category === "Custom Type";
  const protectedMember = snapshot?.category === "Flags Enum" && selected === "None";
  const start = (op: Operation) => {
    change(() => { setOperation(op); setName(op === "rename" ? selected : ""); setValue(""); setFrom(snapshot?.conversions?.fromUnderlyingImplicit ?? false); setTo(snapshot?.conversions?.toUnderlyingImplicit ?? false); if (op === "add") { setHasInitializer(false); setInitializer(resetInitializer()); } });
    window.requestAnimationFrame(() => document.getElementById(op === "drop" ? "type-plan" : op === "conversions" ? "type-from" : "type-name")?.focus());
  };
  const cancel = () => { const old = operation; change(() => setOperation(null)); window.requestAnimationFrame(() => document.getElementById(`type-${old}-action`)?.focus()); };
  const getPlan = async () => {
    if (inFlight.current || !snapshot || !operation) return;
    const current = revision.current;
    inFlight.current = true; setBusy(true); setError(null); setPlan(null);
    const input = operation === "conversions" ? { operation, target: snapshot.name, fromUnderlyingImplicit: from, toUnderlyingImplicit: to }
      : operation === "add" ? custom ? { operation: "add_custom", target: snapshot.name, field: { key, name, type, nullable: modifier === "nullable", array: modifier === "array" }, initializer: hasInitializer ? initializerJson(initializer) : null }
        : { operation: "add_enum", target: snapshot.name, name, value }
        : { operation: `${operation}_${custom ? "custom" : "enum"}`, target: snapshot.name, [custom ? "field" : "member"]: selected, ...(operation === "rename" ? { newName: name } : {}) };
    try { const next = await invoke<Plan>("plan_type_migration", { projectPath, input }); if (mounted.current && current === revision.current) { setPlan(next); setResult(null); setConfirmed(false); } }
    catch (error) { if (mounted.current && current === revision.current) setError(message(error)); }
    finally { inFlight.current = false; if (mounted.current) setBusy(false); }
  };
  const apply = async () => {
    if (inFlight.current || !plan || !canWrite || (plan.destructive && !confirmed) || result?.state === "success") return;
    if (!beginApply(plan.files.map(file => file.path))) { setError("Affected files have unsaved changes or a Save in progress. Resolve them before Apply."); return; }
    inFlight.current = true; setBusy(true); setError(null);
    try {
      let outcome: MigrationResult;
      try { outcome = await invoke<MigrationResult>("apply_table_migration", { projectPath, token: plan.token, allowDestructive: plan.destructive && confirmed }); }
      catch (error) {
        if (error && typeof error === "object" && "diagnostic" in error) throw error;
        outcome = { state: "recovery_required", files: plan.files.map(file => file.path), diagnostic: { code: "E-MIGRATION-TRANSPORT", message: message(error) } };
      }
      if (mounted.current) setResult(outcome);
      await onResult(outcome);
      if (mounted.current && outcome.state === "success") {
        // Hide the old declaration while loading the committed authority.
        setSnapshot(null); setPlan(null); setOperation(null);
        const next = await invoke<Snapshot>("open_type", { projectPath, relativePath: path });
        if (mounted.current) setSnapshot(next);
      }
    } catch (error) { if (mounted.current) setError(message(error)); }
    finally { inFlight.current = false; endApply(); if (mounted.current) setBusy(false); }
  };
  const blocked = plan?.files.filter(file => dirtyPaths.includes(file.path)) ?? [];
  const stale = result?.state === "not_started" && /stale/i.test(result.diagnostic?.code + " " + result.diagnostic?.message);
  return <section className="table-editor" aria-label="Type Editor" tabIndex={0}>
    {error && <Alert role="alert" type="error" title={error} />}
    {!snapshot && !error && <Spin tip="Loading Type"><div style={{ minHeight: 80 }} /></Spin>}
    {snapshot && <>
      <h2>{snapshot.name} <Tag>{snapshot.category}</Tag></h2><p className="source-provenance">{snapshot.path}</p>
      {snapshot.underlying && <p>Underlying: {snapshot.underlying}</p>}
      {snapshot.conversions ? <>
        <p>fromUnderlyingImplicit: {String(snapshot.conversions.fromUnderlyingImplicit)} · toUnderlyingImplicit: {String(snapshot.conversions.toUnderlyingImplicit)}</p>
        <Button id="type-conversions-action" disabled={!canWrite || busy} onClick={() => start("conversions")}>Edit conversions</Button>
      </> : <>
        {custom ? <Table<Field> size="small" pagination={false} rowKey="name" dataSource={snapshot.fields}
          rowSelection={{ type: "radio", selectedRowKeys: [selected], onChange: keys => change(() => { setSelected(String(keys[0])); setOperation(null); }), getCheckboxProps: field => ({ disabled: busy, "aria-label": `Select field ${field.name}` }) }}
          columns={[{ title: "Key", dataIndex: "key" }, { title: "Field", dataIndex: "name" }, { title: "Type", dataIndex: "type" }, { title: "Modifier", render: (_, f) => f.array ? "Array" : f.nullable ? "Nullable" : "Required" }]} />
          : <Table size="small" pagination={false} rowKey="name" dataSource={snapshot.members}
            rowSelection={{ type: "radio", selectedRowKeys: [selected], onChange: keys => change(() => { setSelected(String(keys[0])); setOperation(null); }), getCheckboxProps: member => ({ disabled: busy, "aria-label": `Select member ${member.name}` }) }}
            columns={[{ title: "Member", dataIndex: "name" }, { title: "Numeric value", dataIndex: "value" }]} />}
        <Space><Button id="type-add-action" disabled={!canWrite || busy} onClick={() => start("add")}>Add {custom ? "Field" : "Member"}</Button>
          <Button id="type-rename-action" disabled={!selected || protectedMember || !canWrite || busy} onClick={() => start("rename")}>Rename {custom ? "Field" : "Member"}</Button>
          <Button id="type-drop-action" danger disabled={!selected || protectedMember || !canWrite || busy} onClick={() => start("drop")}>Drop {custom ? "Field" : "Member"}</Button></Space>
      </>}
      {operation && <Form layout="vertical" disabled={busy} className="migration-form">
        <h3>{operation} {snapshot.name}{operation !== "add" && operation !== "conversions" ? `.${selected}` : ""}</h3>
        {operation === "conversions" ? <Space><Checkbox id="type-from" checked={from} onChange={event => change(() => setFrom(event.target.checked))}>fromUnderlyingImplicit</Checkbox><Checkbox checked={to} onChange={event => change(() => setTo(event.target.checked))}>toUnderlyingImplicit</Checkbox></Space>
          : operation !== "drop" && <Form.Item label={custom ? "Field name" : "Member name"} htmlFor="type-name"><Input id="type-name" value={name} onChange={event => change(() => setName(event.target.value))} /></Form.Item>}
        {operation === "add" && (custom ? <>
          <Form.Item label="MessagePack key"><InputNumber aria-label="MessagePack key" value={key} onChange={value => change(() => setKey(value))} /></Form.Item>
          <Form.Item label="Field type"><Select aria-label="Field type" value={type} options={snapshot.fieldTypes.map(value => ({ value, label: value }))} onChange={value => change(() => { setType(value); setHasInitializer(false); setInitializer(resetInitializer()); })} /></Form.Item>
          <Form.Item label="Modifier"><Select aria-label="Field modifier" value={modifier} options={["required", "nullable", "array"].map(value => ({ value, label: value }))} onChange={value => change(() => { setModifier(value); setHasInitializer(false); setInitializer(resetInitializer()); })} /></Form.Item>
          <TypedInitializer
            typeName={type}
            modifier={modifier as "required" | "nullable" | "array"}
            shape={snapshot.initializerShapes[type] ?? null}
            enabled={hasInitializer}
            value={initializer}
            disabled={busy}
            onEnabledChange={enabled => change(() => { setHasInitializer(enabled); if (!enabled) setInitializer(resetInitializer()); })}
            onChange={value => change(() => setInitializer(value))}
          />
        </> : <Form.Item label="Numeric value" htmlFor="type-number"><Input id="type-number" value={value} onChange={event => change(() => setValue(event.target.value))} /></Form.Item>)}
        {operation === "drop" && <Alert type="warning" title={`Destructive: drop ${snapshot.name}.${selected}${custom ? " and its values" : ""}.`} />}
        <Space><Button id="type-plan" onClick={() => void getPlan()} loading={busy} disabled={!canWrite}>Plan / Re-plan</Button><Button onClick={cancel}>Cancel</Button></Space>
      </Form>}
      {plan && <section aria-label="Type Migration Plan" className="migration-plan">
        <h3 id="type-plan-summary" tabIndex={-1}>{plan.operation}: {plan.target}.{plan.selector}</h3>
        <p>{plan.files.length} affected files · {plan.affectedOccurrenceCount} affected value occurrences · {plan.destructive ? "Destructive" : "Non-destructive"}</p>
        {plan.diagnostics.length === 0 && <p>Migration validation passed.</p>}
        {plan.diagnostics.map((d, i) => <Alert key={i} title={diagnosticText(d)} type="error" />)}
        <Tabs items={plan.files.map(file => ({ key: file.path, label: file.path, children: <div className="migration-diff"><section><h4>Before</h4><pre>{file.before}</pre></section><section><h4>After</h4><pre>{file.after}</pre></section></div> }))} />
        {blocked.length > 0 && <Alert type="warning" title="Apply blocked by unsaved affected files" description={blocked.map(f => f.path).join(", ")} />}
        {plan.destructive && <Checkbox disabled={busy} checked={confirmed} onChange={event => setConfirmed(event.target.checked)}>I confirm dropping {plan.target}.{plan.selector}{custom ? " and its values" : ""}.</Checkbox>}
        <Button type="primary" danger={plan.destructive} loading={busy} disabled={!canWrite || stale || blocked.length > 0 || (plan.destructive && !confirmed) || result?.state === "success"} onClick={() => void apply()}>Apply reviewed Plan</Button>
      </section>}
    </>}
    {result && <Alert role="status" type={result.state === "success" ? "success" : "warning"} title={stale ? "Stale Plan — re-plan required" : result.state} description={<>{result.diagnostic?.message}{result.fileStates?.map(f => <p key={f.path}>{f.path}: {f.state}</p>)}{result.recoveryWorkspace && <p>Recovery workspace: {result.recoveryWorkspace}</p>}</>} />}
  </section>;
}
