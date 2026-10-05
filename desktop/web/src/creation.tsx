import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Alert, Button, Checkbox, Divider, Flex, Form, Input, Modal, Select, Space, Spin, Tooltip, Typography, type MenuProps } from "antd";
import { ArrowDownOutlined, ArrowUpOutlined, CloseOutlined, DeleteOutlined, PlusOutlined, SettingOutlined } from "@ant-design/icons";
import type { Inventory } from "./types";
import { desktop } from "./workspace";

type Key = {fields:string[];nonUnique:boolean};
type Reference = {name:string;fields:string[];targetTable:string;targetFields:string[];csharpName:string};
type Field = {key:string;name:string;typeName:string;nullable:boolean;array:boolean};
export type Artifact =
  | {category:"folder"}
  | {category:"table";name:string;csharpName:string|null;fields:Field[];primary:Key;secondary:Key[];references:Reference[];inline:boolean}
  | {category:"data";table:string}
  | {category:"valueObject";name:string;underlying:string;fromImplicit:boolean;toImplicit:boolean}
  | {category:"enum"|"flags";name:string;underlying:string;members:[string,string][]}
  | {category:"custom";name:string;fields:Field[]};
type Category = Artifact["category"];
type Choices = {roots:{root:string;path:string}[];fieldTypes:string[];valueObjectUnderlying:string[];enumUnderlying:string[];tables:string[]};
type Preview = {valid:boolean;identity:string|null;category:Category;diagnostics:{code:string;message:string}[]};
type Request = {root:string;path:string;artifact:Artifact};
type Result = {source:string;outcome:string;message:string};
type Draft = {id:number;root:string;folder:string;filename:string;category:Category;artifact:Artifact|null;advanced:boolean;pending:boolean;result:Result|null;error:string|null;preview:Preview|null};
const categories: [Category,string][] = [["folder","Folder"],["table","Table"],["data","Data"],["valueObject","Value Object"],["enum","Enum"],["flags","Flags Enum"],["custom","Custom Type"]];
const join = (folder:string,name:string) => folder && folder!=="."?`${folder}/${name}`:name;
const parent = (path:string) => path.includes("/")?path.slice(0,path.lastIndexOf("/")):".";
const errorText = (error:unknown) => error && typeof error==="object" && "message" in error ? `${"code" in error?String(error.code)+": ":""}${String(error.message)}`:String(error);
const options = (values:string[]) => values.map(value=>({value,label:value}));
const reorder = <T,>(items:T[],index:number,delta:number) => {
  const next=items.slice(); const other=index+delta;
  if(other<0||other>=items.length)return next;
  [next[index],next[other]]=[next[other],next[index]]; return next;
};

