import {useCallback,useEffect,useLayoutEffect,useRef,useState} from "react";
import {App,Alert,Button,Drawer,Empty,Flex,Input,Modal,Select,Space,Spin,Tabs,Tag,Tooltip,Typography,type InputRef} from "antd";
import {DeleteOutlined,DiffOutlined,EditOutlined,LeftOutlined,PlusOutlined,ReloadOutlined,RightOutlined,SaveOutlined,SettingOutlined,WarningOutlined} from "@ant-design/icons";
import {desktop,type Surface} from "./workspace";

export interface ConfigEntry {index:number;text:string;valid:boolean;reason:string|null}
export interface ConfigList {entries:ConfigEntry[];total:number;start:number;editable:boolean;reason:string|null}
export interface ConfigView {
  revision:number;dirty:boolean;outcome:string|null;editable:boolean;reason:string|null;valid:boolean;
  profiles:{name:string;editable:boolean;reason:string|null;location:{line:number;column:number}}[];profileCount:number;profileStart:number;
  targets:{index:number;kind:string|null;path:string|null;editable:boolean;reason:string|null;location:{line:number;column:number}}[];targetCount:number;targetStart:number;
  detail:{name:string;include:ConfigList;exclude:ConfigList}|null;
  problems:{message:string;location:{line:number;column:number}}[];canAddProfile:boolean;canAddTarget:boolean;
}
type Typing = {revision:number;initial:string;text:string;composing:boolean} & (
  {kind:"tag";profile:string;exclude:boolean;index:number|null} |
  {kind:"path";index:number} | {kind:"profile"} | {kind:"target";targetKind:"csharp"|"binary"|null}
);
type Write = {source:string;outcome:string;message:string};
type Comparison = {before:string;after:string;conflict:boolean};
const describe=(error:unknown)=>typeof error==="object"&&error&&"message" in error?String(error.message):String(error);
const operation=(t:Typing)=>t.kind==="tag"?{operation:"tags",profile:t.profile,exclude:t.exclude,edit:{operation:t.index===null?"add":"replace",index:t.index,text:t.text}}:
  t.kind==="path"?{operation:"targetPath",index:t.index,text:t.text}:t.kind==="profile"?{operation:"addProfile",name:t.text}:{operation:"addTarget",kind:t.targetKind,path:t.text};

