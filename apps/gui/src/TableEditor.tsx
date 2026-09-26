import { useEffect, useRef, useState } from "react";
import { Alert, Button, Checkbox, Dropdown, Form, Input, InputNumber, Modal, Select, Space, Spin, Table, Tabs, Tag } from "antd";
import { MoreHorizontal } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { TypedInitializer, initializerJson, resetInitializer } from "./TypedInitializer";
import type { AuthoringValue, ResolvedAuthoringType } from "./data-editor-types";
import { migrationApplyDisabled, migrationBlockedFiles, migrationDestructiveAuthorization } from "./authoring-workflow";

type Field = { key:number; name:string; type:string; nullable:boolean; array:boolean };
type Reference = { name:string; csharpName?:string|null; effectiveCsharpName?:string; sourceFields:string[]; targetTable:string; targetFields:string[]; targetKeyKind?:"primary"|"secondary"; cardinality?:"single"|"many"; optionality?:"required"|"nullable" };
type Snapshot = { path:string; schema:{ table:string; fields:Field[]; primaryKey:{fields:string[]}; secondaryKeys:{fields:string[];nonUnique:boolean}[]; references?:Reference[] }; fieldTypes:string[]; initializerShapes:Record<string,ResolvedAuthoringType>; references?:Reference[]; referenceDiagnostics?:{code:string;message:string;source?:string;schemaPath?:string;valuePath?:string;recordIdentity?:string}[] };
type Plan = { token:string; table:string; operation:string; field:string; destructive:boolean; affectedRecordCount:number; files:{path:string;before:string;after:string}[]; diagnostics:{code:string;message:string}[] };
type ReferenceDraft = { operation:"add_reference"|"edit_reference"|"remove_reference"; selectedName:string; name:string; csharpName:string; sourceFields:string; targetTable:string; targetFields:string };
export type MigrationResult = { state:string; files:string[]; fileStates?:{path:string;state:string}[]; diagnostic?:{code:string;message:string}|null; recoveryWorkspace?:string|null };
const message = (error:unknown):string => {
  if (!error || typeof error !== "object" || !("diagnostic" in error)) return String(error);
  const diagnostic = (error as {diagnostic:{code:string;message:string;source?:string;schemaPath?:string;schema_path?:string}}).diagnostic;
  return [diagnostic.source, diagnostic.schemaPath ?? diagnostic.schema_path, `${diagnostic.code}: ${diagnostic.message}`].filter(Boolean).join(" · ");
};
export type TableEditorProps = { projectPath:string; path:string; canWrite:boolean; dirtyPaths:string[];
  beginApply:(paths:string[])=>boolean; onResult:(result:MigrationResult)=>Promise<void>; endApply:()=>void;
  onOverview?:()=>void;
  onCreateData?:()=>void;
  embedded?:boolean;
  direct?: boolean;
};

export default function TableEditor(props: TableEditorProps) {
  if (props.direct) return <SpreadsheetTableEditor {...props} />;
  return <LegacyTableEditor {...props} />;
}

