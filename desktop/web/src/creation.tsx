import { uiMessage } from "./language";
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
const categories: [Category,string][] = [["folder","フォルダー"],["table","テーブル"],["data","データ"],["valueObject","値オブジェクト"],["enum","列挙型"],["flags","フラグ列挙型"],["custom","カスタム型"]];
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
    <Flex justify="space-between" align="center"><Typography.Text type="secondary">{categories.find(([kind])=>kind===draft.category)?.[1]}</Typography.Text><Space size={0}><Tooltip title="作成内容の詳細"><Button type="text" icon={<SettingOutlined aria-hidden="true"/>} aria-label="作成内容の詳細" disabled={committing||uncertain} onClick={()=>patch({advanced:true})}/></Tooltip><Tooltip title="作成をキャンセル"><Button type="text" icon={<CloseOutlined aria-hidden="true"/>} aria-label="作成をキャンセル" disabled={committing||uncertain} onClick={cancel}/></Tooltip></Space></Flex>
    {/* Ant Input recreates its input when the affix structure changes. Keep the
        suffix mounted so preview completion cannot discard focus or composition. */}
    <Input ref={r=>{filename.current=r?.input??null;}} aria-label="新しいソースのファイル名" value={draft.filename} disabled={committing||uncertain} suffix={<span className="creation-pending">{draft.pending?<Spin size="small"/>:null}</span>} onChange={e=>change({filename:e.target.value},!draft.advanced)} onKeyDown={e=>{if(e.nativeEvent.isComposing)return;if(e.key==="Escape"){e.preventDefault();cancel();}if(e.key==="Enter"){e.preventDefault();void create();}}}/>
    <Typography.Text type="secondary" className="creation-destination" title={`${draft.root} · ${draft.folder}`}>{draft.folder} · {draft.preview?.identity??(draft.category==="folder"?"フォルダー":"…")}</Typography.Text>
    {draft.artifact?.category==="data"&&<Select aria-label="データのテーブル" value={draft.artifact.table||undefined} placeholder="テーブルを選択" options={options(choices?.tables??[])} disabled={committing||uncertain} onChange={table=>change({artifact:{category:"data",table}})}/>}
    {!draft.advanced&&<Feedback draft={draft}/>}
    {uncertain&&<Button onClick={()=>void create(true)} loading={draft.pending}>保存先を再確認</Button>}
  </div>;
  const menu:MenuProps={items:categories.map(([key,label])=>({key,label})),onClick:({key})=>begin(key as Category)};
  const modal=draft&&<Modal title={`新規: ${categories.find(([kind])=>kind===draft.category)?.[1]}`} open={draft.advanced} afterOpenChange={open=>{if(open)focusFilename();}} width={730} onCancel={()=>{if(!committing&&!uncertain)patch({advanced:false});}} mask={{closable:!committing&&!uncertain}} keyboard={!committing&&!uncertain} footer={<Space><Button disabled={committing||uncertain} onClick={cancel}>キャンセル</Button><Button type="primary" loading={committing} disabled={draft.pending||(!uncertain&&!draft.preview?.valid)} onClick={()=>void create(!!uncertain)}>{uncertain?"保存先を再確認":"作成"}</Button></Space>}>
    <Form layout="vertical" className="creation-form" disabled={committing||uncertain}>
      <Flex gap={12}><Form.Item label="ソースルート"><Select aria-label="作成先のソースルート" value={draft.root} options={choices?.roots.map(r=>({value:r.root,label:r.root}))} onChange={root=>{const path=choices!.roots.find(r=>r.root===root)!.path||".";change({root,folder:path});}}/></Form.Item><Form.Item label="フォルダー"><Select aria-label="作成先のフォルダー" value={draft.folder} options={options(inventory.folders.map(f=>f||".").filter(folder=>{const path=choices?.roots.find(r=>r.root===draft.root)?.path;return path!==undefined&&(!path||folder===path||folder.startsWith(path+"/"));}))} onChange={folder=>change({folder})}/></Form.Item></Flex>
      <Form.Item label="ファイル名"><Input ref={r=>{advancedFilename.current=r?.input??null;}} aria-label="作成するファイル名" value={draft.filename} onChange={e=>change({filename:e.target.value})} onKeyDown={e=>{if(e.nativeEvent.isComposing)return;if(e.key==="Enter")e.preventDefault();}}/></Form.Item>
      {draft.artifact&&choices&&<Declaration artifact={draft.artifact} choices={choices} disabled={committing||uncertain} update={artifact=>change({artifact})}/>}
    </Form><Feedback draft={draft}/>
  </Modal>;
  return {menu,begin,draft,inline:content,modal,path:draft?join(draft.folder,draft.filename):null,ready:!!choices};
}
function Feedback({draft}:{draft:Draft}) {
  const messages=[...(draft.error?[draft.error]:[]),...(!draft.pending?(draft.preview?.diagnostics.map(d=>`${d.code}: ${d.message}`)??[]):[]),...(draft.result&&draft.result.outcome!=="Success"?[`${draft.result.outcome}: ${draft.result.message}`]:[])];
  return messages.length?<Alert type={draft.result?.outcome==="OutcomeUnknown"?"warning":"error"} showIcon title={messages.map(uiMessage).join("\n")}/>:null;
}
function Declaration({artifact:a,choices,update,disabled}:{artifact:Artifact;choices:Choices;update:(a:Artifact)=>void;disabled:boolean}) {
  if(a.category==="folder")return null;
  if(a.category==="data")return <Form.Item label="テーブル"><Select aria-label="データのテーブル" value={a.table||undefined} placeholder="テーブルを選択" options={options(choices.tables)} onChange={table=>update({...a,table})}/></Form.Item>;
  return <>
    <Form.Item label={a.category==="table"?"テーブルの識別子":"型の名前"}><Input aria-label="ソースの識別子" value={a.name} onChange={e=>update({...a,name:e.target.value})}/></Form.Item>
    {a.category==="table"&&<><Form.Item label="C#名（任意）"><Input aria-label="C#名" value={a.csharpName??""} onChange={e=>update({...a,csharpName:e.target.value||null})}/></Form.Item><Checkbox checked={a.inline} onChange={e=>update({...a,inline:e.target.checked})}>空のインラインレコード</Checkbox></>}
    {(a.category==="table"||a.category==="custom")&&<FieldList fields={a.fields} choices={choices.fieldTypes} disabled={disabled} update={fields=>update({...a,fields})}/>}
    {a.category==="table"&&<>
      <Form.Item label="主キー（順序付き）"><Select mode="multiple" aria-label="主キーのフィールド" value={a.primary.fields} options={options(a.fields.map(f=>f.name))} onChange={fields=>update({...a,primary:{fields,nonUnique:false}})}/></Form.Item>
      <div className="creation-list-heading"><Typography.Text>副キー</Typography.Text><Button icon={<PlusOutlined aria-hidden="true"/>} onClick={()=>update({...a,secondary:[...a.secondary,{fields:[],nonUnique:false}]})}>キーを追加</Button></div>
      {a.secondary.map((k,i)=><Flex className="creation-list-row" key={i} gap={8} align="center"><Select mode="multiple" aria-label={`副キー ${i+1} フィールド`} value={k.fields} options={options(a.fields.map(f=>f.name))} onChange={fields=>update({...a,secondary:a.secondary.map((old,j)=>i===j?{...old,fields}:old)})}/><Checkbox checked={k.nonUnique} onChange={e=>update({...a,secondary:a.secondary.map((old,j)=>i===j?{...old,nonUnique:e.target.checked}:old)})}>重複を許可</Checkbox><OrderButtons index={i} count={a.secondary.length} label="キー" move={delta=>update({...a,secondary:reorder(a.secondary,i,delta)})} remove={()=>update({...a,secondary:a.secondary.filter((_,j)=>i!==j)})}/></Flex>)}
    </>}
    {"underlying" in a&&<Form.Item label="基になる型"><Select aria-label="基になる型" value={a.underlying} options={options(a.category==="valueObject"?choices.valueObjectUnderlying:choices.enumUnderlying)} onChange={underlying=>update({...a,underlying})}/></Form.Item>}
    {a.category==="valueObject"&&<Space direction="vertical"><Checkbox checked={a.fromImplicit} onChange={e=>update({...a,fromImplicit:e.target.checked})}>基になる型からの暗黙の変換</Checkbox><Checkbox checked={a.toImplicit} onChange={e=>update({...a,toImplicit:e.target.checked})}>基になる型への暗黙の変換</Checkbox></Space>}
    {(a.category==="enum"||a.category==="flags")&&<>
      <div className="creation-list-heading"><Typography.Text>メンバー</Typography.Text><Button icon={<PlusOutlined aria-hidden="true"/>} onClick={()=>update({...a,members:[...a.members,["",""]]})}>メンバーを追加</Button></div>
      <div role="list" aria-label="列挙型のメンバー">{a.members.map(([name,value],i)=><Flex role="listitem" className="creation-list-row" key={i} gap={8} align="center"><Input aria-label={`メンバー ${i+1} 名前`} value={name} placeholder="名前" onChange={e=>update({...a,members:a.members.map((m,j)=>i===j?[e.target.value,m[1]]:m)})}/><Input aria-label={`メンバー ${i+1} 値`} value={value} placeholder="整数を明示" inputMode="numeric" onChange={e=>update({...a,members:a.members.map((m,j)=>i===j?[m[0],e.target.value]:m)})}/><OrderButtons index={i} count={a.members.length} label="メンバー" move={delta=>update({...a,members:reorder(a.members,i,delta)})} remove={()=>update({...a,members:a.members.filter((_,j)=>i!==j)})}/></Flex>)}</div>
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
    <div className="creation-list-heading"><Typography.Text>フィールド</Typography.Text><Button icon={<PlusOutlined aria-hidden="true"/>} disabled={disabled} onClick={()=>void add().catch(desktop.showError)}>フィールドを追加</Button></div>
    <div role="list" aria-label="フィールドの宣言">{fields.map((f,i)=><div role="listitem" className="creation-field" key={i}>
      <Flex gap={8} align="center"><Input className="creation-key" aria-label={`フィールド ${i+1} MessagePackキー`} value={f.key} inputMode="numeric" onChange={e=>set(i,{key:e.target.value})}/><Input aria-label={`フィールド ${i+1} 名前`} value={f.name} placeholder="名前" onChange={e=>set(i,{name:e.target.value})}/><Select aria-label={`フィールド ${i+1} の型`} value={f.typeName} options={options(choices)} onChange={typeName=>set(i,{typeName})}/><OrderButtons index={i} count={fields.length} label="フィールド" move={delta=>update(reorder(fields,i,delta))} remove={()=>update(fields.filter((_,j)=>i!==j))}/></Flex>
      <Space size={16}><Checkbox checked={f.nullable} onChange={e=>set(i,{nullable:e.target.checked})}>Nullを許可</Checkbox><Checkbox checked={f.array} onChange={e=>set(i,{array:e.target.checked})}>配列</Checkbox></Space>
    </div>)}</div>
    <Divider/>
  </>;
}
function OrderButtons({index,count,label,move,remove}:{index:number;count:number;label:string;move:(delta:number)=>void;remove:()=>void}) {
  return <Space size={0}><Tooltip title="前に移動"><Button type="text" icon={<ArrowUpOutlined aria-hidden="true"/>} disabled={index===0} aria-label={`移動: ${label} ${index+1} 前へ`} onClick={()=>move(-1)}/></Tooltip><Tooltip title="後に移動"><Button type="text" icon={<ArrowDownOutlined aria-hidden="true"/>} disabled={index===count-1} aria-label={`移動: ${label} ${index+1} 後へ`} onClick={()=>move(1)}/></Tooltip><Tooltip title="削除"><Button type="text" icon={<DeleteOutlined aria-hidden="true"/>} aria-label={`削除: ${label} ${index+1}`} onClick={remove}/></Tooltip></Space>;
}
