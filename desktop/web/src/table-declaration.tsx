import { uiMessage, statusLabel, uiTerms } from "./language";
import {useEffect, useLayoutEffect, useRef, useState} from "react";
import {Alert, Button, Checkbox, Descriptions, Drawer, Empty, Flex, Form, Input, Select, Space, Spin, Table, Tabs, Tag, Typography} from "antd";
import {DeleteOutlined, EditOutlined, PlusOutlined, ReloadOutlined} from "@ant-design/icons";
import {desktop, type Surface} from "./workspace";
import type {DeclarationKey, ReferenceDetail, ReferenceInput, SetResult, TableDeclarationChange, TableDeclarationReview, TableDeclarationSnapshot} from "./types";

type Context={id:number;epoch:number;target:string;source:string;table:string;snapshot:TableDeclarationSnapshot|null;change:TableDeclarationChange|null;review:TableDeclarationReview|null;pending:boolean;applying:boolean;closing:boolean;error:string|null;stale:boolean;uncertain:boolean;outcome:SetResult|null;referenceKey:number|null;externalVersion:number};
const errorText=(e:unknown)=>e&&typeof e==="object"&&"message" in e?`${"code" in e?String(e.code)+": ":""}${String(e.message)}`:String(e);
const labels:Record<TableDeclarationChange["kind"],string>={setFieldKey:"MessagePackキー",setPrimaryKey:"主キー",addSecondaryKey:"副キーを追加",editSecondaryKey:"副キーを編集",removeSecondaryKey:"副キーを削除",addReference:"参照を追加",editReference:"参照を編集",removeReference:"参照を削除"};
const keyLabel=(key:DeclarationKey)=>`${key.fields.join(" → ")}${key.nonUnique?" · 重複許可":" · 一意"}`;
export function useTableDeclaration(s:Surface) {
  const [context,setContext]=useState<Context|null>(null),live=useRef(context);live.current=context;
  const request=useRef(0),ids=useRef(0),closed=useRef<{epoch:number;target:string;input:number}|null>(null);
  const focus=useRef<{input:number;id:number}|null>(null);
  const focusDetail=()=>{const root=document.querySelector('.ant-drawer-open');(root?.querySelector<HTMLElement>("[aria-label=\"テーブル宣言の変更計画\"],.table-declaration-operation input:not(:disabled)")??root?.querySelector<HTMLElement>("button[aria-label=\"テーブル宣言の変更を確認\"]:not(:disabled)")??root?.querySelector<HTMLElement>('.table-declarations button:not(:disabled)'))?.focus();};
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
  useEffect(()=>{const c=live.current;if(c&&!c.applying&&c.externalVersion!==s.status.externalVersion)patch({stale:true,error:"ソースが外部で変更されています。取得済みの宣言を保持しています。現在のテーブル定義を読み直してください。"});},[s.status.externalVersion]);
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
      for(const file of r.plan.files)if(!(await desktop.guardSource(file.source,c.epoch,"テーブル宣言変更前の未保存変更")))return;
      if(!current(c))return;
      if(dirty.length){patch({stale:true,error:"未保存変更を処理しました。テーブル定義を読み直し、現在の宣言から確認してください。"});return;}
      const removal=c.change?.kind==="removeReference"?r.before.references[c.change.occurrence]?.declaration.name:c.change?.kind==="removeSecondaryKey"?keyLabel(r.before.secondary[c.change.occurrence]):null;
      if(r.plan.destructive&&await desktop.choose(labels[c.change!.kind],`${c.table} · ${removal??"対象宣言"}を削除します。レコードの値は保持します。`,["Delete","Cancel"])!=="Delete")return;
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
      if(received){if(current(c))patch({review:null,outcome:received,error:`${statusLabel(received.outcome)} · 表示を再取得できません: ${errorText(e)}`,stale:true});}
      else if(!attempted||known){desktop.protectWriteView(key,false);if(current(c))patch({error:errorText(e),stale:attempted||errorText(e).includes("E-MIGRATION-STALE")});}
      else {desktop.migrationUncertain(r.plan.token,c.epoch,c.target);if(current(c))patch({error:"Outcome Unknown: 自動再試行を止めています。再確認で現在のソース一式を確認してください。",uncertain:true});}
    }finally{if(current(c)){focus.current={input:desktop.inputIntent,id:c.id};patch({applying:false});}else if(live.current?.id===c.id){live.current=null;setContext(null);}}
  }
  const c=context,d=c?.snapshot?.detail,operation=c?.change;
  const blocked=!!c&&(c.pending||c.applying||c.uncertain||c.stale||s.status.recoveryRequired||s.pending||s.busy||!!s.uncertainField);
  const reference=operation&&"declaration" in operation?operation.declaration:null;
  const setReference=(input:Partial<ReferenceInput>)=>{if(operation&&"declaration" in operation)change({...operation,declaration:{...operation.declaration,...input}});};
  const newReference=():ReferenceInput=>({name:"",fields:[],targetTable:"",targetFields:[],csharpName:null});
  const footer=c&&<Space>
    <Button disabled={c.applying} onClick={close}>閉じる</Button>
    {c.uncertain?<Button type="primary" aria-label="状態を再確認（対象ソース一式）" disabled={c.applying} onClick={()=>void desktop.recheckFieldOperation().then(()=>{if(current(c)&&!desktop.surface.uncertainField)patch({uncertain:false,review:null,stale:true,error:desktop.surface.error});})}>{uiTerms.recheck}</Button>:<>
      {c.stale&&<Button icon={<ReloadOutlined aria-hidden="true"/>} disabled={c.applying||c.pending} onClick={()=>void load(c)}>テーブル定義を再取得</Button>}
      {operation&&!c.review&&<><Button disabled={c.pending||c.applying} onClick={()=>change(null)}>定義に戻る</Button><Button aria-label="テーブル宣言の変更を確認" type="primary" loading={c.pending} disabled={blocked} onClick={()=>void review()}>変更を確認</Button></>}
      {c.review&&<><Button disabled={c.applying} onClick={()=>patch({review:null})}>入力を修正</Button><Button type="primary" danger={c.review.plan.destructive} loading={c.applying} disabled={blocked} onClick={()=>void apply()}>適用</Button></>}
    </>}
  </Space>;
  const ordered=(label:string,values:string[],options:string[],update:(v:string[])=>void)=><Form.Item label={`${label} · 選択順`}><Select aria-label={label} mode="multiple" value={values} disabled={blocked} options={options.map(value=>({value,label:value}))} onChange={update}/></Form.Item>;
  const control=(label:string,changeValue:TableDeclarationChange,remove=false)=><Button type="text" size="small" aria-label={label} title={label} danger={remove} icon={remove?<DeleteOutlined aria-hidden="true"/>:<EditOutlined aria-hidden="true"/>} disabled={blocked} onClick={()=>change(changeValue)}/>;
  return {open,drawer:<Drawer title={operation?`${labels[operation.kind]} · ${c?.table}`:`${uiTerms.tableDefinition} · ${c?.table??""}`} open={!!c&&!c.closing&&!s.comparison&&!s.choice} size={620} closable={!c?.applying} keyboard={!c?.applying} mask={{closable:!c?.applying}} onClose={close} focusable={{focusTriggerAfterClose:false}} destroyOnHidden={false} footer={footer} afterOpenChange={visible=>{
    if(!visible){if(live.current?.closing){live.current=null;setContext(null);const r=closed.current;if(r&&r.epoch===desktop.surface.status.epoch&&r.target===desktop.surface.target&&r.input===desktop.inputIntent)document.querySelector<HTMLElement>("button[aria-label=\"テーブルの操作\"]")?.focus();}return;}
    const f=focus.current;if(!f||f.id!==live.current?.id||f.input!==desktop.inputIntent)return;
    focusDetail();
  }}>
    <div className="table-declarations">{c?.uncertain&&<Typography.Paragraph>{uiTerms.sourceRecheckDescription}</Typography.Paragraph>}
      <Typography.Paragraph type="secondary">{c?.source}  · 保存済みの宣言</Typography.Paragraph>
      {c?.pending&&!d?<Spin/>:d&&!operation?<Tabs size="small" destroyOnHidden items={[
        {key:"fields",label:"MessagePackキー",children:<Table size="small" rowKey="occurrence" pagination={{pageSize:20,hideOnSinglePage:true}} dataSource={d.fields.map((f,occurrence)=>({...f,occurrence}))} columns={[{title:"フィールド",dataIndex:"name"},{title:"キー",dataIndex:"key"},{title:"",width:44,render:(_,f)=>control(`編集: ${f.name} MessagePackキー`,{kind:"setFieldKey",occurrence:f.occurrence,key:String(f.key)})}]}/>},
        {key:"keys",label:"キー",children:<Space direction="vertical" className="declaration-section">
          <Flex align="center" justify="space-between"><span><Typography.Text strong>主キー</Typography.Text><Typography.Paragraph>{keyLabel(d.primary)}</Typography.Paragraph></span>{control("主キーを編集",{kind:"setPrimaryKey",fields:d.primary.fields})}</Flex>
          <Flex align="center" justify="space-between"><Typography.Text strong>副キー</Typography.Text><Button size="small" icon={<PlusOutlined aria-hidden="true"/>} disabled={blocked} onClick={()=>change({kind:"addSecondaryKey",fields:[],nonUnique:false})}>キーを追加</Button></Flex>
          <Table size="small" showHeader={false} rowKey="occurrence" pagination={{pageSize:20,hideOnSinglePage:true}} dataSource={d.secondary.map((key,occurrence)=>({...key,occurrence}))} locale={{emptyText:<Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="副キーなし"/>}} columns={[{render:(_,key)=>keyLabel(key)},{width:78,render:(_,key)=><Space size={0}>{control(`副キーを編集: ${key.occurrence+1}`,{kind:"editSecondaryKey",occurrence:key.occurrence,fields:key.fields,nonUnique:key.nonUnique})}{control(`副キーを削除: ${key.occurrence+1}`,{kind:"removeSecondaryKey",occurrence:key.occurrence},true)}</Space>}]}/>
        </Space>},
        {key:"references",label:"参照",children:<Space direction="vertical" className="declaration-section"><Button size="small" icon={<PlusOutlined aria-hidden="true"/>} disabled={blocked} onClick={()=>change({kind:"addReference",declaration:newReference()})}>参照を追加</Button>
          <Table size="small" showHeader={false} rowKey="occurrence" pagination={{pageSize:20,hideOnSinglePage:true}} dataSource={d.references} locale={{emptyText:<Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="参照なし"/>}} columns={[{render:(_,ref:ReferenceDetail)=><ReferenceSummary reference={ref}/>},{width:78,render:(_,ref:ReferenceDetail)=><Space size={0}>{control(`編集: ${ref.declaration.name} 参照`,{kind:"editReference",occurrence:ref.occurrence,declaration:ref.declaration})}{control(`削除: ${ref.declaration.name} 参照`,{kind:"removeReference",occurrence:ref.occurrence},true)}</Space>}]}/>
        </Space>},
      ]}/>:d&&operation&&!c?.review?<Form layout="vertical" className="table-declaration-operation" disabled={blocked} onKeyDown={e=>{if(e.key==="Enter"&&!e.nativeEvent.isComposing&&e.target instanceof HTMLInputElement&&e.target.getAttribute('role')!=="combobox"){e.preventDefault();void review();}}}>
        {operation.kind==="setFieldKey"&&<Form.Item label={`MessagePackキー · ${d.fields[operation.occurrence]?.name}`}><Input aria-label="MessagePackキー" inputMode="numeric" value={operation.key} onChange={e=>change({...operation,key:e.target.value})}/></Form.Item>}
        {"fields" in operation&&ordered("キーのフィールド",operation.fields,d.keyFields,fields=>change({...operation,fields}))}
        {"nonUnique" in operation&&<Form.Item><Checkbox checked={operation.nonUnique} onChange={e=>change({...operation,nonUnique:e.target.checked})}>重複を許可</Checkbox></Form.Item>}
        {reference&&<><Form.Item label="参照名"><Input aria-label="参照名" value={reference.name} onChange={e=>setReference({name:e.target.value})}/></Form.Item>
          {ordered("参照元のフィールド",reference.fields,d.referenceFields,fields=>setReference({fields}))}
          <Form.Item label="参照先のテーブル"><Select aria-label="参照先のテーブル" value={reference.targetTable||undefined} options={d.targets.map(t=>({value:t.table,label:t.table}))} onChange={targetTable=>{setReference({targetTable,targetFields:[]});patch({referenceKey:null});}}/></Form.Item>
          <Form.Item label="参照先のキー"><Select aria-label="参照先のキー" value={c?.referenceKey??undefined} options={(d.targets.find(t=>t.table===reference.targetTable)?.keys??[]).map((key,value)=>({value,label:keyLabel(key)}))} onChange={index=>{const key=d.targets.find(t=>t.table===reference.targetTable)?.keys[index];if(key){setReference({targetFields:key.fields});patch({referenceKey:index});}}}/></Form.Item>
          <Form.Item label="C#ヘルパー名の指定（任意）"><Input aria-label="参照ヘルパー名の指定" value={reference.csharpName??""} placeholder="既定のヘルパー名" onChange={e=>setReference({csharpName:e.target.value||null})}/></Form.Item>
        </>}
        {(operation.kind==="removeReference"||operation.kind==="removeSecondaryKey")&&<Alert type="warning" showIcon title="対象宣言を削除" description={operation.kind==="removeReference"?d.references[operation.occurrence]?.declaration.name:keyLabel(d.secondary[operation.occurrence])}/>}
      </Form>:null}
      {c?.review&&<div aria-label="テーブル宣言の変更計画" tabIndex={-1}>
        <Descriptions size="small" column={1} items={[{key:"saved",label:"入力",children:"保存済みソース・取得時点の設定"},{key:"source",label:"書き換えるソース",children:c.review.plan.files.map(f=>f.source).join(", ")||"変更なし"},...(operation?.kind==="setFieldKey"?[{key:"field",label:c.review.after.fields[operation.occurrence].name,children:`キー ${c.review.before.fields[operation.occurrence].key} → ${c.review.after.fields[operation.occurrence].key}`}]:[]),...(operation?.kind==="setPrimaryKey"?[{key:"primary",label:"主キー",children:`${keyLabel(c.review.before.primary)} → ${keyLabel(c.review.after.primary)}`}]:[]),...(operation?.kind==="addSecondaryKey"?[{key:"secondary",label:"副キーを追加",children:keyLabel(c.review.after.secondary.at(-1)!)}]:operation?.kind==="editSecondaryKey"?[{key:"secondary",label:`副キー ${operation.occurrence+1}`,children:`${keyLabel(c.review.before.secondary[operation.occurrence])} → ${keyLabel(c.review.after.secondary[operation.occurrence])}`}]:operation?.kind==="removeSecondaryKey"?[{key:"secondary",label:"副キーを削除",children:keyLabel(c.review.before.secondary[operation.occurrence])}]:[])]}/>
        {operation?.kind==="removeReference"&&<Alert type="warning" title={`削除: ${c.review.before.references[operation.occurrence].declaration.name}`}/>}
        {!!(c.review.after.references.length+c.review.after.dependents.length)&&<Table size="small" showHeader={false} rowKey={ref=>`${ref.source}:${ref.occurrence}`} pagination={{pageSize:20,hideOnSinglePage:true}} dataSource={[...c.review.after.references,...c.review.after.dependents]} columns={[{render:(_,ref:ReferenceDetail)=><ReferenceSummary reference={ref}/>}]}/>}
        {c.review.plan.files.map(file=><Flex key={file.source} justify="space-between" className="type-plan-file"><Typography.Text>{file.source} · {file.beforeBytes} → {file.afterBytes} バイト</Typography.Text><Button size="small" disabled={c.applying} onClick={()=>void desktop.migrationCompare(c.review!.plan.token,file.source,c.review!.plan.files.map(f=>f.source)).catch(desktop.showError)}>比較</Button></Flex>)}
      </div>}
      {!!(c?.review?.dirtyDependencies.length||c?.snapshot?.dirtySources.length||c?.review?.configDirty||c?.snapshot?.configDirty)&&<Alert type="warning" showIcon title="未保存変更は変更計画に含めません" description={<Typography.Text>{[...(c?.review?.dirtyDependencies??c?.snapshot?.dirtySources??[]),...(c?.review?.configDirty||c?.snapshot?.configDirty?["プロジェクト設定"]:[])].join(", ")}</Typography.Text>}/>}
      {!!d?.diagnostics.length&&!c?.review&&<Alert type="warning" showIcon title="宣言の問題" description={d.diagnostics.slice(0,8).map((p,i)=><div key={i}>{p.code}: {uiMessage(p.message)}</div>)}/>}
      {c?.error&&<Alert type={c.uncertain?"warning":"error"} showIcon title="変更を適用できません" description={uiMessage(c.error)}/>}
    </div>
  </Drawer>};
}
function ReferenceSummary({reference:r}:{reference:ReferenceDetail}) {
  return <Flex vertical gap="small" className="type-plan-file"><Typography.Text strong>{r.table} · {r.declaration.name}</Typography.Text><Typography.Text type="secondary">{r.declaration.fields.join(" → ")} → {r.declaration.targetTable} ({r.declaration.targetFields.join(" → ")})</Typography.Text><Space><Tag>{r.helper}</Tag>{r.multi!==null&&<Tag>{r.multi?"複数":"単一"}</Tag>}{r.optional!==null&&<Tag>{r.optional?"任意":"必須"}</Tag>}</Space>{r.problem&&<Typography.Text type="danger">{uiMessage(r.problem)}</Typography.Text>}</Flex>;
}