function LegacyTableEditor({projectPath,path,canWrite,dirtyPaths,beginApply,onResult,endApply,onOverview,onCreateData,embedded=false}: TableEditorProps) {
  const [snapshot,setSnapshot]=useState<Snapshot|null>(null);
  const [selected,setSelected]=useState("");
  const [operation,setOperation]=useState<"add"|"rename"|"drop"|null>(null);
  const [referenceOperation,setReferenceOperation]=useState<"add_reference"|"edit_reference"|"remove_reference"|null>(null);
  const [selectedReference,setSelectedReference]=useState("");
  const [referenceName,setReferenceName]=useState("");const [referenceCsharpName,setReferenceCsharpName]=useState("");const [referenceSourceFields,setReferenceSourceFields]=useState("");const [referenceTargetTable,setReferenceTargetTable]=useState("");const [referenceTargetFields,setReferenceTargetFields]=useState("");
  const [name,setName]=useState("");const [key,setKey]=useState<number|null>(0);const [type,setType]=useState("int");const [modifier,setModifier]=useState("required");
  const [hasInitializer,setHasInitializer]=useState(false);const [initializer,setInitializer]=useState<AuthoringValue>(resetInitializer());
  const [plan,setPlan]=useState<Plan|null>(null);const [confirmed,setConfirmed]=useState(false);
  const [busy,setBusy]=useState(false);const [error,setError]=useState<string|null>(null);const [result,setResult]=useState<MigrationResult|null>(null);
  const revision=useRef(0);const inFlight=useRef(false);const mounted=useRef(true);
  const actionOrigin=useRef<HTMLElement|null>(null);
  useEffect(()=>{if(plan)document.getElementById("migration-plan-summary")?.focus();},[plan]);
  useEffect(()=>{ mounted.current=true; let disposed=false;
    void invoke<Snapshot>("open_table",{projectPath,relativePath:path}).then(value=>{if(!disposed){setSnapshot(value);setSelected(value.schema.fields[0]?.name??"");}}).catch(error=>{if(!disposed)setError(message(error));});
    return ()=>{disposed=true;mounted.current=false;revision.current+=1;};
  },[projectPath,path]);
  const change=(fn:()=>void)=>{revision.current+=1;fn();setPlan(null);setConfirmed(false);setError(null);setResult(null);};
  const start=(value:"add"|"rename"|"drop", target=selected, origin?:HTMLElement|null)=>{actionOrigin.current=origin??document.activeElement as HTMLElement;change(()=>{setSelected(target);setOperation(value);setReferenceOperation(null);setName(value==="rename"?target:"");setKey(snapshot?.schema.fields.length?Math.max(...snapshot.schema.fields.map(field=>field.key))+1:0);if(value==="add"){setHasInitializer(false);setInitializer(resetInitializer());}});window.requestAnimationFrame(()=>document.getElementById(value==="drop"?"migration-plan":"migration-name")?.focus());};
  const startReference=(value:"add_reference"|"edit_reference"|"remove_reference", reference?:Reference)=>{actionOrigin.current=document.activeElement as HTMLElement;change(()=>{setOperation(null);setReferenceOperation(value);setSelectedReference(reference?.name??"");setReferenceName(reference?.name??"");setReferenceCsharpName(reference?.csharpName??"");setReferenceSourceFields(reference?.sourceFields.join(", ")??"");setReferenceTargetTable(reference?.targetTable??"");setReferenceTargetFields(reference?.targetFields.join(", ")??"");});window.requestAnimationFrame(()=>document.getElementById(value==="remove_reference"?"migration-plan":"reference-name")?.focus());};
  const cancel=()=>{change(()=>{setOperation(null);setReferenceOperation(null);});window.requestAnimationFrame(()=>actionOrigin.current?.focus());};
  const getPlan=async()=>{
    if(inFlight.current||!snapshot||(!operation&&!referenceOperation))return;
    const current=revision.current;inFlight.current=true;setBusy(true);setError(null);
    const input=referenceOperation?{operation:referenceOperation,table:snapshot.schema.table,...(referenceOperation==="edit_reference"||referenceOperation==="remove_reference"?{name:selectedReference}:{}),...(referenceOperation!=="remove_reference"?{reference:{name:referenceName,csharpName:referenceCsharpName||null,fields:referenceSourceFields.split(",").map(value=>value.trim()).filter(Boolean),target:{table:referenceTargetTable,fields:referenceTargetFields.split(",").map(value=>value.trim()).filter(Boolean)}}}: {})}:operation==="add"?{operation,table:snapshot.schema.table,field:{key,name,type,nullable:modifier==="nullable",array:modifier==="array"},initializer:hasInitializer?initializerJson(initializer):null}:operation==="rename"?{operation,table:snapshot.schema.table,field:selected,newName:name}:{operation,table:snapshot.schema.table,field:selected};
    try {const next=await invoke<Plan>("plan_table_migration",{projectPath,input});if(mounted.current&&current===revision.current){setPlan(next);setResult(null);setConfirmed(false);}}
    catch(error){if(mounted.current&&current===revision.current)setError(message(error));}
    finally{inFlight.current=false;if(mounted.current)setBusy(false);}
  };
  const apply=async()=>{
    if(inFlight.current||!plan||!canWrite||(plan.destructive&&!confirmed)||result?.state==="success")return;
    if(!beginApply(plan.files.map(file=>file.path))){setError("Affected files have unsaved changes or a Save in progress. Resolve them before Apply.");return;}
    inFlight.current=true;setBusy(true);setError(null);
    try {
      let outcome:MigrationResult;
      try {outcome=await invoke<MigrationResult>("apply_table_migration",{projectPath,token:plan.token,allowDestructive: migrationDestructiveAuthorization(plan.destructive, confirmed)});}
      catch(error){
        if(error&&typeof error==="object"&&"diagnostic" in error)throw error;
        outcome={state:"recovery_required",files:plan.files.map(file=>file.path),diagnostic:{code:"E-MIGRATION-TRANSPORT",message:message(error)}};
      }
      if(mounted.current)setResult(outcome);
      await onResult(outcome);
      if(mounted.current&&outcome.state==="success"){
        const next=await invoke<Snapshot>("open_table",{projectPath,relativePath:path});
        if(mounted.current){setSnapshot(next);setOperation(null);setReferenceOperation(null);setPlan(null);}
      }
    }catch(error){if(mounted.current)setError(message(error));}
    finally{inFlight.current=false;endApply();if(mounted.current)setBusy(false);}
  };
  const stale = result?.state === "not_started" && !!result.diagnostic?.message.includes("stale");
  const blocked = migrationBlockedFiles(plan?.files ?? [], dirtyPaths);
  return <section className="table-editor" aria-label="Table Editor">
    {error&&!operation&&!referenceOperation&&<Alert role="alert" type="error" title={error}/>}
    {!snapshot&&!error&&<Spin tip="Loading Table"><div style={{minHeight:80}}/></Spin>}
    {snapshot&&<>
      {embedded ? <div className="table-context-line"><strong>{snapshot.schema.table}</strong><span>{snapshot.path}</span></div>
        : <div className="typed-editor-heading"><div><h2>{snapshot.schema.table} <Tag>Table</Tag></h2><p className="source-provenance">{snapshot.path}</p></div><Space>{onOverview&&<Button onClick={onOverview}>Table Overview</Button>}{onCreateData&&<Button onClick={onCreateData}>New data file</Button>}</Space></div>}
      <div className="typed-list-heading"><h3>Fields <span>{snapshot.schema.fields.length}</span></h3><Button id="table-add" disabled={!canWrite||busy} onClick={event=>start("add","",event.currentTarget)}>Add Field</Button></div>
      <Table<Field> size="small" pagination={false} rowKey="name" dataSource={snapshot.schema.fields}
        columns={[{title:"Key",dataIndex:"key"},{title:"Field",dataIndex:"name"},{title:"Type",dataIndex:"type"},{title:"Modifier",render:(_,field)=>field.array?"Array":field.nullable?"Nullable":"Required"},{title:"Indexes",render:(_,field)=><>{snapshot.schema.primaryKey?.fields.includes(field.name)&&<Tag>Primary Key</Tag>}{snapshot.schema.secondaryKeys?.map((key,index)=>key.fields.includes(field.name)?<Tag key={index}>Secondary {index+1}{key.nonUnique?" · non-unique":""}</Tag>:null)}</>},{title:"",key:"actions",width:46,render:(_,field)=><Dropdown menu={{items:[{key:"rename",label:"Rename Field",disabled:!canWrite||busy,onClick:()=>start("rename",field.name,document.querySelector<HTMLElement>(`[data-field-action="${CSS.escape(field.name)}"]`))},{key:"drop",label:"Drop Field",danger:true,disabled:!canWrite||busy,onClick:()=>start("drop",field.name,document.querySelector<HTMLElement>(`[data-field-action="${CSS.escape(field.name)}"]`))}]}} trigger={["click"]}><Button type="text" size="small" data-field-action={field.name} aria-label={`Actions for field ${field.name}`} icon={<MoreHorizontal size={16}/>} /></Dropdown>}]} />
      <p>Primary Key: {snapshot.schema.primaryKey?.fields.join(" → ")}</p>
      {snapshot.schema.secondaryKeys?.map((key,index)=><p key={index}>Secondary {index+1}: {key.fields.join(" → ")} {key.nonUnique?"(non-unique)":"(unique)"}</p>)}
      <section aria-label="References" className="table-references">
        <h3>References</h3>
        {(snapshot.references ?? snapshot.schema.references ?? []).map(reference=><div key={reference.name} className="table-reference">
          <strong>{reference.name}</strong>
          <span> · {reference.sourceFields.join(" → ")} → {reference.targetTable}.{reference.targetFields.join(" → ")}</span>
          <Tag>{reference.csharpName ? `exact: ${reference.csharpName}` : "implicit helper"}</Tag>
          <Tag>{reference.targetKeyKind ?? "unresolved target"}</Tag>
          <Tag>{reference.cardinality ?? "unresolved cardinality"}</Tag>
          <Tag>{reference.optionality ?? "unresolved optionality"}</Tag>
          <Tag>{reference.effectiveCsharpName ?? "unresolved helper name"}</Tag>
          <Button size="small" disabled={!canWrite||busy} onClick={()=>startReference("edit_reference",reference)}>Edit</Button>
          <Button size="small" danger disabled={!canWrite||busy} onClick={()=>startReference("remove_reference",reference)}>Remove</Button>
        </div>)}
        {(snapshot.referenceDiagnostics ?? []).map((diagnostic,index)=><Alert key={`${diagnostic.code}-${index}`} type="error" title={`${diagnostic.code}: ${diagnostic.message}`} description={[diagnostic.source,diagnostic.schemaPath,diagnostic.valuePath,diagnostic.recordIdentity].filter(Boolean).join(" · ")}/>) }
        {(snapshot.references ?? snapshot.schema.references ?? []).length===0&&<p>No References declared.</p>}
        <Button disabled={!canWrite||busy} onClick={()=>startReference("add_reference")}>Add Reference</Button>
      </section>
      <Modal open={operation!==null||referenceOperation!==null} title="Schema change" onCancel={cancel} footer={null} width={780} destroyOnHidden>
      {error&&<Alert role="alert" type="error" title={error}/>}
      {referenceOperation&&<Form layout="vertical" disabled={busy} className="migration-form">
        <h3>{referenceOperation==="add_reference"?"Add Reference":referenceOperation==="edit_reference"?`Edit Reference ${selectedReference}`:`Remove Reference ${selectedReference}`}</h3>
        {referenceOperation!=="remove_reference"&&<>
          <Form.Item label="Reference name" htmlFor="reference-name"><Input id="reference-name" value={referenceName} onChange={event=>change(()=>setReferenceName(event.target.value))}/></Form.Item>
          <Form.Item label="C# helper name (optional, exact)" htmlFor="reference-csharp-name"><Input id="reference-csharp-name" value={referenceCsharpName} onChange={event=>change(()=>setReferenceCsharpName(event.target.value))}/></Form.Item>
          <Form.Item label="Source fields (ordered, comma-separated)" htmlFor="reference-source-fields"><Input id="reference-source-fields" value={referenceSourceFields} onChange={event=>change(()=>setReferenceSourceFields(event.target.value))}/></Form.Item>
          <Form.Item label="Target table" htmlFor="reference-target-table"><Input id="reference-target-table" value={referenceTargetTable} onChange={event=>change(()=>setReferenceTargetTable(event.target.value))}/></Form.Item>
          <Form.Item label="Target key fields (ordered, comma-separated)" htmlFor="reference-target-fields"><Input id="reference-target-fields" value={referenceTargetFields} onChange={event=>change(()=>setReferenceTargetFields(event.target.value))}/></Form.Item>
        </>}
        {referenceOperation==="remove_reference"&&<Alert type="warning" title={`Remove Reference ${selectedReference}.`}/>}
        <Space><Button id="migration-plan" onClick={()=>void getPlan()} loading={busy} disabled={!canWrite}>Plan / Re-plan</Button><Button onClick={cancel}>Cancel</Button></Space>
      </Form>}
      {operation&&<Form layout="vertical" disabled={busy} className="migration-form">
        <h3>{operation==="add"?"Add Field":`${operation==="rename"?"Rename":"Drop"} ${selected}`}</h3>
        {operation!=="drop"&&<Form.Item label="Field name" htmlFor="migration-name"><Input id="migration-name" value={name} onChange={event=>change(()=>setName(event.target.value))}/></Form.Item>}
        {operation==="add"&&<>
          <Form.Item label="MessagePack key"><InputNumber aria-label="MessagePack key" value={key} onChange={value=>change(()=>setKey(value))}/></Form.Item>
          <Form.Item label="Field type"><Select aria-label="Field type" value={type} options={snapshot.fieldTypes.map(value=>({value,label:value}))} onChange={value=>change(()=>{setType(value);setHasInitializer(false);setInitializer(resetInitializer());})}/></Form.Item>
          <Form.Item label="Modifier"><Select aria-label="Field modifier" value={modifier} options={["required","nullable","array"].map(value=>({value,label:value}))} onChange={value=>change(()=>{setModifier(value);setHasInitializer(false);setInitializer(resetInitializer());})}/></Form.Item>
          <TypedInitializer
            typeName={type}
            modifier={modifier as "required"|"nullable"|"array"}
            shape={snapshot.initializerShapes[type]??null}
            enabled={hasInitializer}
            value={initializer}
            disabled={busy}
            onEnabledChange={enabled=>change(()=>{setHasInitializer(enabled);if(!enabled)setInitializer(resetInitializer());})}
            onChange={value=>change(()=>setInitializer(value))}
          />
        </>}
        {operation==="drop"&&<Alert type="warning" title={`Destructive: remove ${snapshot.schema.table}.${selected} and its values from every record.`}/>}
        <Space><Button id="migration-plan" onClick={()=>void getPlan()} loading={busy} disabled={!canWrite}>Plan / Re-plan</Button><Button onClick={cancel}>Cancel</Button></Space>
      </Form>}
      {plan&&<section aria-label="Migration Plan" className="migration-plan">
        <h3 id="migration-plan-summary" tabIndex={-1}>{plan.operation}: {plan.table}.{plan.field}</h3><p>{plan.files.length} affected files · {plan.affectedRecordCount} affected records · {plan.destructive?"Destructive":"Non-destructive"}</p>
        {plan.diagnostics.map((diagnostic,index)=><Alert key={index} title={`${diagnostic.code}: ${diagnostic.message}`} type="error"/>)}
        <Tabs items={plan.files.map(file=>({key:file.path,label:file.path,children:<div className="migration-diff"><section><h4>Before</h4><pre>{file.before}</pre></section><section><h4>After</h4><pre>{file.after}</pre></section></div>}))}/>
        {blocked.length>0&&<Alert type="warning" title="Apply blocked by unsaved affected files" description={blocked.map(file=>file.path).join(", ")}/>}
        {plan.destructive&&<Checkbox disabled={busy} checked={confirmed} onChange={event=>setConfirmed(event.target.checked)}>I confirm dropping {plan.table}.{plan.field} and its record values.</Checkbox>}
        <Button type="primary" danger={plan.destructive} loading={busy} disabled={migrationApplyDisabled({ canWrite, stale, blocked: blocked.length > 0, destructive: plan.destructive, confirmed, succeeded: result?.state === "success" })} onClick={()=>void apply()}>Apply reviewed Plan</Button>
      </section>}
      {result&&<Alert role="status" type={result.state==="success"?"success":"warning"} title={result.state==="not_started"&&result.diagnostic?.message.includes("stale")?"Stale Plan — re-plan required":result.state} description={result.diagnostic?.message}/>}
      </Modal>
      {result&&!operation&&!referenceOperation&&<Alert role="status" type={result.state==="success"?"success":"warning"} title={result.state==="not_started"&&result.diagnostic?.message.includes("stale")?"Stale Plan — re-plan required":result.state} description={result.diagnostic?.message}/>}
    </>}
  </section>;
}

