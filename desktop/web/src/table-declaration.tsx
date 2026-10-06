import {useEffect, useLayoutEffect, useRef, useState} from "react";
import {Alert, Button, Checkbox, Descriptions, Drawer, Empty, Flex, Form, Input, Select, Space, Spin, Table, Tabs, Tag, Typography} from "antd";
import {DeleteOutlined, EditOutlined, PlusOutlined, ReloadOutlined} from "@ant-design/icons";
import {desktop, type Surface} from "./workspace";
import type {DeclarationKey, ReferenceDetail, ReferenceInput, SetResult, TableDeclarationChange, TableDeclarationReview, TableDeclarationSnapshot} from "./types";

type Context={id:number;epoch:number;target:string;source:string;table:string;snapshot:TableDeclarationSnapshot|null;change:TableDeclarationChange|null;review:TableDeclarationReview|null;pending:boolean;applying:boolean;closing:boolean;error:string|null;stale:boolean;uncertain:boolean;outcome:SetResult|null;referenceKey:number|null;externalVersion:number};
const errorText=(e:unknown)=>e&&typeof e==="object"&&"message" in e?`${"code" in e?String(e.code)+": ":""}${String(e.message)}`:String(e);
const labels:Record<TableDeclarationChange["kind"],string>={setFieldKey:"MessagePack key",setPrimaryKey:"Primary Key",addSecondaryKey:"Add Secondary Key",editSecondaryKey:"Edit Secondary Key",removeSecondaryKey:"Remove Secondary Key",addReference:"Add Reference",editReference:"Edit Reference",removeReference:"Remove Reference"};
const keyLabel=(key:DeclarationKey)=>`${key.fields.join(" → ")}${key.nonUnique?" · nonunique":" · unique"}`;
export function useTableDeclaration(s:Surface) {
  const [context,setContext]=useState<Context|null>(null),live=useRef(context);live.current=context;
  const request=useRef(0),ids=useRef(0),closed=useRef<{epoch:number;target:string;input:number}|null>(null);
  const focus=useRef<{input:number;id:number}|null>(null);
  const focusDetail=()=>{const root=document.querySelector('.ant-drawer-open');(root?.querySelector<HTMLElement>('[aria-label="Table declaration Plan"],.table-declaration-operation input:not(:disabled)')??root?.querySelector<HTMLElement>('button[aria-label="Review Table declaration change"]:not(:disabled)')??root?.querySelector<HTMLElement>('.table-declarations button:not(:disabled)'))?.focus();};
  const patch=(value:Partial<Context>)=>{if(live.current){live.current={...live.current,...value};setContext(live.current);}};
  const current=(c:Context)=>live.current?.id===c.id&&desktop.surface.status.epoch===c.epoch&&desktop.surface.target===c.target;
  useLayoutEffect(()=>{
    const frame=requestAnimationFrame(()=>{const f=focus.current,c=live.current;if(!f||!c||c.closing||f.id!==c.id||f.input!==desktop.inputIntent||!current(c)||desktop.surface.choice||desktop.surface.comparison)return;
      focusDetail();});
    return()=>cancelAnimationFrame(frame);
  },[context?.change?.kind,context?.review?.plan.token,context?.pending,s.choice,s.comparison]);
  useEffect(()=>{if(live.current&&(live.current.epoch!==s.status.epoch||live.current.target!==s.target)&&!live.current.applying){request.current++;live.current=null;setContext(null);}},[s.status.epoch,s.target]);
  const wasOverlay=useRef(false);
  useLayoutEffect(()=>{const overlay=!!s.choice||!!s.comparison;if(wasOverlay.current&&!overlay&&live.current)focus.current={id:live.current.id,input:desktop.inputIntent};wasOverlay.current=overlay;},[s.choice,s.comparison]);
  useEffect(()=>{const c=live.current;if(c&&!c.applying&&c.externalVersion!==s.status.externalVersion)patch({stale:true,error:"external sourceが変更されています。captured declarationを保持しています。現在のTable detailを再取得してください。"});},[s.status.externalVersion]);
  function close(){const c=live.current;if(!c||c.applying)return;closed.current={epoch:c.epoch,target:c.target,input:desktop.inputIntent};request.current++;patch({closing:true});}
  async function load(c:Context) {
    const mine=++request.current;patch({pending:true,error:null});
    try {
      const reply=await desktop.rpc<TableDeclarationSnapshot>({kind:"tableDeclarationDetail",epoch:c.epoch,source:c.source,table:c.table});
      if(mine===request.current&&current(c)&&reply.host.epoch===c.epoch)patch({snapshot:reply.data,change:null,review:null,pending:false,stale:false,outcome:null,externalVersion:desktop.surface.status.externalVersion});
    }catch(e){if(mine===request.current&&current(c))patch({pending:false,error:errorText(e),stale:true});}
  }
  async function open() {
    const expected=s.projection,input=desktop.inputIntent;
    if(s.pending||s.busy||s.status.recoveryRequired||s.uncertainField||!expected||!(await desktop.commit()))return;
    const p=desktop.surface.projection;if(!p||p.sessionEpoch!==expected.sessionEpoch||p.clicked!==expected.clicked||p.sessionEpoch!==desktop.surface.status.epoch||p.clicked!==desktop.surface.target||desktop.surface.pending)return;
    const c:Context={id:++ids.current,epoch:p.sessionEpoch,target:p.clicked,source:p.table.source,table:p.table.name,snapshot:null,change:null,review:null,pending:true,applying:false,closing:false,error:null,stale:false,uncertain:false,outcome:null,referenceKey:null,externalVersion:desktop.surface.status.externalVersion};
    focus.current={input,id:c.id};live.current=c;setContext(c);await load(c);
  }
  function change(value:TableDeclarationChange|null){request.current++;if(live.current)focus.current={id:live.current.id,input:desktop.inputIntent};
    const previous=live.current?.change;const referenceKey=value?.kind==="editReference"&&previous?.kind!=="editReference"?live.current?.snapshot?.detail.references[value.occurrence]?.selectedKey??null:value?.kind==="addReference"&&previous?.kind!=="addReference"?null:live.current?.referenceKey??null;
    patch({change:value,review:null,error:null,outcome:null,referenceKey});}
  async function review() {
    const c=live.current,d=c?.snapshot?.detail;if(!c||!d||!c.change||c.pending||c.applying||c.uncertain||!current(c))return;
    const mine=++request.current,input=desktop.inputIntent;patch({pending:true,error:null,review:null});
    try {
      const reply=await desktop.rpc<TableDeclarationReview>({kind:"tableDeclarationPlan",epoch:c.epoch,command:{table:d.table,source:d.source,identity:d.identity,change:c.change}});
      if(mine===request.current&&current(c)&&reply.host.epoch===c.epoch){focus.current={input,id:c.id};patch({review:reply.data,pending:false});}
    }catch(e){if(mine===request.current&&current(c))patch({pending:false,error:errorText(e),stale:errorText(e).includes("E-MIGRATION-STALE")});}
  }
  async function apply() {
    const c=live.current,r=c?.review;if(!c||!r||c.pending||c.applying||c.uncertain||c.stale||!current(c)||s.status.recoveryRequired)return;
    patch({applying:true,error:null});
    const key=`field:${r.plan.token}`;let received:SetResult|null=null,attempted=false;
    try {
      const inventory=await desktop.refreshInventory();if(!current(c)||!inventory)return;
      const dirty=r.plan.files.filter(f=>inventory.dirty.includes(f.source));
      for(const file of r.plan.files)if(!(await desktop.guardSource(file.source,c.epoch,"Table宣言変更前の未保存変更")))return;
      if(!current(c))return;
      if(dirty.length){patch({stale:true,error:"未保存変更を処理しました。Table detailを再取得し、現在のdeclarationからReviewしてください。"});return;}
      const removal=c.change?.kind==="removeReference"?r.before.references[c.change.occurrence]?.declaration.name:c.change?.kind==="removeSecondaryKey"?keyLabel(r.before.secondary[c.change.occurrence]):null;
      if(r.plan.destructive&&await desktop.choose(labels[c.change!.kind],`${c.table} · ${removal??"対象declaration"}を削除します。record valuesは保持します。`,["Delete","Cancel"])!=="Delete")return;
      if(!current(c))return;
      attempted=true;desktop.protectWriteView(key,true);
      const reply=await desktop.rpc<SetResult>({kind:"migrationApply",epoch:c.epoch,token:r.plan.token,authorizeDestructive:r.plan.destructive});
      received=reply.data;desktop.protectWriteView(key,false);
      if(!current(c))return;
      patch({review:null,outcome:received});
      await desktop.refreshInventory();if(desktop.surface.target===c.target)await desktop.refresh();
      if(received.outcome==="Success")await load(c);
      else patch({error:`${received.outcome}: ${received.message}`,stale:true});
    }catch(e){
      const known=!!e&&typeof e==="object"&&"code" in e;
      if(received){if(current(c))patch({review:null,outcome:received,error:`${received.outcome} · 表示を再取得できません: ${errorText(e)}`,stale:true});}
      else if(!attempted||known){desktop.protectWriteView(key,false);if(current(c))patch({error:errorText(e),stale:attempted||errorText(e).includes("E-MIGRATION-STALE")});}
      else {desktop.migrationUncertain(r.plan.token,c.epoch,c.target);if(current(c))patch({error:"Outcome Unknown: 自動再試行を止めています。Recheckでactual source setを確認してください。",uncertain:true});}
    }finally{if(current(c)){focus.current={input:desktop.inputIntent,id:c.id};patch({applying:false});}else if(live.current?.id===c.id){live.current=null;setContext(null);}}
  }
  const c=context,d=c?.snapshot?.detail,operation=c?.change;
  const blocked=!!c&&(c.pending||c.applying||c.uncertain||c.stale||s.status.recoveryRequired||s.pending||s.busy||!!s.uncertainField);
  const reference=operation&&"declaration" in operation?operation.declaration:null;
  const setReference=(input:Partial<ReferenceInput>)=>{if(operation&&"declaration" in operation)change({...operation,declaration:{...operation.declaration,...input}});};
  const newReference=():ReferenceInput=>({name:"",fields:[],targetTable:"",targetFields:[],csharpName:null});
  const footer=c&&<Space>
    <Button disabled={c.applying} onClick={close}>Close</Button>
    {c.uncertain?<Button type="primary" disabled={c.applying} onClick={()=>void desktop.recheckFieldOperation().then(()=>{if(current(c)&&!desktop.surface.uncertainField)patch({uncertain:false,review:null,stale:true,error:desktop.surface.error});})}>Recheck actual source set</Button>:<>
      {c.stale&&<Button icon={<ReloadOutlined/>} disabled={c.applying||c.pending} onClick={()=>void load(c)}>Reload Table detail</Button>}
      {operation&&!c.review&&<><Button disabled={c.pending||c.applying} onClick={()=>change(null)}>Back to detail</Button><Button aria-label="Review Table declaration change" type="primary" loading={c.pending} disabled={blocked} onClick={()=>void review()}>Review change</Button></>}
      {c.review&&<><Button disabled={c.applying} onClick={()=>patch({review:null})}>Edit input</Button><Button type="primary" danger={c.review.plan.destructive} loading={c.applying} disabled={blocked} onClick={()=>void apply()}>Apply</Button></>}
    </>}
  </Space>;
  const ordered=(label:string,values:string[],options:string[],update:(v:string[])=>void)=><Form.Item label={`${label} · 選択順`}><Select aria-label={label} mode="multiple" value={values} disabled={blocked} options={options.map(value=>({value,label:value}))} onChange={update}/></Form.Item>;
  const control=(label:string,changeValue:TableDeclarationChange,remove=false)=><Button type="text" size="small" aria-label={label} title={label} danger={remove} icon={remove?<DeleteOutlined/>:<EditOutlined/>} disabled={blocked} onClick={()=>change(changeValue)}/>;
  return {open,drawer:<Drawer title={operation?`${labels[operation.kind]} · ${c?.table}`:`Table detail · ${c?.table??""}`} open={!!c&&!c.closing&&!s.comparison&&!s.choice} size={620} closable={!c?.applying} keyboard={!c?.applying} mask={{closable:!c?.applying}} onClose={close} focusable={{focusTriggerAfterClose:false}} destroyOnHidden={false} footer={footer} afterOpenChange={visible=>{
    if(!visible){if(live.current?.closing){live.current=null;setContext(null);const r=closed.current;if(r&&r.epoch===desktop.surface.status.epoch&&r.target===desktop.surface.target&&r.input===desktop.inputIntent)document.querySelector<HTMLElement>('button[aria-label="Table actions"]')?.focus();}return;}
    const f=focus.current;if(!f||f.id!==live.current?.id||f.input!==desktop.inputIntent)return;
    focusDetail();
  }}>
    <div className="table-declarations">
      <Typography.Paragraph type="secondary">{c?.source} · saved declaration</Typography.Paragraph>
      {c?.pending&&!d?<Spin/>:d&&!operation?<Tabs size="small" destroyOnHidden items={[
        {key:"fields",label:"MessagePack keys",children:<Table size="small" rowKey="occurrence" pagination={{pageSize:20,hideOnSinglePage:true}} dataSource={d.fields.map((f,occurrence)=>({...f,occurrence}))} columns={[{title:"Field",dataIndex:"name"},{title:"Key",dataIndex:"key"},{title:"",width:44,render:(_,f)=>control(`Edit ${f.name} MessagePack key`,{kind:"setFieldKey",occurrence:f.occurrence,key:String(f.key)})}]}/>},
        {key:"keys",label:"Keys",children:<Space direction="vertical" className="declaration-section">
          <Flex align="center" justify="space-between"><span><Typography.Text strong>Primary Key</Typography.Text><Typography.Paragraph>{keyLabel(d.primary)}</Typography.Paragraph></span>{control("Edit Primary Key",{kind:"setPrimaryKey",fields:d.primary.fields})}</Flex>
          <Flex align="center" justify="space-between"><Typography.Text strong>Secondary Keys</Typography.Text><Button size="small" icon={<PlusOutlined/>} disabled={blocked} onClick={()=>change({kind:"addSecondaryKey",fields:[],nonUnique:false})}>Add Key</Button></Flex>
          <Table size="small" showHeader={false} rowKey="occurrence" pagination={{pageSize:20,hideOnSinglePage:true}} dataSource={d.secondary.map((key,occurrence)=>({...key,occurrence}))} locale={{emptyText:<Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="Secondary Keyなし"/>}} columns={[{render:(_,key)=>keyLabel(key)},{width:78,render:(_,key)=><Space size={0}>{control(`Edit Secondary Key ${key.occurrence+1}`,{kind:"editSecondaryKey",occurrence:key.occurrence,fields:key.fields,nonUnique:key.nonUnique})}{control(`Remove Secondary Key ${key.occurrence+1}`,{kind:"removeSecondaryKey",occurrence:key.occurrence},true)}</Space>}]}/>
        </Space>},
        {key:"references",label:"References",children:<Space direction="vertical" className="declaration-section"><Button size="small" icon={<PlusOutlined/>} disabled={blocked} onClick={()=>change({kind:"addReference",declaration:newReference()})}>Add Reference</Button>
          <Table size="small" showHeader={false} rowKey="occurrence" pagination={{pageSize:20,hideOnSinglePage:true}} dataSource={d.references} locale={{emptyText:<Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="Referenceなし"/>}} columns={[{render:(_,ref:ReferenceDetail)=><ReferenceSummary reference={ref}/>},{width:78,render:(_,ref:ReferenceDetail)=><Space size={0}>{control(`Edit ${ref.declaration.name} Reference`,{kind:"editReference",occurrence:ref.occurrence,declaration:ref.declaration})}{control(`Remove ${ref.declaration.name} Reference`,{kind:"removeReference",occurrence:ref.occurrence},true)}</Space>}]}/>
        </Space>},
      ]}/>:d&&operation&&!c?.review?<Form layout="vertical" className="table-declaration-operation" disabled={blocked} onKeyDown={e=>{if(e.key==="Enter"&&!e.nativeEvent.isComposing&&e.target instanceof HTMLInputElement&&e.target.getAttribute('role')!=="combobox"){e.preventDefault();void review();}}}>
        {operation.kind==="setFieldKey"&&<Form.Item label={`MessagePack key · ${d.fields[operation.occurrence]?.name}`}><Input aria-label="MessagePack key" inputMode="numeric" value={operation.key} onChange={e=>change({...operation,key:e.target.value})}/></Form.Item>}
        {"fields" in operation&&ordered("Key fields",operation.fields,d.keyFields,fields=>change({...operation,fields}))}
        {"nonUnique" in operation&&<Form.Item><Checkbox checked={operation.nonUnique} onChange={e=>change({...operation,nonUnique:e.target.checked})}>Nonunique</Checkbox></Form.Item>}
        {reference&&<><Form.Item label="Reference name"><Input aria-label="Reference name" value={reference.name} onChange={e=>setReference({name:e.target.value})}/></Form.Item>
          {ordered("Source fields",reference.fields,d.referenceFields,fields=>setReference({fields}))}
          <Form.Item label="Target Table"><Select aria-label="Reference target Table" value={reference.targetTable||undefined} options={d.targets.map(t=>({value:t.table,label:t.table}))} onChange={targetTable=>{setReference({targetTable,targetFields:[]});patch({referenceKey:null});}}/></Form.Item>
          <Form.Item label="Target Key"><Select aria-label="Reference target Key" value={c?.referenceKey??undefined} options={(d.targets.find(t=>t.table===reference.targetTable)?.keys??[]).map((key,value)=>({value,label:keyLabel(key)}))} onChange={index=>{const key=d.targets.find(t=>t.table===reference.targetTable)?.keys[index];if(key){setReference({targetFields:key.fields});patch({referenceKey:index});}}}/></Form.Item>
          <Form.Item label="C# helper override · optional"><Input aria-label="Reference helper override" value={reference.csharpName??""} placeholder="Default helper name" onChange={e=>setReference({csharpName:e.target.value||null})}/></Form.Item>
        </>}
        {(operation.kind==="removeReference"||operation.kind==="removeSecondaryKey")&&<Alert type="warning" showIcon title="対象declarationを削除" description={operation.kind==="removeReference"?d.references[operation.occurrence]?.declaration.name:keyLabel(d.secondary[operation.occurrence])}/>}
      </Form>:null}
      {c?.review&&<div aria-label="Table declaration Plan" tabIndex={-1}>
        <Descriptions size="small" column={1} items={[{key:"saved",label:"Input",children:"Saved source / captured config"},{key:"source",label:"Write source",children:c.review.plan.files.map(f=>f.source).join(", ")||"変更なし"},...(operation?.kind==="setFieldKey"?[{key:"field",label:c.review.after.fields[operation.occurrence].name,children:`key ${c.review.before.fields[operation.occurrence].key} → ${c.review.after.fields[operation.occurrence].key}`}]:[]),...(operation?.kind==="setPrimaryKey"?[{key:"primary",label:"Primary Key",children:`${keyLabel(c.review.before.primary)} → ${keyLabel(c.review.after.primary)}`}]:[]),...(operation?.kind==="addSecondaryKey"?[{key:"secondary",label:"Add Secondary Key",children:keyLabel(c.review.after.secondary.at(-1)!)}]:operation?.kind==="editSecondaryKey"?[{key:"secondary",label:`Secondary Key ${operation.occurrence+1}`,children:`${keyLabel(c.review.before.secondary[operation.occurrence])} → ${keyLabel(c.review.after.secondary[operation.occurrence])}`}]:operation?.kind==="removeSecondaryKey"?[{key:"secondary",label:"Remove Secondary Key",children:keyLabel(c.review.before.secondary[operation.occurrence])}]:[])]}/>
        {operation?.kind==="removeReference"&&<Alert type="warning" title={`Remove ${c.review.before.references[operation.occurrence].declaration.name}`}/>}
        {!!(c.review.after.references.length+c.review.after.dependents.length)&&<Table size="small" showHeader={false} rowKey={ref=>`${ref.source}:${ref.occurrence}`} pagination={{pageSize:20,hideOnSinglePage:true}} dataSource={[...c.review.after.references,...c.review.after.dependents]} columns={[{render:(_,ref:ReferenceDetail)=><ReferenceSummary reference={ref}/>}]}/>}
        {c.review.plan.files.map(file=><Flex key={file.source} justify="space-between" className="type-plan-file"><Typography.Text>{file.source} · {file.beforeBytes} → {file.afterBytes} bytes</Typography.Text><Button size="small" disabled={c.applying} onClick={()=>void desktop.migrationCompare(c.review!.plan.token,file.source,c.review!.plan.files.map(f=>f.source)).catch(desktop.showError)}>Compare</Button></Flex>)}
      </div>}
      {!!(c?.review?.dirtyDependencies.length||c?.snapshot?.dirtySources.length||c?.review?.configDirty||c?.snapshot?.configDirty)&&<Alert type="warning" showIcon title="未保存変更はPlanに含めません" description={<Typography.Text>{[...(c?.review?.dirtyDependencies??c?.snapshot?.dirtySources??[]),...(c?.review?.configDirty||c?.snapshot?.configDirty?["Project Settings"]:[])].join(", ")}</Typography.Text>}/>}
      {!!d?.diagnostics.length&&!c?.review&&<Alert type="warning" showIcon title="Declaration Problems" description={d.diagnostics.slice(0,8).map((p,i)=><div key={i}>{p.code}: {p.message}</div>)}/>}
      {c?.error&&<Alert type={c.uncertain?"warning":"error"} showIcon title="変更を適用できません" description={c.error}/>}
    </div>
  </Drawer>};
}
function ReferenceSummary({reference:r}:{reference:ReferenceDetail}) {
  return <Flex vertical gap="small" className="type-plan-file"><Typography.Text strong>{r.table} · {r.declaration.name}</Typography.Text><Typography.Text type="secondary">{r.declaration.fields.join(" → ")} → {r.declaration.targetTable} ({r.declaration.targetFields.join(" → ")})</Typography.Text><Space><Tag>{r.helper}</Tag>{r.multi!==null&&<Tag>{r.multi?"Many":"One"}</Tag>}{r.optional!==null&&<Tag>{r.optional?"Optional":"Required"}</Tag>}</Space>{r.problem&&<Typography.Text type="danger">{r.problem}</Typography.Text>}</Flex>;
}