// Only typed form input lives here. Rust supplies proposals, choices, validation,
// source construction and exclusive commit; no YAML/type rule is reconstructed.
export function useCreation(inventory:Inventory,epoch:number,context:string,onCreated:(path:string,folder:boolean,inputIntent:number)=>void) {
  const [choices,setChoices]=useState<Choices|null>(null),[draft,setDraft]=useState<Draft|null>(null);
  const live=useRef(draft); live.current=draft;
  const revision=useRef(0),requestId=useRef(0),filename=useRef<HTMLInputElement|null>(null),advancedFilename=useRef<HTMLInputElement|null>(null);
  const focusRequest=useRef<{id:number;advanced:boolean;intent:number;done:boolean}|null>(null);
  const origin=useRef<HTMLElement|null>(null);
  const patch=(change:Partial<Draft>)=>{
    const current=live.current;if(!current)return;
    live.current={...current,...change};setDraft(live.current);
  };
  useEffect(()=>{
    let current=true;
    void desktop.rpc<Choices>({kind:"creationChoices",epoch}).then(reply=>{if(current&&reply.host.epoch===desktop.surface.status.epoch)setChoices(reply.data);}).catch(()=>{});
    return()=>{current=false;};
  },[epoch,inventory.externalVersion,inventory.generation]);
  useEffect(()=>{revision.current++;setDraft(null);},[epoch]);
  useLayoutEffect(()=>{
    if(!draft)return;
    focusRequest.current={id:draft.id,advanced:draft.advanced,intent:desktop.inputIntent,done:false};
    const frame=requestAnimationFrame(focusFilename); return()=>cancelAnimationFrame(frame);
  },[draft?.id,draft?.advanced]);
  function focusFilename() {
    const request=focusRequest.current;
    if(!request||request.done||request.id!==live.current?.id||request.advanced!==live.current.advanced||request.intent!==desktop.inputIntent)return;
    const input=request.advanced?advancedFilename.current:filename.current;
    if(input?.isConnected){input.focus();request.done=true;}
  }
  const target=(folder:string) => {
    const candidates=choices?.roots.filter(r=>!r.path||folder===r.path||folder.startsWith(r.path+"/"))??[];
    const root=candidates.sort((a,b)=>b.path.length-a.path.length)[0]??choices?.roots[0];
    return root?{root:root.root,folder:candidates.length?folder:(root.path||".")}:null;
  };
  async function validate(next:Draft,propose=false) {
    const mine=++revision.current;
    patch({preview:null,pending:true,error:null});
    try {
      const artifact=propose?(await desktop.rpc<Artifact>({kind:"creationDefaults",epoch,category:next.category,path:join(next.folder,next.filename),table:desktop.surface.projection?.table.name??null})).data:next.artifact;
      if(mine!==revision.current||!artifact||desktop.surface.status.epoch!==epoch)return;
      const request:Request={root:next.root,path:join(next.folder,next.filename),artifact};
      const reply=await desktop.rpc<Preview>({kind:"creationPreview",epoch,request});
      if(mine===revision.current&&desktop.surface.status.epoch===epoch)patch({artifact,preview:reply.data,pending:false});
    } catch(error) {if(mine===revision.current)patch({pending:false,error:errorText(error)});}
  }
  function begin(category:Category) {
    const destination=target(context.startsWith("folder:")?context.slice(7):parent(context));
    if(!destination)return;
    origin.current=document.activeElement as HTMLElement;
    const file=category==="folder"?"new-folder":category==="table"?"new-table.yaml":category==="data"?"new-data.yaml":`${category==="valueObject"?"value-object":category==="custom"?"custom-type":category}.yaml`;
    const next:Draft={...destination,id:++requestId.current,filename:file,category,artifact:null,advanced:false,pending:true,preview:null,result:null,error:null};
    live.current=next;setDraft(next); void validate(next,true);
  }
  function cancel() {
    if(live.current?.result?.outcome==="OutcomeUnknown"||live.current?.result?.outcome==="RecoveryRequired")return;
    revision.current++;live.current=null;setDraft(null);
    if(origin.current?.isConnected)origin.current.focus();
  }
  function change(change:Partial<Draft>,propose=false) {
    if(!live.current)return;
    const next={...live.current,...change,result:null,error:null};
    live.current=next;setDraft(next); void validate(next,propose);
  }
  async function create(recheck=false) {
    const d=live.current;
    if(!d||d.pending||!d.artifact||(!recheck&&!d.preview?.valid))return;
    const request:Request={root:d.root,path:join(d.folder,d.filename),artifact:d.artifact};
    const intent=desktop.inputIntent;
    patch({pending:true,error:null});
    try {
      const reply=await desktop.rpc<Result>(recheck?{kind:"recheckCreation",epoch,path:request.path}:{kind:"create",epoch,request});
      if(desktop.surface.status.epoch!==epoch||live.current?.id!==d.id)return;
      patch({result:reply.data,pending:false});
      await desktop.refreshInventory();
      if(reply.data.outcome==="Success"&&desktop.surface.status.epoch===epoch) {
        revision.current++;setDraft(null);onCreated(request.path,d.category==="folder",intent);
      }
    } catch(error) {if(desktop.surface.status.epoch===epoch&&live.current?.id===d.id)patch({pending:false,error:errorText(error)});}
  }
  const uncertain=draft?.result?.outcome==="OutcomeUnknown"||draft?.result?.outcome==="RecoveryRequired";
  const committing=!!draft?.pending&&!!draft.artifact&&!!draft.preview;
  const content=draft&&<div className="creation-inline" onClick={e=>e.stopPropagation()} onKeyDown={e=>e.stopPropagation()}>
    <Flex justify="space-between" align="center"><Typography.Text type="secondary">{categories.find(([kind])=>kind===draft.category)?.[1]}</Typography.Text><Space size={0}><Tooltip title="Advanced creation"><Button type="text" icon={<SettingOutlined/>} aria-label="Advanced creation" disabled={committing||uncertain} onClick={()=>patch({advanced:true})}/></Tooltip><Tooltip title="Cancel creation"><Button type="text" icon={<CloseOutlined/>} aria-label="Cancel creation" disabled={committing||uncertain} onClick={cancel}/></Tooltip></Space></Flex>
    {/* Ant Input recreates its input when the affix structure changes. Keep the
        suffix mounted so preview completion cannot discard focus or composition. */}
    <Input ref={r=>{filename.current=r?.input??null;}} aria-label="New artifact filename" value={draft.filename} disabled={committing||uncertain} suffix={<span className="creation-pending">{draft.pending?<Spin size="small"/>:null}</span>} onChange={e=>change({filename:e.target.value},!draft.advanced)} onKeyDown={e=>{if(e.nativeEvent.isComposing)return;if(e.key==="Escape"){e.preventDefault();cancel();}if(e.key==="Enter"){e.preventDefault();void create();}}}/>
    <Typography.Text type="secondary" className="creation-destination" title={`${draft.root} · ${draft.folder}`}>{draft.folder} · {draft.preview?.identity??(draft.category==="folder"?"Folder":"…")}</Typography.Text>
    {draft.artifact?.category==="data"&&<Select aria-label="Data Table" value={draft.artifact.table||undefined} placeholder="Tableを選択" options={options(choices?.tables??[])} disabled={committing||uncertain} onChange={table=>change({artifact:{category:"data",table}})}/>}
    {!draft.advanced&&<Feedback draft={draft}/>}
    {uncertain&&<Button onClick={()=>void create(true)} loading={draft.pending}>Recheck destination</Button>}
  </div>;
  const menu:MenuProps={items:categories.map(([key,label])=>({key,label})),onClick:({key})=>begin(key as Category)};
  const modal=draft&&<Modal title={`New ${categories.find(([kind])=>kind===draft.category)?.[1]}`} open={draft.advanced} afterOpenChange={open=>{if(open)focusFilename();}} width={730} onCancel={()=>{if(!committing&&!uncertain)patch({advanced:false});}} mask={{closable:!committing&&!uncertain}} keyboard={!committing&&!uncertain} footer={<Space><Button disabled={committing||uncertain} onClick={cancel}>Cancel</Button><Button type="primary" loading={committing} disabled={draft.pending||(!uncertain&&!draft.preview?.valid)} onClick={()=>void create(!!uncertain)}>{uncertain?"Recheck destination":"Create"}</Button></Space>}>
    <Form layout="vertical" className="creation-form" disabled={committing||uncertain}>
      <Flex gap={12}><Form.Item label="Source root"><Select aria-label="Creation source root" value={draft.root} options={choices?.roots.map(r=>({value:r.root,label:r.root}))} onChange={root=>{const path=choices!.roots.find(r=>r.root===root)!.path||".";change({root,folder:path});}}/></Form.Item><Form.Item label="Folder"><Select aria-label="Creation folder" value={draft.folder} options={options(inventory.folders.map(f=>f||".").filter(folder=>{const path=choices?.roots.find(r=>r.root===draft.root)?.path;return path!==undefined&&(!path||folder===path||folder.startsWith(path+"/"));}))} onChange={folder=>change({folder})}/></Form.Item></Flex>
      <Form.Item label="Filename"><Input ref={r=>{advancedFilename.current=r?.input??null;}} aria-label="Creation filename" value={draft.filename} onChange={e=>change({filename:e.target.value})} onKeyDown={e=>{if(e.nativeEvent.isComposing)return;if(e.key==="Enter")e.preventDefault();}}/></Form.Item>
      {draft.artifact&&choices&&<Declaration artifact={draft.artifact} choices={choices} disabled={committing||uncertain} update={artifact=>change({artifact})}/>}
    </Form><Feedback draft={draft}/>
  </Modal>;
  return {menu,draft,inline:content,modal,path:draft?join(draft.folder,draft.filename):null,ready:!!choices};
}
function Feedback({draft}:{draft:Draft}) {
  const messages=[...(draft.error?[draft.error]:[]),...(!draft.pending?(draft.preview?.diagnostics.map(d=>`${d.code}: ${d.message}`)??[]):[]),...(draft.result&&draft.result.outcome!=="Success"?[`${draft.result.outcome}: ${draft.result.message}`]:[])];
  return messages.length?<Alert type={draft.result?.outcome==="OutcomeUnknown"?"warning":"error"} showIcon title={messages.join("\n")}/>:null;
}
function Declaration({artifact:a,choices,update,disabled}:{artifact:Artifact;choices:Choices;update:(a:Artifact)=>void;disabled:boolean}) {
  if(a.category==="folder")return null;
  if(a.category==="data")return <Form.Item label="Table"><Select aria-label="Data Table" value={a.table||undefined} placeholder="Tableを選択" options={options(choices.tables)} onChange={table=>update({...a,table})}/></Form.Item>;
  return <>
    <Form.Item label={a.category==="table"?"Table identity":"Type name"}><Input aria-label="Artifact identity" value={a.name} onChange={e=>update({...a,name:e.target.value})}/></Form.Item>
    {a.category==="table"&&<><Form.Item label="C# name (optional)"><Input aria-label="C# name" value={a.csharpName??""} onChange={e=>update({...a,csharpName:e.target.value||null})}/></Form.Item><Checkbox checked={a.inline} onChange={e=>update({...a,inline:e.target.checked})}>Empty inline records</Checkbox></>}
    {(a.category==="table"||a.category==="custom")&&<FieldList fields={a.fields} choices={choices.fieldTypes} disabled={disabled} update={fields=>update({...a,fields})}/>}
    {a.category==="table"&&<>
      <Form.Item label="Primary Key (ordered)"><Select mode="multiple" aria-label="Primary Key fields" value={a.primary.fields} options={options(a.fields.map(f=>f.name))} onChange={fields=>update({...a,primary:{fields,nonUnique:false}})}/></Form.Item>
      <div className="creation-list-heading"><Typography.Text>Secondary Keys</Typography.Text><Button icon={<PlusOutlined/>} onClick={()=>update({...a,secondary:[...a.secondary,{fields:[],nonUnique:false}]})}>Add key</Button></div>
      {a.secondary.map((k,i)=><Flex className="creation-list-row" key={i} gap={8} align="center"><Select mode="multiple" aria-label={`Secondary Key ${i+1} fields`} value={k.fields} options={options(a.fields.map(f=>f.name))} onChange={fields=>update({...a,secondary:a.secondary.map((old,j)=>i===j?{...old,fields}:old)})}/><Checkbox checked={k.nonUnique} onChange={e=>update({...a,secondary:a.secondary.map((old,j)=>i===j?{...old,nonUnique:e.target.checked}:old)})}>Nonunique</Checkbox><OrderButtons index={i} count={a.secondary.length} label="key" move={delta=>update({...a,secondary:reorder(a.secondary,i,delta)})} remove={()=>update({...a,secondary:a.secondary.filter((_,j)=>i!==j)})}/></Flex>)}
    </>}
    {"underlying" in a&&<Form.Item label="Underlying type"><Select aria-label="Underlying type" value={a.underlying} options={options(a.category==="valueObject"?choices.valueObjectUnderlying:choices.enumUnderlying)} onChange={underlying=>update({...a,underlying})}/></Form.Item>}
    {a.category==="valueObject"&&<Space direction="vertical"><Checkbox checked={a.fromImplicit} onChange={e=>update({...a,fromImplicit:e.target.checked})}>Implicit conversion from underlying</Checkbox><Checkbox checked={a.toImplicit} onChange={e=>update({...a,toImplicit:e.target.checked})}>Implicit conversion to underlying</Checkbox></Space>}
    {(a.category==="enum"||a.category==="flags")&&<>
      <div className="creation-list-heading"><Typography.Text>Members</Typography.Text><Button icon={<PlusOutlined/>} onClick={()=>update({...a,members:[...a.members,["",""]]})}>Add member</Button></div>
      <div role="list" aria-label="Enum members">{a.members.map(([name,value],i)=><Flex role="listitem" className="creation-list-row" key={i} gap={8} align="center"><Input aria-label={`Member ${i+1} name`} value={name} placeholder="Name" onChange={e=>update({...a,members:a.members.map((m,j)=>i===j?[e.target.value,m[1]]:m)})}/><Input aria-label={`Member ${i+1} value`} value={value} placeholder="Explicit integer" inputMode="numeric" onChange={e=>update({...a,members:a.members.map((m,j)=>i===j?[m[0],e.target.value]:m)})}/><OrderButtons index={i} count={a.members.length} label="member" move={delta=>update({...a,members:reorder(a.members,i,delta)})} remove={()=>update({...a,members:a.members.filter((_,j)=>i!==j)})}/></Flex>)}</div>
    </>}
  </>;
}
function FieldList({fields,choices,update,disabled}:{fields:Field[];choices:string[];update:(f:Field[])=>void;disabled:boolean}) {
  const live=useRef(fields);live.current=fields;
  const epoch=desktop.surface.status.epoch;
  async function add() {
    const observed=live.current;
    const reply=await desktop.rpc<Field>({kind:"creationField",epoch,fields:observed});
    if(live.current===observed&&desktop.surface.status.epoch===epoch)update([...observed,reply.data]);
  }
  const set=(i:number,change:Partial<Field>)=>update(fields.map((f,j)=>i===j?{...f,...change}:f));
  return <>
    <Divider/>
    <div className="creation-list-heading"><Typography.Text>Fields</Typography.Text><Button icon={<PlusOutlined/>} disabled={disabled} onClick={()=>void add().catch(desktop.showError)}>Add field</Button></div>
    <div role="list" aria-label="Field declarations">{fields.map((f,i)=><div role="listitem" className="creation-field" key={i}>
      <Flex gap={8} align="center"><Input className="creation-key" aria-label={`Field ${i+1} MessagePack key`} value={f.key} inputMode="numeric" onChange={e=>set(i,{key:e.target.value})}/><Input aria-label={`Field ${i+1} name`} value={f.name} placeholder="Name" onChange={e=>set(i,{name:e.target.value})}/><Select aria-label={`Field ${i+1} type`} value={f.typeName} options={options(choices)} onChange={typeName=>set(i,{typeName})}/><OrderButtons index={i} count={fields.length} label="field" move={delta=>update(reorder(fields,i,delta))} remove={()=>update(fields.filter((_,j)=>i!==j))}/></Flex>
      <Space size={16}><Checkbox checked={f.nullable} onChange={e=>set(i,{nullable:e.target.checked})}>Nullable</Checkbox><Checkbox checked={f.array} onChange={e=>set(i,{array:e.target.checked})}>Array</Checkbox></Space>
    </div>)}</div>
    <Divider/>
  </>;
}
function OrderButtons({index,count,label,move,remove}:{index:number;count:number;label:string;move:(delta:number)=>void;remove:()=>void}) {
  return <Space size={0}><Tooltip title="Move earlier"><Button type="text" icon={<ArrowUpOutlined/>} disabled={index===0} aria-label={`Move ${label} ${index+1} earlier`} onClick={()=>move(-1)}/></Tooltip><Tooltip title="Move later"><Button type="text" icon={<ArrowDownOutlined/>} disabled={index===count-1} aria-label={`Move ${label} ${index+1} later`} onClick={()=>move(1)}/></Tooltip><Tooltip title="Delete"><Button type="text" icon={<DeleteOutlined/>} aria-label={`Delete ${label} ${index+1}`} onClick={remove}/></Tooltip></Space>;
}