function SpreadsheetTableEditor({projectPath,path,canWrite,dirtyPaths,beginApply,onResult,endApply,onOverview,onCreateData,embedded=false}: TableEditorProps) {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [draftNames, setDraftNames] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<MigrationResult | null>(null);
  const [lastPlan, setLastPlan] = useState<Plan | null>(null);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [dropTarget, setDropTarget] = useState<string | null>(null);
  const [referenceDraft, setReferenceDraft] = useState<ReferenceDraft | null>(null);
  const mounted = useRef(true);
  const lastNameCommit = useRef<string | null>(null);
  const skipNameBlur = useRef<string | null>(null);
  const lastInput = useRef<Record<string, unknown> | null>(null);

  const load = async () => {
    const next = await invoke<Snapshot>("open_table", { projectPath, relativePath: path });
    if (!mounted.current) return;
    setSnapshot(next);
    setDraftNames(Object.fromEntries(next.schema.fields.map((field) => [field.name, field.name])));
  };

  useEffect(() => {
    mounted.current = true;
    void load().catch((next) => { if (mounted.current) setError(message(next)); });
    return () => { mounted.current = false; };
  }, [projectPath, path]);

  const execute = async (input: Record<string, unknown>, destructive = false) => {
    if (!snapshot || busy || !canWrite) return false;
    lastInput.current = input;
    setBusy(true);
    setError(null);
    setResult(null);
    setLastPlan(null);
    let applyStarted = false;
    try {
      const plan = await invoke<Plan>("plan_table_migration", { projectPath, input });
      if (mounted.current) setLastPlan(plan);
      const affected = plan.files.map((file) => file.path);
      const blocked = migrationBlockedFiles(plan.files, dirtyPaths);
      if (blocked.length > 0) {
        setError(`Affected files have unsaved changes: ${blocked.map((file) => file.path).join(", ")}`);
        return false;
      }
      if (!beginApply(affected)) {
        setError("Affected files have unsaved changes or a Save in progress. Resolve them before editing the schema.");
        return false;
      }
      applyStarted = true;
      let outcome: MigrationResult;
      try {
        outcome = await invoke<MigrationResult>("apply_table_migration", {
          projectPath,
          token: plan.token,
          allowDestructive: migrationDestructiveAuthorization(destructive || plan.destructive, destructive || plan.destructive),
        });
      } catch (next) {
        if (next && typeof next === "object" && "diagnostic" in next) throw next;
        outcome = {
          state: "recovery_required",
          files: affected,
          diagnostic: { code: "E-MIGRATION-TRANSPORT", message: message(next) },
        };
      }
      if (mounted.current) setResult(outcome);
      await onResult(outcome);
      if (outcome.state === "success") {
        await load();
        if (mounted.current) {
          setDropTarget(null);
          setLastPlan(null);
          lastInput.current = null;
        }
      }
      return outcome.state === "success";
    } catch (next) {
      if (mounted.current) setError(message(next));
      return false;
    } finally {
      if (applyStarted) endApply();
      if (mounted.current) setBusy(false);
    }
  };

  const retry = () => {
    if (lastInput.current) void execute(lastInput.current);
  };

  const commitRename = (field: Field, value: string) => {
    const newName = value.trim();
    if (!newName || newName === field.name) return;
    const marker = `${field.name}\u0000${newName}`;
    if (lastNameCommit.current === marker) return;
    lastNameCommit.current = marker;
    void execute({ operation: "rename", table: snapshot?.schema.table, field: field.name, newName }).then((success) => {
      if (!success) lastNameCommit.current = null;
    });
  };

  const changeType = (field: Field, newType: string) => {
    if (newType === field.type) return;
    void execute({ operation: "change_type", table: snapshot?.schema.table, field: field.name, newType });
  };

  const addField = () => {
    const typeName = snapshot?.fieldTypes.find((candidate) => candidate === "int") ?? snapshot?.fieldTypes[0];
    if (!typeName) return;
    void execute({ operation: "add_direct", table: snapshot?.schema.table, typeName });
  };

  const confirmDrop = () => {
    if (!dropTarget) return;
    const field = dropTarget;
    setDropTarget(null);
    void execute({ operation: "drop", table: snapshot?.schema.table, field }, true);
  };

  const startReference = (operation: ReferenceDraft["operation"], reference?: Reference) => {
    setResult(null);
    setError(null);
    setReferenceDraft({
      operation,
      selectedName: reference?.name ?? "",
      name: reference?.name ?? "",
      csharpName: reference?.csharpName ?? "",
      sourceFields: reference?.sourceFields.join(", ") ?? "",
      targetTable: reference?.targetTable ?? "",
      targetFields: reference?.targetFields.join(", ") ?? "",
    });
  };

  const submitReference = async () => {
    if (!snapshot || !referenceDraft) return;
    const fields = referenceDraft.sourceFields.split(",").map((value) => value.trim()).filter(Boolean);
    const targetFields = referenceDraft.targetFields.split(",").map((value) => value.trim()).filter(Boolean);
    const input = referenceDraft.operation === "remove_reference"
      ? { operation: "remove_reference", table: snapshot.schema.table, name: referenceDraft.selectedName }
      : {
        operation: referenceDraft.operation,
        table: snapshot.schema.table,
        ...(referenceDraft.operation === "edit_reference" ? { name: referenceDraft.selectedName } : {}),
        reference: {
          name: referenceDraft.name,
          csharpName: referenceDraft.csharpName || null,
          fields,
          target: { table: referenceDraft.targetTable, fields: targetFields },
        },
      };
    const success = await execute(input, referenceDraft.operation === "remove_reference");
    if (success && mounted.current) setReferenceDraft(null);
  };

  return <section className="table-editor table-editor-direct" aria-label="Table Editor">
    {error && <Alert role="alert" type="error" title={error} action={lastInput.current ? <Button size="small" onClick={retry} disabled={!canWrite || busy}>Retry</Button> : undefined} />}
    {result && result.state !== "success" && <Alert role="status" type="warning" title={result.state} description={result.diagnostic?.message} />}
    {!snapshot && !error && <Spin tip="Loading Table"><div style={{ minHeight: 72 }} /></Spin>}
    {snapshot && <>
      <div className="table-context-line"><strong>{snapshot.schema.table}</strong><span>{snapshot.path}</span></div>
      <div className="direct-schema-toolbar">
        <h3>Fields <span>{snapshot.schema.fields.length}</span></h3>
        <Space>
          {!embedded && onOverview && <Button onClick={onOverview}>Table Overview</Button>}
          {!embedded && onCreateData && <Button onClick={onCreateData}>New data file</Button>}
          {lastPlan && <Button onClick={() => setDetailsOpen(true)} disabled={busy}>Details</Button>}
          <Button aria-label="Add Field" disabled={!canWrite || busy} onClick={addField}>＋ Add Field</Button>
        </Space>
      </div>
      <div className="direct-schema-scroll">
        <table className="direct-schema-grid">
          <thead><tr><th>Key</th><th>Field name</th><th>Type</th><th>Modifier</th><th aria-label="Field actions" /><th><Button type="text" size="small" aria-label="Add Field at end of columns" disabled={!canWrite || busy} onClick={addField}>＋</Button></th></tr></thead>
          <tbody>
            {snapshot.schema.fields.map((field) => <tr key={field.name}>
              <td>
                <span>{field.key}</span>
                {snapshot.schema.primaryKey?.fields.includes(field.name) && <Tag>Primary</Tag>}
                {snapshot.schema.secondaryKeys?.map((key, index) => key.fields.includes(field.name) ? <Tag key={index}>Secondary {index + 1}{key.nonUnique ? " · non-unique" : ""}</Tag> : null)}
              </td>
              <td>
                <input
                  aria-label={`Field name ${field.name}`}
                  value={draftNames[field.name] ?? field.name}
                  disabled={!canWrite || busy}
                  onChange={(event) => setDraftNames((current) => ({ ...current, [field.name]: event.target.value }))}
                  onKeyDown={(event) => {
                      if (event.nativeEvent.isComposing || event.keyCode === 229) return;
                      if (event.key === "Enter") {
                      event.preventDefault();
                      commitRename(field, event.currentTarget.value);
                      event.currentTarget.blur();
                    } else if (event.key === "Escape") {
                      event.preventDefault();
                      skipNameBlur.current = field.name;
                      setDraftNames((current) => ({ ...current, [field.name]: field.name }));
                      event.currentTarget.blur();
                    }
                  }}
                  onBlur={(event) => {
                    if (skipNameBlur.current === field.name) {
                      skipNameBlur.current = null;
                      return;
                    }
                    commitRename(field, event.currentTarget.value);
                  }}
                />
              </td>
              <td>
                <select aria-label={`Field type ${field.name}`} value={field.type} disabled={!canWrite || busy} onChange={(event) => changeType(field, event.target.value)}>
                  {snapshot.fieldTypes.map((type) => <option key={type} value={type}>{type}</option>)}
                </select>
              </td>
              <td>{field.array ? "Array" : field.nullable ? "Nullable" : "Required"}</td>
              <td>
                <Dropdown menu={{ items: [
                  { key: "rename", label: "Rename Field", onClick: () => document.querySelector<HTMLInputElement>(`[aria-label="Field name ${CSS.escape(field.name)}"]`)?.focus() },
                  { key: "drop", label: "Drop Field", danger: true, onClick: () => setDropTarget(field.name) },
                ] }} trigger={["click"]}>
                  <Button type="text" size="small" aria-label={`Actions for field ${field.name}`} icon={<MoreHorizontal size={16} />} />
                </Dropdown>
                <Button type="text" danger aria-label={`Drop Field ${field.name}`} disabled={!canWrite || busy} onClick={() => setDropTarget(field.name)}>Drop</Button>
              </td>
              <td />
            </tr>)}
          </tbody>
        </table>
      </div>
      <p className="direct-schema-hint">Name and type changes are applied through the shared migration safety checks. Keys and modifiers remain unchanged.</p>
      <section aria-label="References" className="table-references direct-schema-references">
        <h3>References</h3>
        {(snapshot.references ?? snapshot.schema.references ?? []).map((reference) => <div key={reference.name} className="table-reference">
          <strong>{reference.name}</strong>
          <span> · {reference.sourceFields.join(" → ")} → {reference.targetTable}.{reference.targetFields.join(" → ")}</span>
          <Tag>{reference.csharpName ? `exact: ${reference.csharpName}` : "implicit helper"}</Tag>
          <Tag>{reference.targetKeyKind ?? "unresolved target"}</Tag>
          <Tag>{reference.cardinality ?? "unresolved cardinality"}</Tag>
          <Tag>{reference.optionality ?? "unresolved optionality"}</Tag>
          <Tag>{reference.effectiveCsharpName ?? "unresolved helper name"}</Tag>
        </div>)}
        {(snapshot.referenceDiagnostics ?? []).map((diagnostic, index) => <Alert key={`${diagnostic.code}-${index}`} type="error" title={`${diagnostic.code}: ${diagnostic.message}`} />)}
        {(snapshot.references ?? snapshot.schema.references ?? []).length === 0 && <p>No References declared.</p>}
        {(snapshot.references ?? snapshot.schema.references ?? []).map((reference) => <div key={`actions-${reference.name}`} className="table-reference-actions">
          <Button size="small" disabled={!canWrite || busy} onClick={() => startReference("edit_reference", reference)}>Edit Reference</Button>
          <Button size="small" danger disabled={!canWrite || busy} onClick={() => startReference("remove_reference", reference)}>Remove Reference</Button>
        </div>)}
        <Button size="small" disabled={!canWrite || busy} onClick={() => startReference("add_reference")}>Add Reference</Button>
      </section>
    </>}
    {dropTarget && <div className="direct-schema-confirm" role="alertdialog" aria-label="Confirm Drop Field">
      <strong>Drop {snapshot?.schema.table}.{dropTarget}?</strong>
      <span>This removes the field from the schema and record values after review by the backend migration planner.</span>
      <Space><Button danger onClick={confirmDrop} disabled={busy}>Drop Field</Button><Button onClick={() => setDropTarget(null)} disabled={busy}>Cancel</Button></Space>
    </div>}
    {result && result.state !== "success" && <Alert role="status" type="warning" title={result.state} description={result.diagnostic?.message} action={<Button size="small" onClick={retry} disabled={!canWrite || busy || !lastInput.current}>Re-plan</Button>} />}
    <Modal open={detailsOpen} title="Migration Details" onCancel={() => setDetailsOpen(false)} footer={<Button onClick={() => setDetailsOpen(false)}>Close</Button>}>
      {lastPlan && <section aria-label="Migration Plan">
        <h3>{lastPlan.operation}: {lastPlan.table}.{lastPlan.field}</h3>
        <p>{lastPlan.files.length} affected files · {lastPlan.affectedRecordCount} affected records · {lastPlan.destructive ? "Destructive" : "Non-destructive"}</p>
        {lastPlan.files.map((file) => <div className="migration-diff" key={file.path}><strong>{file.path}</strong><section><h4>Before</h4><pre>{file.before}</pre></section><section><h4>After</h4><pre>{file.after}</pre></section></div>)}
      </section>}
    </Modal>
    <Modal open={referenceDraft !== null} title={referenceDraft?.operation === "add_reference" ? "Add Reference" : referenceDraft?.operation === "edit_reference" ? "Edit Reference" : "Remove Reference"} onCancel={() => setReferenceDraft(null)} footer={null} destroyOnHidden>
      {referenceDraft?.operation === "remove_reference" ? <Alert type="warning" title={`Remove Reference ${referenceDraft.selectedName}?`} description="The shared migration planner will remove the declaration without rewriting unrelated source." /> : referenceDraft && <Space direction="vertical" style={{ width: "100%" }}>
        <label>Reference name<Input aria-label="Reference name" value={referenceDraft.name} onChange={(event) => setReferenceDraft({ ...referenceDraft, name: event.target.value })} /></label>
        <label>C# helper name (optional)<Input aria-label="C# helper name" value={referenceDraft.csharpName} onChange={(event) => setReferenceDraft({ ...referenceDraft, csharpName: event.target.value })} /></label>
        <label>Source fields<Input aria-label="Reference source fields" value={referenceDraft.sourceFields} onChange={(event) => setReferenceDraft({ ...referenceDraft, sourceFields: event.target.value })} /></label>
        <label>Target table<Input aria-label="Reference target table" value={referenceDraft.targetTable} onChange={(event) => setReferenceDraft({ ...referenceDraft, targetTable: event.target.value })} /></label>
        <label>Target fields<Input aria-label="Reference target fields" value={referenceDraft.targetFields} onChange={(event) => setReferenceDraft({ ...referenceDraft, targetFields: event.target.value })} /></label>
      </Space>}
      <Space style={{ marginTop: 16 }}>
        <Button danger={referenceDraft?.operation === "remove_reference"} onClick={() => void submitReference()} disabled={!canWrite || busy}>{referenceDraft?.operation === "remove_reference" ? "Remove Reference" : "Apply Reference"}</Button>
        <Button onClick={() => setReferenceDraft(null)} disabled={busy}>Cancel</Button>
      </Space>
    </Modal>
  </section>;
}
