import {useEffect, useLayoutEffect, useRef, useState, type ReactNode} from "react";
import {Alert, Button, Checkbox, Descriptions, Dropdown, Flex, Form, Input, Modal, Radio, Select, Space, Spin, Table, Tag, Typography, type MenuProps} from "antd";
import {DeleteOutlined, EditOutlined, MoreOutlined, PlusOutlined, SettingOutlined} from "@ant-design/icons";
import {desktop, basename, type Surface} from "./workspace";
import type {ConstantInput, DeclarationInput, MigrationReview, SetResult, Shape, TypeOperation, TypeProjection} from "./types";
import {TypedInitializer} from "./initializer";
type Draft={id:number;target:TypeProjection;command:TypeOperation;input:ConstantInput;shape:Shape|null;shapeError:string|null;review:MigrationReview|null;pending:boolean;applying:boolean;closing:boolean;error:string|null;outcome:SetResult|null;stale:boolean;uncertain:boolean;choices:string[]};
const message=(error:unknown)=>error&&typeof error==="object"&&"message" in error?`${"code" in error?String(error.code)+": ":""}${String(error.message)}`:String(error);
const labels:Record<TypeOperation["operation"],string>={setValueObjectConversions:"Conversion settings",addEnumMember:"Add member",renameEnumMember:"Rename member",dropEnumMember:"Drop member",addCustomField:"Add field",renameCustomField:"Rename field",dropCustomField:"Drop field"};
function targetLabel(c:TypeOperation){return "member" in c?c.member:"field" in c?c.field:"name" in c?c.name:"declaration" in c?c.declaration.name:c.typeName;}
function changeLabel(c:TypeOperation){const target=targetLabel(c);return "newName" in c?`${target} → ${c.newName}`:c.operation==="addEnumMember"?`${target} = ${c.value}`:c.operation==="addCustomField"?`${target} · ${c.declaration.typeName}${c.declaration.array?"[]":c.declaration.nullable?"?":""} · key ${c.declaration.key}`:target;}

