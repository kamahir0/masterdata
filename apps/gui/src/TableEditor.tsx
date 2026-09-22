import { useEffect, useRef, useState } from "react";
import { Alert, Button, Checkbox, Form, Input, InputNumber, Select, Space, Spin, Table, Tabs, Tag } from "antd";
import { invoke } from "@tauri-apps/api/core";
import { TypedInitializer, initializerJson, resetInitializer } from "./TypedInitializer";
import type { AuthoringValue, ResolvedAuthoringType } from "./data-editor-types";
import MigrationCompatibilityImpact, { type MigrationCompatibility } from "./MigrationCompatibilityImpact";

type Field = { key:number; name:string; type:string; nullable:boolean; array:boolean };
type Reference = { name:string; csharpName?:string|null; effectiveCsharpName?:string; sourceFields:string[]; targetTable:string; targetFields:string[]; targetKeyKind?:"primary"|"secondary"; cardinality?:"single"|"many"; optionality?:"required"|"nullable" };
type Snapshot = { path:string; schema:{ table:string; fields:Field[]; primaryKey:{fields:string[]}; secondaryKeys:{fields:string[];nonUnique:boolean}[]; references?:Reference[] }; fieldTypes:string[]; initializerShapes:Record<string,ResolvedAuthoringType>; references?:Reference[]; referenceDiagnostics?:{code:string;message:string;source?:string;schemaPath?:string;valuePath?:string;recordIdentity?:string}[] };
type Plan = { token:string; table:string; operation:string; field:string; destructive:boolean; affectedRecordCount:number; files:{path:string;before:string;after:string}[]; diagnostics:{code:string;message:string}[]; compatibility:MigrationCompatibility };
export type MigrationResult = { state:string; files:string[]; fileStates?:{path:string;state:string}[]; diagnostic?:{code:string;message:string}|null; recoveryWorkspace?:string|null };
const message = (error:unknown):string => {
  if (!error || typeof error !== "object" || !("diagnostic" in error)) return String(error);
  const diagnostic = (error as {diagnostic:{code:string;message:string;source?:string;schemaPath?:string;schema_path?:string}}).diagnostic;
  return [diagnostic.source, diagnostic.schemaPath ?? diagnostic.schema_path, `${diagnostic.code}: ${diagnostic.message}`].filter(Boolean).join(" · ");
};
export default function TableEditor({projectPath,path,canWrite,dirtyPaths,beginApply,onResult,endApply}: {
  projectPath:string; path:string; canWrite:boolean; dirtyPaths:string[];
  beginApply:(paths:string[])=>boolean; onResult:(result:MigrationResult)=>Promise<void>; endApply:()=>void;
}) {
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
  useEffect(()=>{if(plan)document.getElementById("migration-plan-summary")?.focus();},[plan]);
  useEffect(()=>{ mounted.current=true; let disposed=false;
    void invoke<Snapshot>("open_table",{projectPath,relativePath:path}).then(value=>{if(!disposed){setSnapshot(value);setSelected(value.schema.fields[0]?.name??"");}}).catch(error=>{if(!disposed)setError(message(error));});
    return ()=>{disposed=true;mounted.current=false;revision.current+=1;};
  },[projectPath,path]);
  const change=(fn:()=>void)=>{revision.current+=1;fn();setPlan(null);setConfirmed(false);setError(null);setResult(null);};
  const start=(value:"add"|"rename"|"drop")=>{change(()=>{setOperation(value);setReferenceOperation(null);setName(value==="rename"?selected:"");setKey(snapshot?.schema.fields.length?Math.max(...snapshot.schema.fields.map(field=>field.key))+1:0);if(value==="add"){setHasInitializer(false);setInitializer(resetInitializer());}});window.requestAnimationFrame(()=>document.getElementById(value==="drop"?"migration-plan":"migration-name")?.focus());};
  const startReference=(value:"add_reference"|"edit_reference"|"remove_reference", reference?:Reference)=>{change(()=>{setOperation(null);setReferenceOperation(value);setSelectedReference(reference?.name??"");setReferenceName(reference?.name??"");setReferenceCsharpName(reference?.csharpName??"");setReferenceSourceFields(reference?.sourceFields.join(", ")??"");setReferenceTargetTable(reference?.targetTable??"");setReferenceTargetFields(reference?.targetFields.join(", ")??"");});window.requestAnimationFrame(()=>document.getElementById(value==="remove_reference"?"migration-plan":"reference-name")?.focus());};
  const cancel=()=>{change(()=>{setOperation(null);setReferenceOperation(null);});window.requestAnimationFrame(()=>document.getElementById(operation === "rename" ? "table-rename" : operation === "drop" ? "table-drop" : "table-add")?.focus());};
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
      try {outcome=await invoke<MigrationResult>("apply_table_migration",{projectPath,token:plan.token,allowDestructive:plan.destructive&&confirmed});}
      catch(error){
        if(error&&typeof error==="object"&&"diagnostic" in error)throw error;
        outcome={state:"recovery_required",files:plan.files.map(file=>file.path),diagnostic:{code:"E-MIGRATION-TRANSPORT",message:message(error)}};
      }
      if(mounted.current)setResult(outcome);
      await onResult(outcome);
      if(mounted.current&&outcome.state==="success"){
        const next=await invoke<Snapshot>("open_table",{projectPath,relativePath:path});
        if(mounted.current){setSnapshot(next);setOperation(null);setPlan(null);}
      }
    }catch(error){if(mounted.current)setError(message(error));}
    finally{inFlight.current=false;endApply();if(mounted.current)setBusy(false);}
  };
  const stale = result?.state === "not_started" && !!result.diagnostic?.message.includes("stale");
  const blocked=plan?.files.filter(file=>dirtyPaths.includes(file.path))??[];
  return <section className="table-editor" aria-label="Table Editor">
    {error&&<Alert role="alert" type="error" title={error}/>}
    {!snapshot&&!error&&<Spin tip="Loading Table"><div style={{minHeight:80}}/></Spin>}
    {snapshot&&<>
      <h2>{snapshot.schema.table} <Tag>Table</Tag></h2><p className="source-provenance">{snapshot.path}</p>
      <Table<Field> size="small" pagination={false} rowKey="name" dataSource={snapshot.schema.fields}
        rowSelection={{type:"radio",selectedRowKeys:[selected],onChange:keys=>change(()=>{setSelected(String(keys[0]));setOperation(null);}),getCheckboxProps:field=>({disabled:busy,"aria-label":`Select field ${field.name}`})}}
        columns={[{title:"Key",dataIndex:"key"},{title:"Field",dataIndex:"name"},{title:"Type",dataIndex:"type"},{title:"Modifier",render:(_,field)=>field.array?"Array":field.nullable?"Nullable":"Required"},{title:"Indexes",render:(_,field)=><>{snapshot.schema.primaryKey?.fields.includes(field.name)&&<Tag>Primary Key</Tag>}{snapshot.schema.secondaryKeys?.map((key,index)=>key.fields.includes(field.name)?<Tag key={index}>Secondary {index+1}{key.nonUnique?" · non-unique":""}</Tag>:null)}</>}]} />
      <p>Primary Key: {snapshot.schema.primaryKey?.fields.join(" → ")}</p>
      {snapshot.schema.secondaryKeys?.map((key,index)=><p key={index}>Secondary {index+1}: {key.fields.join(" → ")} {key.nonUnique?"(non-unique)":"(unique)"}</p>)}
      <section aria-label="References" className="table-references">
        <h3>References</h3>
        {(snapshot.references ?? snapshot.schema.references ?? []).map(reference=><div key={reference.name} className="table-reference">
          <strong>{reference.name}</strong>
          <span> · {reference.sourceFields.join(" → ")} → {reference.targetTable}.{reference.targetFields.join(" → ")}</span>
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
      <Space><Button id="table-add" disabled={!canWrite||busy} onClick={()=>start("add")}>Add Field</Button><Button id="table-rename" disabled={!selected||!canWrite||busy} onClick={()=>start("rename")}>Rename Field</Button><Button id="table-drop" danger disabled={!selected||!canWrite||busy} onClick={()=>start("drop")}>Drop Field</Button></Space>
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
        <MigrationCompatibilityImpact value={plan.compatibility}/>
        <Tabs items={plan.files.map(file=>({key:file.path,label:file.path,children:<div className="migration-diff"><section><h4>Before</h4><pre>{file.before}</pre></section><section><h4>After</h4><pre>{file.after}</pre></section></div>}))}/>
        {blocked.length>0&&<Alert type="warning" title="Apply blocked by unsaved affected files" description={blocked.map(file=>file.path).join(", ")}/>}
        {plan.destructive&&<Checkbox disabled={busy} checked={confirmed} onChange={event=>setConfirmed(event.target.checked)}>I confirm dropping {plan.table}.{plan.field} and its record values.</Checkbox>}
        <Button type="primary" danger={plan.destructive} loading={busy} disabled={!canWrite||stale||blocked.length>0||(plan.destructive&&!confirmed)||result?.state==="success"} onClick={()=>void apply()}>Apply reviewed Plan</Button>
      </section>}
      {result&&<Alert role="status" type={result.state==="success"?"success":"warning"} title={result.state==="not_started"&&result.diagnostic?.message.includes("stale")?"Stale Plan — re-plan required":result.state} description={result.diagnostic?.message}/>}
    </>}
  </section>;
}