// Only this bounded config projection and temporary text live in React. TOML
// spans, tag interpretation, occurrence identity and Save authority stay in Rust.
export function ProjectSettings({s}:{s:Surface}) {
  const {message}=App.useApp();
  const [view,setView]=useState<ConfigView|null>(null),[section,setSection]=useState("profiles"),[selected,setSelected]=useState<string|null>(null),
    [loading,setLoading]=useState(false),[busy,setBusy]=useState(false),[error,setError]=useState<string|null>(null),[typing,setTyping]=useState<Typing|null>(null),
    [newItem,setNewItem]=useState<"profile"|"target"|null>(null),[comparison,setComparison]=useState<Comparison|null>(null),[writeUncertain,setWriteUncertain]=useState(false);
  const epoch=useRef(s.status.epoch),value=useRef(view),input=useRef<InputRef>(null),formInput=useRef<InputRef>(null),openingIntent=useRef(0),
    active=useRef(typing),profile=useRef(selected),starts=useRef<[number,number,number,number]>([0,0,0,0]),
    sequence=useRef(0),committing=useRef<Promise<boolean>|null>(null),running=useRef(false),origin=useRef<HTMLElement|null>(null),
    focusReturn=useRef<{epoch:number;intent:number;label:string}|null>(null),modalVisible=useRef(false),uncertain=useRef(false);
  const markUncertain=useCallback((next:boolean)=>{uncertain.current=next;setWriteUncertain(next);},[]);
  value.current=view;
  const restoreFocus=useCallback(()=>{
    const pending=focusReturn.current;
    if(!pending||running.current||active.current||modalVisible.current)return;
    requestAnimationFrame(()=>{
      if(focusReturn.current!==pending||running.current||active.current||modalVisible.current)return;
      focusReturn.current=null;
      if(pending.epoch!==desktop.surface.status.epoch||pending.intent!==desktop.inputIntent||!desktop.surface.settingsOpen)return;
      document.querySelector<HTMLButtonElement>(`.settings-drawer button[aria-label="${CSS.escape(pending.label)}"]`)?.focus();
    });
  },[]);
  const update=useCallback((t:Typing|null)=>{active.current=t;setTyping(t);desktop.settingsTyping(!!t&&(t.text!==t.initial||(t.kind==="target"&&t.targetKind!==null)));},[]);
  const read=useCallback(async()=>{
    const captured=epoch.current,mine=++sequence.current;setLoading(true);
    try {
      let reply=await desktop.rpc<ConfigView>({kind:"configView",epoch:captured,profile:profile.current,starts:starts.current});
      if(captured!==desktop.surface.status.epoch||mine!==sequence.current)return false;
      if(profile.current===null&&reply.data.profiles.length){profile.current=reply.data.profiles[0].name;setSelected(profile.current);
        reply=await desktop.rpc<ConfigView>({kind:"configView",epoch:captured,profile:profile.current,starts:starts.current});}
      if(captured!==desktop.surface.status.epoch||mine!==sequence.current)return false;
      value.current=reply.data;setView(reply.data);return true;
    } catch(e){if(captured===desktop.surface.status.epoch&&mine===sequence.current)setError(describe(e));return false;}
    finally {if(captured===desktop.surface.status.epoch&&mine===sequence.current)setLoading(false);}
  },[]);
  const apply=useCallback(async(revision:number,operation:Record<string,unknown>)=>{
    if(running.current||uncertain.current)return false;
    const captured=epoch.current;running.current=true;setBusy(true);setError(null);
    try {
      await desktop.rpc({kind:"configEdit",epoch:captured,revision,operation});
      if(captured!==desktop.surface.status.epoch)return false;
      await read();return true;
    } catch(e){if(captured===desktop.surface.status.epoch){setError(describe(e));await read();}return false;}
    finally {if(captured===desktop.surface.status.epoch){running.current=false;setBusy(false);}}
  },[read]);
  const commit=useCallback((explicit=false):Promise<boolean>=>{
    if(uncertain.current){setError("Outcome Unknown: actual configをRecheckしてから続けてください。");return Promise.resolve(false);}
    if(committing.current)return committing.current;
    const t=active.current;
    if(!t||(!explicit&&t.text===t.initial&&(t.kind!=="target"||t.targetKind===null)))return Promise.resolve(true);
    if(t.composing){setError("入力の変換を確定してから続けてください。");return Promise.resolve(false);}
    if(t.kind==="target"&&!t.targetKind){setError("追加するtargetのkindを選択してください。");return Promise.resolve(false);}
    const captured=epoch.current,intent=desktop.inputIntent;
    const pending:Promise<boolean>=apply(t.revision,operation(t)).then(ok=>{
      if(ok&&active.current===t){
        const label=t.kind==="tag"?(t.index===null?`Add ${t.exclude?"Exclude":"Include"} tags`:`Edit ${t.exclude?"Exclude":"Include"} tags ${t.index+1}`):
          t.kind==="path"?`Publish target ${t.index+1} path`:t.kind==="profile"?"New Build Profile":"New Publish target";
        focusReturn.current={epoch:captured,intent,label};
        update(null);if(t.kind==="profile"){profile.current=t.text;setSelected(t.text);starts.current[2]=0;starts.current[3]=0;void read();}setNewItem(null);
      }
      return ok&&captured===desktop.surface.status.epoch;
    }).finally(()=>{if(committing.current===pending)committing.current=null;});committing.current=pending;return pending;
  },[apply,read,update]);
  const save=useCallback(async()=>{
    const captured=epoch.current;
    if(running.current&&!committing.current)return;
    if(!(await commit())||running.current||captured!==desktop.surface.status.epoch)return;
    const v=value.current;if(!v)return;
    running.current=true;setBusy(true);setError(null);
    let observed=false;
    try {
      const reply=await desktop.rpc<Write>({kind:"configSave",epoch:captured,revision:v.revision});
      if(captured!==desktop.surface.status.epoch)return;
      if(reply?.data?.source!=="masterdata.toml"||!['Success','Conflict','Failure','OutcomeUnknown','NotAttempted','RecoveryRequired'].includes(reply.data.outcome))throw new Error("Config Saveの応答を確認できません。");
      observed=true;
      if(reply.data.outcome!=="Success")setError(`${reply.data.outcome}: ${reply.data.message}`);
      else void message.success("Project Settingsを保存しました",1.5);
      await read();await desktop.refreshInventory();
    } catch(e){if(captured===desktop.surface.status.epoch){
      // A native domain rejection is definitive. Loss of the commit reply is
      // not: retain protection even if an earlier status event looked clean.
      const definitive=typeof e==="object"&&e!==null&&"code" in e&&"message" in e;
      if(!observed&&!definitive){markUncertain(true);setError(`Outcome Unknown: ${describe(e)}`);}else setError(describe(e));
    }}
    finally {if(captured===desktop.surface.status.epoch){running.current=false;setBusy(false);}}
  },[commit,read,message,markUncertain]);
  const protectedInput=()=>uncertain.current||!!active.current&&(active.current.text!==active.current.initial||(active.current.kind==="target"&&active.current.targetKind!==null));
  const hooks=useRef({commit:()=>commit(),dirty:protectedInput,save});
  hooks.current={commit:()=>commit(),dirty:protectedInput,save};
  useLayoutEffect(()=>desktop.bindSettings({commit:()=>hooks.current.commit(),dirty:()=>hooks.current.dirty(),save:()=>hooks.current.save()}),[]);
  useEffect(()=>{
    epoch.current=s.status.epoch;sequence.current++;running.current=false;committing.current=null;
    profile.current=null;setSelected(null);starts.current=[0,0,0,0];update(null);markUncertain(false);value.current=null;setView(null);setComparison(null);setNewItem(null);focusReturn.current=null;setError(null);setBusy(false);setLoading(false);
  },[s.status.epoch,update,markUncertain]);
  useEffect(()=>{
    if(!s.settingsOpen||!s.inventory)return;
    origin.current=document.querySelector<HTMLButtonElement>('button[aria-label="Project menu"]');openingIntent.current=desktop.inputIntent;
    void read();
  },[s.settingsOpen,s.status.epoch,read]);
  useEffect(()=>{if(s.settingsOpen&&s.inventory&&!running.current)void read();},[s.status.configIdentity,s.status.externalVersion,read]);
  useLayoutEffect(()=>{if(typing&&s.settingsOpen){const captured=epoch.current,t=typing;requestAnimationFrame(()=>{if(epoch.current===captured&&active.current===t&&desktop.surface.settingsOpen)(t.kind==="profile"||t.kind==="target"?formInput:input).current?.focus({cursor:"end"});});}},[typing?.kind,typing&&"index" in typing?typing.index:null,typing&&typing.kind==="tag"?typing.exclude:null,newItem,s.settingsOpen]);
  // A committed input unmounts, and the trigger may still be disabled while
  // the modal closes. Restore only after both finish, without stealing a newer
  // navigation/focus intent (including a different Project session).
  useLayoutEffect(()=>restoreFocus(),[typing,newItem,busy,s.settingsOpen,restoreFocus]);
  const blocked=busy||s.busy||writeUncertain||s.status.recoveryRequired||!view?.editable;
  const begin=async(t:Typing)=>{const captured=epoch.current;if(!(await commit())||captured!==desktop.surface.status.epoch)return;const v=value.current;if(!v?.editable)return;update({...t,revision:v.revision});};
  async function chooseProfile(name:string){const captured=epoch.current;if(!(await commit())||captured!==desktop.surface.status.epoch)return;profile.current=name;setSelected(name);starts.current[2]=0;starts.current[3]=0;await read();}
  async function chooseSection(next:string){const captured=epoch.current;if(!(await commit())||captured!==desktop.surface.status.epoch)return;setSection(next);}
  async function page(at:number,index:number){const captured=epoch.current;if(!(await commit())||captured!==desktop.surface.status.epoch)return;starts.current[index]=at;await read();}
  async function add(kind:"profile"|"target"){
    const captured=epoch.current;if(!(await commit())||captured!==desktop.surface.status.epoch)return;const v=value.current;if(!v)return;
    modalVisible.current=true;setNewItem(kind);update(kind==="profile"?{kind,revision:v.revision,initial:"",text:"",composing:false}:{kind,revision:v.revision,initial:"",text:"",composing:false,targetKind:null});
  }
  async function compare(external=false){
    const captured=epoch.current;if((!uncertain.current&&!(await commit()))||captured!==desktop.surface.status.epoch)return;
    try {const reply=await desktop.rpc<Comparison>({kind:"configCompare",epoch:captured,external});if(captured===desktop.surface.status.epoch)setComparison(reply.data);}
    catch(e){if(captured===desktop.surface.status.epoch)setError(describe(e));}
  }
  async function reload(){
    if(running.current)return;
    const v=value.current;if(!v)return;
    const captured=epoch.current;
    if(v.dirty||active.current||uncertain.current||v.outcome==="OutcomeUnknown"){
      const result=await desktop.choose("Project SettingsをReload", "masterdata.tomlの未保存設定と入力を破棄し、diskから読み直します。YAMLのdraftは保持します。",["Reload","Cancel"]);
      if(result!=="Reload"||captured!==desktop.surface.status.epoch)return;
    }
    running.current=true;setBusy(true);
    try {await desktop.rpc({kind:"configReload",epoch:captured,revision:v.revision,discardAuthorized:true});
      if(captured!==desktop.surface.status.epoch)return;update(null);markUncertain(false);setNewItem(null);setComparison(null);setError(null);await read();await desktop.refreshInventory();}
    catch(e){if(captured===desktop.surface.status.epoch)setError(describe(e));}
    finally {if(captured===desktop.surface.status.epoch){running.current=false;setBusy(false);}}
  }
  async function recheck(){
    const captured=epoch.current;running.current=true;setBusy(true);
    try {const reply=await desktop.rpc<Write>({kind:"configRecheck",epoch:captured});if(captured!==desktop.surface.status.epoch)return;markUncertain(false);
      setError(reply.data.outcome==="Success"?null:`${reply.data.outcome}: ${reply.data.message}`);await read();await desktop.refreshInventory();}
    catch(e){if(captured===desktop.surface.status.epoch)setError(describe(e));}
    finally{if(captured===desktop.surface.status.epoch){running.current=false;setBusy(false);}}
  }
  const textInput=(t:Typing,label:string)=><Input ref={t.kind==="profile"||t.kind==="target"?formInput:input} aria-label={label} value={t.text} disabled={blocked} status={error?"error":undefined}
    onChange={e=>update({...t,text:e.target.value})} onCompositionStart={()=>update({...t,composing:true})}
    onCompositionEnd={e=>update({...t,text:e.currentTarget.value,composing:false})}
    onKeyDown={e=>{if(e.nativeEvent.isComposing||active.current?.composing)return;
      if(e.key==="Enter"){e.preventDefault();void commit(true);}else if(e.key==="Escape"&&!newItem){e.preventDefault();e.stopPropagation();update(null);}}}/>;
  const detail=view?.detail;
  function tags(exclude:boolean,list:ConfigList){
    const label=exclude?"Exclude tags":"Include tags",current=typing?.kind==="tag"&&typing.exclude===exclude&&typing.profile===detail?.name?typing:null;
    return <section aria-label={label} className="settings-tag-list">
      <Flex justify="space-between" align="center"><Typography.Text strong>{label}</Typography.Text>
        <Tooltip title="Tagを追加"><Button type="text" icon={<PlusOutlined/>} aria-label={`Add ${label}`} disabled={blocked||!list.editable}
          onClick={()=>void begin({kind:"tag",profile:detail!.name,exclude,index:null,revision:view!.revision,initial:"",text:"",composing:false})}/></Tooltip></Flex>
      {list.reason&&<Alert type="warning" showIcon title={list.reason}/>}
      <div role="list" aria-label={`${label} entries`}>
        {list.entries.map(entry=><div role="listitem" key={entry.index} className="settings-entry">
          {current?.index===entry.index?textInput(current,`Edit ${label} ${entry.index+1}`):<Button type="text" className="settings-value" disabled={blocked||!list.editable}
            aria-label={`Edit ${label} ${entry.index+1}`} onClick={()=>void begin({kind:"tag",profile:detail!.name,exclude,index:entry.index,revision:view!.revision,initial:entry.text,text:entry.text,composing:false})}>{entry.text||<Typography.Text type="secondary">Empty tag</Typography.Text>}</Button>}
          <Tooltip title={entry.reason}><span className="settings-entry-state">{!entry.valid&&<WarningOutlined aria-label={entry.reason??"Invalid tag"}/>}</span></Tooltip>
          <Tooltip title="削除"><Button type="text" icon={<DeleteOutlined/>} aria-label={`Remove ${label} ${entry.index+1}`} disabled={blocked||!list.editable}
            onClick={()=>void (async()=>{const captured=epoch.current;if(await commit()&&captured===desktop.surface.status.epoch){const v=value.current;if(v)await apply(v.revision,{operation:"tags",profile:detail!.name,exclude,edit:{operation:"remove",index:entry.index}});}})()}/></Tooltip>
        </div>)}
        {current?.index===null&&<div className="settings-entry">{textInput(current,`New ${label}`)}<Button icon={<PlusOutlined/>} aria-label={`Commit ${label}`} disabled={blocked} onClick={()=>void commit(true)}>Add</Button></div>}
        {!list.total&&!current&&<Typography.Text type="secondary">指定なし</Typography.Text>}
      </div>
      <Pages start={list.start} total={list.total} disabled={busy} page={next=>void page(next,exclude?3:2)}/>
    </section>;
  }
  const selectedProfile=view?.profiles.find(p=>p.name===selected),missing=selected!==null&&!detail;
  return <>
    <Drawer title={<Space><SettingOutlined/>Project Settings</Space>} open={s.settingsOpen} size={510} mask={false} focusable={{trap:false}} className="settings-drawer"
      afterOpenChange={open=>{if(open&&openingIntent.current===desktop.inputIntent){if(active.current)input.current?.focus();else document.querySelector<HTMLElement>('.settings-sections [role="tab"][aria-selected="true"]')?.focus();}}}
      onClose={()=>{if(busy)return;desktop.projectSettings(false);if(origin.current?.isConnected)origin.current.focus();}}
      extra={<Space size={4}><span className="settings-pending">{loading&&<Spin size="small"/>}</span>
        <Tooltip title="未保存設定を比較"><Button type="text" icon={<DiffOutlined/>} aria-label="Compare Project Settings" disabled={busy||(!view?.dirty&&!s.settingsInputDirty)} onClick={()=>void compare()}/></Tooltip>
        <Button type="primary" icon={<SaveOutlined/>} aria-label="Save Project Settings" disabled={blocked||s.deliveryCapturing||s.status.configUncertain} onClick={()=>void save()}>Save</Button></Space>}>
      <Flex vertical gap={12}>
        <Flex justify="space-between" align="center"><Typography.Text type="secondary" className="settings-file">masterdata.toml</Typography.Text><span className="settings-state">{(view?.dirty||s.settingsInputDirty)&&<Tag color="processing">Unsaved</Tag>}</span></Flex>
        <SettingsSections section={section} disabled={busy} select={next=>void chooseSection(next)}/>
        {view?.reason&&<Alert type="warning" showIcon title="設定を編集できません" description={view.reason}/>}
        {s.status.recoveryRequired&&<Alert type="warning" showIcon title="Recovery Required" description="source-setのRecoveryを完了してから設定を保存してください。"/>}
        {view?.outcome==="Conflict"&&<Alert type="error" showIcon title="masterdata.tomlが外部で変更されました" description={<Space><Button onClick={()=>void compare(true)}>Compare</Button><Button onClick={()=>void reload()} disabled={busy}>Reload…</Button></Space>}/>}
        {(writeUncertain||view?.outcome==="OutcomeUnknown")&&<Alert type="warning" showIcon title="保存結果を確認できません" description={<Button onClick={()=>void recheck()} disabled={busy}>Recheck actual config</Button>}/>}
        {error&&<Alert type="error" showIcon closable onClose={()=>setError(null)} title="設定変更を完了できません" description={error}/>}
        {section==="profiles"?<>
          <Flex gap={8}><Select aria-label="Settings Profile" style={{flex:1}} placeholder="Profileを選択" value={selected} disabled={busy}
            options={[...(view?.profiles??[]).map(p=>({value:p.name,label:p.name})),...(missing&&selected&&!selectedProfile?[{value:selected,label:`${selected} — unavailable`}]:[])]} onChange={name=>void chooseProfile(name)}/>
            <Button icon={<PlusOutlined/>} aria-label="New Build Profile" disabled={blocked||!view?.canAddProfile} onClick={()=>void add("profile")}>Profile</Button></Flex>
          <Pages start={view?.profileStart??0} total={view?.profileCount??0} disabled={busy} page={next=>void page(next,0)}/>
          {selectedProfile?.reason&&<Alert type="warning" showIcon title={selectedProfile.reason} description={`masterdata.toml:${selectedProfile.location.line}:${selectedProfile.location.column}`}/>}
          {detail?<>{tags(false,detail.include)}{tags(true,detail.exclude)}</>:!loading&&<Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={selected?"このProfileは編集できません":"Profileはまだありません"}/>}
        </>:<>
          <Flex justify="space-between" align="center"><Typography.Text strong>Publish targets</Typography.Text><Button icon={<PlusOutlined/>} aria-label="New Publish target" disabled={blocked||!view?.canAddTarget} onClick={()=>void add("target")}>Target</Button></Flex>
          <div role="list" aria-label="Publish target occurrences">{view?.targets.map(target=><section role="listitem" key={target.index} className="settings-target">
            <Flex justify="space-between" align="center"><Space><Typography.Text type="secondary">{target.index+1}</Typography.Text><Tag>{target.kind??"Unknown kind"}</Tag></Space><Typography.Text type="secondary">Line {target.location.line}</Typography.Text></Flex>
            {typing?.kind==="path"&&typing.index===target.index?textInput(typing,`Publish target ${target.index+1} path`):<Button type="text" className="settings-value settings-path" icon={<EditOutlined/>} aria-label={`Publish target ${target.index+1} path`} disabled={blocked||!target.editable}
              onClick={()=>void begin({kind:"path",index:target.index,revision:view.revision,initial:target.path??"",text:target.path??"",composing:false})}>{target.path||<Typography.Text type="secondary">Empty path</Typography.Text>}</Button>}
            {target.reason&&<Typography.Text type="warning">{target.reason}</Typography.Text>}
          </section>)}</div>
          {!loading&&!view?.targetCount&&<Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="Publish targetはまだありません"/>}
          <Pages start={view?.targetStart??0} total={view?.targetCount??0} disabled={busy} page={next=>void page(next,1)}/>
          <Typography.Text type="secondary">pathの保存ではPublishを実行しません。</Typography.Text>
        </>}
        {view?.problems.length? <Alert type="warning" showIcon title="設定の問題" description={<div className="settings-problems">{view.problems.map((p,i)=><Typography.Paragraph key={i}>{p.message}<Typography.Text type="secondary"> · {p.location.line}:{p.location.column}</Typography.Text></Typography.Paragraph>)}</div>}/>:null}
        {s.status.environmentError&&view?.outcome!=="Conflict"&&<Alert type="warning" showIcon title="Project serviceを利用できません" description={<Space direction="vertical"><Typography.Text>{s.status.environmentError}</Typography.Text><Button icon={<ReloadOutlined/>} disabled={busy} onClick={()=>void desktop.reloadProject()}>Reload Project…</Button></Space>}/>}
        <Button type="text" icon={<ReloadOutlined/>} aria-label="Discard Project Settings and reload" disabled={busy} onClick={()=>void reload()}>Discard / Reload…</Button>
      </Flex>
    </Drawer>
    <Modal title={newItem==="profile"?"New Build Profile":"New Publish target"} open={newItem!==null} okText="Add" confirmLoading={busy} destroyOnHidden
      focusable={{focusTriggerAfterClose:false}} afterOpenChange={open=>{modalVisible.current=open;if(!open)restoreFocus();}}
      okButtonProps={{disabled:blocked}} cancelButtonProps={{disabled:busy}} closable={!busy} maskClosable={false}
      onCancel={()=>{if(!busy){focusReturn.current={epoch:epoch.current,intent:desktop.inputIntent,label:newItem==="profile"?"New Build Profile":"New Publish target"};setNewItem(null);update(null);}}} onOk={()=>void commit(true)}>
      {typing?.kind==="profile"&&<Flex vertical gap={8}><Typography.Text>Profile name</Typography.Text>{textInput(typing,"New Profile name")}</Flex>}
      {typing?.kind==="target"&&<Flex vertical gap={12}>
        <div><Typography.Text>Kind</Typography.Text><Select aria-label="New target kind" style={{width:"100%"}} value={typing.targetKind} placeholder="kindを選択" disabled={busy}
          options={[{value:"csharp",label:"C# directory"},{value:"binary",label:"MasterMemory binary"}]} onChange={targetKind=>update({...typing,targetKind})}/></div>
        <div><Typography.Text>Path</Typography.Text>{textInput(typing,"New target path")}</div>
      </Flex>}
      {error&&<Alert type="error" showIcon title={error}/>}
    </Modal>
    <Modal title={comparison?.conflict?"Config Conflict · Compare":"Save candidate · masterdata.toml"} open={comparison!==null} width={960} onCancel={()=>setComparison(null)}
      footer={<Space>{comparison?.conflict&&<Button onClick={()=>void reload()} disabled={busy}>Reload…</Button>}<Button onClick={()=>setComparison(null)}>Close</Button></Space>}>
      <div className="comparison-columns"><section><Typography.Text strong>{comparison?.conflict?"Actual disk":"Saved"}</Typography.Text><Input.TextArea aria-label="Config comparison before" value={comparison?.before} readOnly rows={18}/></section>
        <section><Typography.Text strong>Draft</Typography.Text><Input.TextArea aria-label="Config comparison after" value={comparison?.after} readOnly rows={18}/></section></div>
    </Modal>
  </>;
}
function SettingsSections({section,disabled,select}:{section:string;disabled:boolean;select:(s:string)=>void}){
  return <Tabs className="settings-sections" size="small" activeKey={section} onChange={select} items={[{key:"profiles",label:"Profiles",disabled},{key:"targets",label:"Publish Targets",disabled}]}/>;
}
function Pages({start,total,disabled,page}:{start:number;total:number;disabled:boolean;page:(s:number)=>void}){
  if(total<=64)return null;
  return <Flex justify="space-between" align="center"><Typography.Text type="secondary">{start+1}–{Math.min(start+64,total)} / {total}</Typography.Text><Space size={2}>
    <Button type="text" icon={<LeftOutlined/>} aria-label="Previous settings page" disabled={disabled||start===0} onClick={()=>page(Math.max(0,start-64))}/>
    <Button type="text" icon={<RightOutlined/>} aria-label="Next settings page" disabled={disabled||start+64>=total} onClick={()=>page(start+64)}/>
  </Space></Flex>;
}