export function TypeSurface({s,footer}:{s:Surface;footer:ReactNode}) {
  const p=s.typeProjection,ready=p&&!s.pending&&p.clicked===s.target;
  const body=useRef<HTMLDivElement>(null),origin=useRef<{element:HTMLElement;label:string|null;source:string;epoch:number}|null>(null);
  const [height,setHeight]=useState(420),[draft,setDraft]=useState<Draft|null>(null);
  const live=useRef(draft);live.current=draft;
  const ids=useRef(0),version=useRef(0),shapeVersion=useRef(0);
  const planFocus=useRef<{token:string;intent:number}|null>(null);
  const inputFocus=useRef<{id:number;intent:number}|null>(null);
  const wasOverlay=useRef(false);
  const returnFocus=useRef<{element:HTMLElement;label:string|null;source:string;epoch:number;intent:number}|null>(null);
  function restoreOrigin() {
    const saved=returnFocus.current;
    if(!saved)return;
    if(saved.epoch!==desktop.surface.status.epoch||saved.source!==desktop.surface.target||saved.intent!==desktop.inputIntent){returnFocus.current=null;return;}
    if(desktop.surface.pending)return;
    const restored=(saved.label?[...document.querySelectorAll<HTMLElement>('[aria-label]')].find(el=>el.getAttribute('aria-label')===saved.label):null)??(saved.element.isConnected?saved.element:null);
    if(!restored||!restored.getClientRects().length||restored instanceof HTMLButtonElement&&restored.disabled)return;
    restored.focus({preventScroll:true});
    if(document.activeElement===restored)returnFocus.current=null;
  }
  function focusPlan(element=document.querySelector<HTMLElement>('[aria-label="Type Migration Plan"]')) {
    const current=live.current;
    if(element?.isConnected&&current?.review&&planFocus.current?.token===current.review.token&&planFocus.current.intent===desktop.inputIntent&&current.target.clicked===desktop.surface.target&&!desktop.surface.comparison&&!desktop.surface.choice)element.focus();
  }
  const patch=(change:Partial<Draft>)=>{if(live.current){live.current={...live.current,...change};setDraft(live.current);}};
  useLayoutEffect(()=>{if(p&&ready&&body.current){desktop.committedType(p,body.current);if(document.activeElement===document.getElementById('editor-pane'))body.current.focus({preventScroll:true});}},[p,ready]);
  useLayoutEffect(()=>{if(!draft&&ready){const frame=requestAnimationFrame(restoreOrigin);return()=>cancelAnimationFrame(frame);}},[p,ready,draft]);
  useLayoutEffect(()=>{
    const overlay=!!s.comparison||!!s.choice;
    if(wasOverlay.current&&!overlay&&live.current?.review) {
      planFocus.current={token:live.current.review.token,intent:desktop.inputIntent};
      const frame=requestAnimationFrame(()=>focusPlan());
      wasOverlay.current=overlay;return()=>cancelAnimationFrame(frame);
    }
    wasOverlay.current=overlay;
  },[s.comparison,s.choice]);
  useEffect(()=>{const element=body.current;if(!element)return;const observer=new ResizeObserver(()=>setHeight(Math.max(160,element.clientHeight-122)));observer.observe(element);return()=>observer.disconnect();},[]);
  useEffect(()=>{ids.current++;version.current++;shapeVersion.current++;live.current=null;setDraft(null);},[s.status.epoch]);
  function close() {
    if(live.current?.applying)return;
    if(origin.current)returnFocus.current={...origin.current,intent:desktop.inputIntent};
    version.current++;patch({closing:true});
  }
  async function begin(command:TypeOperation, originLabel?:string) {
    if(!p||!desktop.currentType(p)||s.status.recoveryRequired||s.uncertainField)return;
    const el=document.activeElement as HTMLElement;
    origin.current={element:el,label:originLabel??el.getAttribute('aria-label'),source:p.clicked,epoch:p.sessionEpoch};
    const id=++ids.current;
    inputFocus.current={id,intent:desktop.inputIntent};
    const next:Draft={id,target:p,command,input:{kind:"unset"},shape:null,shapeError:null,review:null,pending:false,applying:false,closing:false,error:null,outcome:null,stale:false,uncertain:false,choices:[]};
    live.current=next;setDraft(next);
    if(command.operation==="addCustomField") {
      try {
        const choices=await desktop.rpc<{fieldTypes:string[]}>({kind:"creationChoices",epoch:p.sessionEpoch});
        if(live.current?.id===id&&p.sessionEpoch===desktop.surface.status.epoch)patch({choices:choices.data.fieldTypes});
      }catch(error){if(live.current?.id===id)patch({error:message(error)});}
    }
  }
  async function addField() {
    if(!p||p.declaration.category!=="custom"||!desktop.currentType(p))return;
    try {
      const reply=await desktop.rpc<DeclarationInput>({kind:"creationField",epoch:p.sessionEpoch,fields:p.declaration.fields.map(f=>({...f,key:String(f.key)}))});
      if(desktop.currentType(p))await begin({operation:"addCustomField",typeName:p.name,declaration:reply.data,initializer:null},"Add field");
    }catch(error){desktop.showError(error);}
  }
  function change(command:TypeOperation,input=live.current?.input??{kind:"unset"} as ConstantInput) {
    version.current++;patch({command,input,review:null,error:null,outcome:null,stale:false});
  }
  function fieldChange(changeInput:Partial<DeclarationInput>) {
    const c=live.current?.command;if(c?.operation!=="addCustomField")return;
    const changesType=("typeName" in changeInput&&changeInput.typeName!==c.declaration.typeName)||("nullable" in changeInput&&changeInput.nullable!==c.declaration.nullable)||("array" in changeInput&&changeInput.array!==c.declaration.array);
    change({...c,declaration:{...c.declaration,...changeInput}},changesType?{kind:"unset"}:live.current!.input);
    if(changesType)patch({shape:null,shapeError:null});
  }
  const field=draft?.command.operation==="addCustomField"?draft.command.declaration:null;
  useEffect(()=>{
    const current=live.current;if(!current||current.command.operation!=="addCustomField")return;
    const mine=++shapeVersion.current;
    void desktop.rpc<Shape>({kind:"typeInitializer",epoch:current.target.sessionEpoch,source:current.target.source,identity:current.target.identity,declaration:current.command.declaration}).then(reply=>{
      if(mine===shapeVersion.current&&live.current?.id===current.id&&reply.host.epoch===desktop.surface.status.epoch)patch({shape:reply.data,shapeError:null});
    }).catch(error=>{if(mine===shapeVersion.current&&live.current?.id===current.id)patch({shape:null,shapeError:message(error)});});
    return()=>{shapeVersion.current++;};
  },[draft?.id,field?.typeName,field?.nullable,field?.array]);
  async function plan(replan=false) {
    const current=live.current;if(!current||current.applying||current.uncertain||current.pending)return;
    const mine=++version.current;
    const intent=desktop.inputIntent;
    patch({pending:true,review:null,error:null,outcome:null});
    try {
      let target=current.target;
      if(replan) {
        await desktop.refresh();
        const fresh=desktop.surface.typeProjection;
        if(!fresh||fresh.source!==target.source||fresh.name!==target.name||fresh.sessionEpoch!==target.sessionEpoch)throw new Error("Typeのbindingが変わりました。現在のdeclarationから操作を開始してください。");
        target=fresh;
      }
      const reply=await desktop.rpc<MigrationReview>({kind:"typeMigrationPlan",epoch:target.sessionEpoch,source:target.source,identity:target.identity,command:current.command,input:current.command.operation==="addCustomField"&&current.input.kind!=="unset"?current.input:null});
      if(mine===version.current&&live.current?.id===current.id&&reply.host.epoch===desktop.surface.status.epoch){planFocus.current={token:reply.data.token,intent};patch({target,review:reply.data,pending:false,stale:false});}
    }catch(error){if(mine===version.current&&live.current?.id===current.id)patch({pending:false,error:message(error),stale:message(error).includes("E-MIGRATION-STALE")});}
  }
  async function apply() {
    const current=live.current,review=current?.review;
    if(!current||!review||current.applying||current.pending||current.uncertain||current.stale||desktop.surface.target!==current.target.clicked||current.target.sessionEpoch!==desktop.surface.status.epoch)return;
    const dirty=review.files.filter(file=>desktop.surface.status.dirty.includes(file.source));
    if(dirty.length){patch({error:`未保存のsourceを先に確認してください: ${dirty.map(f=>f.source).join(", ")}`});return;}
    if(review.destructive) {
      const answer=await desktop.choose(`Drop ${targetLabel(current.command)}`,`${current.target.name}の${targetLabel(current.command)}と対象の値を削除します。`,["Delete","Cancel"]);
      if(answer!=="Delete"||live.current!==current||current.target.sessionEpoch!==desktop.surface.status.epoch)return;
    }
    const inputIntent=desktop.inputIntent,key=`field:${review.token}`;
    let received:SetResult|null=null;
    patch({applying:true,error:null});desktop.protectWriteView(key,true);
    try {
      const reply=await desktop.rpc<SetResult>({kind:"migrationApply",epoch:current.target.sessionEpoch,token:review.token,authorizeDestructive:review.destructive});
      received=reply.data;
      desktop.protectWriteView(key,false);
      if(current.target.sessionEpoch!==desktop.surface.status.epoch)return;
      patch({applying:false,outcome:reply.data,review:null});
      await desktop.refreshInventory();
      if(desktop.surface.target===current.target.clicked)await desktop.refresh();
      if(reply.data.outcome==="Success") {
        if(origin.current) {
          const operation=current.command;
          if(operation.operation==="renameEnumMember"||operation.operation==="renameCustomField")origin.current.label=`${operation.newName} ${operation.operation==="renameEnumMember"?"member":"field"} actions`;
          if(operation.operation==="dropEnumMember"||operation.operation==="dropCustomField")origin.current.label=operation.operation==="dropEnumMember"?"Add member":"Add field";
        }
        if(inputIntent===desktop.inputIntent)close();
      }
      else patch({error:`${reply.data.outcome}: ${reply.data.message}`,stale:true});
    }catch(error){
      if(received) {if(current.target.sessionEpoch===desktop.surface.status.epoch&&live.current?.id===current.id)patch({applying:false,outcome:received,review:null,error:`${received.outcome} · 表示の再取得に失敗しました: ${message(error)}`,stale:true});return;}
      const known=!!error&&typeof error==="object"&&"code" in error;
      if(known)desktop.protectWriteView(key,false);
      else desktop.migrationUncertain(review.token,current.target.sessionEpoch,current.target.clicked);
      if(current.target.sessionEpoch===desktop.surface.status.epoch&&live.current?.id===current.id)patch({applying:false,error:known?message(error):"Outcome Unknown: 結果を受信できません。Recheckでactual source setを確認してください。",stale:known,uncertain:!known});
    }
  }
  async function recheck() {
    await desktop.recheckFieldOperation();
    if(!desktop.surface.uncertainField)patch({uncertain:false,review:null,stale:true});
  }
  function actions(name:string,member:boolean):MenuProps {
    const protectedMember=member&&p?.protectedMembers.includes(name);
    return {items:protectedMember?[]:[{key:"rename",label:"Rename…",icon:<EditOutlined/>},{key:"drop",label:"Drop…",danger:true,icon:<DeleteOutlined/>}],onKeyDown:event=>{
      // Ant Menu 6.6.5 activates on keydown without consuming the native default.
      // WKWebView's remaining Enter activation can close the newly opened Modal.
      // Consume the handled Enter, keeping Menu's normal path and key-only input.
      // Composition Enter remains text input; the native Type workflow exercises
      // both Enter representations and Cancel with source/focus preservation.
      if(event.key==="Enter"&&!event.nativeEvent.isComposing&&event.target instanceof HTMLElement) {
        const item=event.target.closest<HTMLElement>('[role="menuitem"]');
        if(item&&item.getAttribute('aria-disabled')!=="true"){event.preventDefault();if(event.which!==13)item.click();}
      }
    },onClick:({key})=>{
      if(!p)return;
      void begin(member?(key==="rename"?{operation:"renameEnumMember",typeName:p.name,member:name,newName:name}:{operation:"dropEnumMember",typeName:p.name,member:name}):(key==="rename"?{operation:"renameCustomField",typeName:p.name,field:name,newName:name}:{operation:"dropCustomField",typeName:p.name,field:name}),`${name} ${member?"member":"field"} actions`);
    }};
  }
  const disabled=!ready||s.busy||s.status.recoveryRequired||!!s.uncertainField;
  const declaration=ready?p.declaration:null;
  const typeLabel=declaration?.category==="enum"?(declaration.flags?"Flags Enum":"Enum"):declaration?.category==="custom"?"Custom Type":"Value Object";
  const dirty=draft?.review?.files.filter(file=>s.status.dirty.includes(file.source)).map(file=>file.source)??[];
  const command=draft?.command;
  return <section id="type-surface">
    <div className="type-context"><Typography.Text strong>{ready?p.name:basename(s.target)}</Typography.Text><Typography.Text type="secondary" className="selection-label">{s.target}</Typography.Text>{declaration&&<Tag>{typeLabel}</Tag>}</div>
    <div className="type-body" ref={body} tabIndex={0} aria-label="Type Editor" aria-busy={s.pending} onFocus={()=>{if(p)desktop.acceptedType(p);}} onKeyDown={()=>{if(p)desktop.acceptedType(p);}}>
      {s.pending?<div className="type-pending" role="status"><Spin size="small"/><span>{basename(s.target)}を開いています…</span></div>:!declaration?<Alert type="error" showIcon title="Typeを開けません" description={s.error}/>:<>
        <div className="type-summary"><Descriptions size="small" column={2} items={[{key:"name",label:"Name",children:p!.name},{key:"category",label:"Category",children:typeLabel},...("underlying" in declaration?[{key:"underlying",label:"Underlying",children:declaration.underlying}]:[])]}/></div>
        {declaration.category==="valueObject"?<>
          <div className="type-list-heading"><Typography.Text strong>Conversions</Typography.Text><Button icon={<SettingOutlined/>} aria-label="Edit conversions" disabled={disabled} onClick={()=>void begin({operation:"setValueObjectConversions",typeName:p!.name,fromImplicit:declaration.from_implicit,toImplicit:declaration.to_implicit},"Edit conversions")}>Edit conversions</Button></div>
          <Descriptions size="small" column={1} items={[{key:"from",label:"From underlying",children:declaration.from_implicit?"Implicit":"Explicit"},{key:"to",label:"To underlying",children:declaration.to_implicit?"Implicit":"Explicit"}]}/>
        </>:<>
          <div className="type-list-heading"><Typography.Text strong>{declaration.category==="enum"?"Members":"Fields"}</Typography.Text><Button icon={<PlusOutlined/>} aria-label={declaration.category==="enum"?"Add member":"Add field"} disabled={disabled} onClick={()=>{if(declaration.category==="enum")void begin({operation:"addEnumMember",typeName:p!.name,name:"",value:""},"Add member");else void addField();}}>{declaration.category==="enum"?"Add member":"Add field"}</Button></div>
          {declaration.category==="enum"?<Table virtual size="small" pagination={false} scroll={{x:660,y:height}} rowKey="name" dataSource={declaration.members.map(([name,value])=>({name,value}))} columns={[{title:"Name",dataIndex:"name",width:330},{title:"Value",dataIndex:"value",width:260,render:value=><span className="type-numeric">{value}</span>},{title:"",width:70,render:(_,row)=>!p!.protectedMembers.includes(row.name)&&<Dropdown menu={actions(row.name,true)} trigger={["click"]}><Button type="text" disabled={disabled} icon={<MoreOutlined/>} aria-label={`${row.name} member actions`}/></Dropdown>}]}/>:<Table virtual size="small" pagination={false} scroll={{x:720,y:height}} rowKey="name" dataSource={declaration.fields} columns={[{title:"Key",dataIndex:"key",width:70},{title:"Name",dataIndex:"name",width:240},{title:"Type",dataIndex:"typeName",width:240},{title:"Modifier",width:100,render:(_,row)=>row.array?"Array":row.nullable?"Nullable":"—"},{title:"",width:70,render:(_,row)=><Dropdown menu={actions(row.name,false)} trigger={["click"]}><Button type="text" disabled={disabled} icon={<MoreOutlined/>} aria-label={`${row.name} field actions`}/></Dropdown>}]}/>}
        </>}
      </>}
      {s.uncertainField&&<Alert type="warning" showIcon title="構造変更の結果を確認できません" description={<Button onClick={()=>void desktop.recheckFieldOperation()}>Recheck actual source set</Button>}/>}
    </div>
    {footer}
    {draft&&<Modal open={!draft.closing&&!s.comparison&&!s.choice} title={`${labels[draft.command.operation]} · ${draft.target.name}`} width={620} destroyOnHidden={false} focusTriggerAfterClose={false} onCancel={close} keyboard={!draft.applying} mask={{closable:!draft.applying}} afterClose={()=>{if(live.current?.closing&&live.current.id===draft.id){live.current=null;setDraft(null);}}} afterOpenChange={open=>{if(!open)return;if(live.current?.review)focusPlan();else if(inputFocus.current?.id===live.current?.id&&inputFocus.current?.intent===desktop.inputIntent)document.querySelector<HTMLElement>('.type-operation input:not(:disabled),.type-operation button')?.focus();}} footer={<Space>
      <Button disabled={draft.applying} onClick={close}>Cancel</Button>
      {draft.uncertain?<Button type="primary" onClick={()=>void recheck()}>Recheck actual source set</Button>:draft.review?<>
        <Button disabled={draft.applying} onClick={()=>{version.current++;patch({review:null});}}>Edit input</Button>
        {draft.stale&&<Button onClick={()=>void plan(true)}>Re-plan current source</Button>}
        <Button type="primary" danger={draft.review.destructive} loading={draft.applying} disabled={!!dirty.length||s.status.recoveryRequired||draft.stale||draft.target.clicked!==s.target||s.pending} onClick={()=>void apply()}>Apply</Button>
      </>:<Button type="primary" loading={draft.pending} disabled={draft.applying||s.status.recoveryRequired||s.pending||draft.target.clicked!==s.target} onClick={()=>void plan(draft.stale)}>{draft.stale?"Re-plan current source":"Review change"}</Button>}
    </Space>}>
      <Typography.Paragraph type="secondary" className="type-source">{draft.target.source}</Typography.Paragraph>
      {!draft.review?<Form layout="vertical" className="type-operation" onKeyDown={e=>{if(e.key==="Enter"&&!e.nativeEvent.isComposing&&e.target instanceof HTMLInputElement){e.preventDefault();void plan(draft.stale);}}}>
        {command?.operation==="setValueObjectConversions"&&<Space direction="vertical"><Checkbox disabled={draft.pending} checked={command.fromImplicit} onChange={e=>change({...command,fromImplicit:e.target.checked})}>Implicit from underlying</Checkbox><Checkbox disabled={draft.pending} checked={command.toImplicit} onChange={e=>change({...command,toImplicit:e.target.checked})}>Implicit to underlying</Checkbox></Space>}
        {(command?.operation==="addEnumMember"||command?.operation==="renameEnumMember"||command?.operation==="renameCustomField")&&<Form.Item label="Name"><Input aria-label="Type operation name" readOnly={draft.pending} value={command.operation==="addEnumMember"?command.name:command.newName} onChange={e=>change(command.operation==="addEnumMember"?{...command,name:e.target.value}:{...command,newName:e.target.value})}/></Form.Item>}
        {command?.operation==="addEnumMember"&&<Form.Item label="Explicit numeric value"><Input aria-label="Member numeric value" readOnly={draft.pending} inputMode="numeric" value={command.value} onChange={e=>change({...command,value:e.target.value})}/></Form.Item>}
        {command?.operation==="addCustomField"&&<>
          <Flex gap={12}><Form.Item label="MessagePack key"><Input aria-label="Custom field key" readOnly={draft.pending} value={command.declaration.key} onChange={e=>fieldChange({key:e.target.value})}/></Form.Item><Form.Item label="Name" className="type-field-name"><Input aria-label="Custom field name" readOnly={draft.pending} value={command.declaration.name} onChange={e=>fieldChange({name:e.target.value})}/></Form.Item></Flex>
          <Form.Item label="Base type"><Select aria-label="Custom field base type" disabled={draft.pending} value={command.declaration.typeName} options={draft.choices.map(type=>({value:type,label:type}))} onChange={typeName=>fieldChange({typeName})}/></Form.Item>
          <Form.Item label="Modifier"><Radio.Group disabled={draft.pending} value={command.declaration.array?"array":command.declaration.nullable?"nullable":"required"} options={[{value:"required",label:"Required"},{value:"nullable",label:"Nullable"},{value:"array",label:"Array"}]} onChange={e=>fieldChange({nullable:e.target.value==="nullable",array:e.target.value==="array"})}/></Form.Item>
          {draft.shape?<TypedInitializer key={`${command.declaration.typeName}:${command.declaration.nullable}:${command.declaration.array}`} shape={draft.shape} value={draft.input} disabled={draft.pending} onChange={input=>change(command,input)}/>:draft.shapeError?<Alert type="error" showIcon title={draft.shapeError}/>:<Spin size="small"/>}
        </>}
        {(command?.operation==="dropEnumMember"||command?.operation==="dropCustomField")&&<Typography.Paragraph>削除対象: <Typography.Text strong>{targetLabel(command)}</Typography.Text></Typography.Paragraph>}
      </Form>:<PlanPanel token={draft.review.token} focus={focusPlan}>
        <Space wrap><Tag color={draft.review.destructive?"error":"default"}>{draft.review.destructive?"Destructive change":"Validated candidates"}</Tag><Typography.Text>{draft.review.affectedRecords} value occurrences · {draft.review.files.length} sources</Typography.Text></Space>
        <Typography.Paragraph>{labels[draft.command.operation]}: <Typography.Text strong>{changeLabel(draft.command)}</Typography.Text></Typography.Paragraph>
        {draft.review.files.map(file=><Flex className="type-plan-file" key={file.source} align="center" gap={12}><Typography.Text>{file.source}</Typography.Text><Button onClick={()=>void desktop.migrationCompare(draft.review!.token,file.source,draft.review!.files.map(f=>f.source)).catch(desktop.showError)}>Compare</Button></Flex>)}
        {!!dirty.length&&<Alert type="warning" showIcon title="未保存のaffected sourceがあるためApplyできません" description={dirty.join(", ")}/>}
      </PlanPanel>}
      {draft.error&&<Alert type={draft.uncertain?"warning":"error"} showIcon title={draft.stale?"Re-planが必要です":"操作を完了できません"} description={draft.error}/>}
    </Modal>}
  </section>;
}
function PlanPanel({token,focus,children}:{token:string;focus:(element:HTMLElement|null)=>void;children:ReactNode}) {
  const element=useRef<HTMLDivElement>(null);
  // Ant Modal owns a deferred portal/motion commit. Focusing in its parent's
  // effect can run before this content exists. Use this panel's own commit,
  // with latest-input guards, and the Modal's opening callback as a fallback.
  useLayoutEffect(()=>{const frame=requestAnimationFrame(()=>focus(element.current));return()=>cancelAnimationFrame(frame);},[token]);
  return <div ref={element} className="type-plan" aria-label="Type Migration Plan" tabIndex={-1}>{children}</div>;
}
