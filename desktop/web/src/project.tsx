import { uiMessage } from "./language";
import {useEffect,useRef,useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import {Alert,Button,Flex,Form,Input,Modal,Space,Typography} from "antd";
import {FolderOpenOutlined,PlusOutlined} from "@ant-design/icons";
import {desktop,unknownProjectReply,type ProjectCreationResult,type Surface} from "./workspace";

const errorText=(e:unknown)=>e&&typeof e==="object"&&"message" in e?String(e.message):String(e);
export function CreateProjectModal({s}:{s:Surface}) {
  const form=s.projectCreation;
  const [path,setPath]=useState(""),[id,setId]=useState(""),[name,setName]=useState(""),[version,setVersion]=useState("0.1.0");
  const [error,setError]=useState<string|null>(null),[result,setResult]=useState<ProjectCreationResult|null>(null);
  const [attemptedPath,setAttemptedPath]=useState<string|null>(null),[picking,setPicking]=useState(false);
  const live=useRef(form);live.current=form;
  const origin=useRef<{element:HTMLElement|null;intent:number;epoch:number}|null>(null);
  useEffect(()=>{
    if(form){setPath("");setId("");setName("");setVersion("0.1.0");setError(null);setResult(null);setAttemptedPath(null);
      origin.current={element:document.activeElement as HTMLElement,intent:desktop.inputIntent,epoch:form.epoch};}
  },[form]);
  const returnFromAction=()=>{if(origin.current)origin.current.intent=desktop.inputIntent;};
  const cancel=()=>{if(!cannotCancel){returnFromAction();desktop.cancelProjectCreation();}};
  async function pick() {
    const expected=form;setPicking(true);
    try {const target=await invoke<string|null>("pick_project");if(target&&live.current===expected)setPath(target);}
    catch(e){if(live.current===expected)setError(errorText(e));}
    finally {setPicking(false);}
  }
  async function create() {
    if(!form||desktop.surface.openingProject||desktop.surface.projectCreationUncertain||attemptedPath===path)return;
    returnFromAction();
    setError(null);setResult(null);setAttemptedPath(path);
    try {const report=await desktop.createProject(path,{id,name,version});if(live.current===form)setResult(report);}
    catch(e){
      if(live.current!==form)return;
      if(!unknownProjectReply(e)) {setAttemptedPath(null);setError(errorText(e));}
      else setResult({outcome:"OutcomeUnknown",root:path,remaining:[],unconfirmed:["作成結果"],message:errorText(e)});
    }
  }
  const running=!!s.openingProject;
  const cannotCancel=running||picking||s.projectCreationUncertain;
  return <Modal title="プロジェクトを作成" open={!!form} width={540}
    closable={!cannotCancel}
    focusable={{focusTriggerAfterClose:false}} keyboard={!cannotCancel} mask={{closable:!cannotCancel}}
    onCancel={cancel}
    afterOpenChange={open=>{
      if(open)return;
      const request=origin.current;if(!request||request.intent!==desktop.inputIntent)return;
      // Ant may focus its portal before this component's effect captures the
      // origin. A closing portal still has geometry; it is not a return target.
      if(desktop.surface.status.epoch===request.epoch&&request.element?.isConnected&&request.element.matches('button,input,select,textarea,a[href],[tabindex]')&&!request.element.closest('.ant-modal-wrap,[role="dialog"],[role="menu"]')&&request.element.getClientRects().length)request.element.focus();
      else document.querySelector<HTMLElement>(desktop.surface.status.epoch!==request.epoch?"button[aria-label=\"テーブルを作成\"]":desktop.surface.inventory?"button[aria-label=\"プロジェクトメニュー\"]":"#welcome button[aria-label=\"プロジェクトを作成\"]")?.focus();
    }}
    footer={<Space><Button disabled={cannotCancel} onClick={cancel}>キャンセル</Button>
      <Button type="primary" icon={<PlusOutlined aria-hidden="true"/>} loading={running} disabled={picking||s.projectCreationUncertain||!path||!id||!name||!version||attemptedPath===path} onClick={()=>void create()}>プロジェクトを作成</Button></Space>}>
    <Form layout="vertical" disabled={running||picking||s.projectCreationUncertain} onKeyDown={e=>{if(e.key==="Enter"&&!e.nativeEvent.isComposing){e.preventDefault();if(path&&id&&name&&version)void create();}}}>
      <Form.Item label="保存先"><Flex gap={6}>
        <Input aria-label="プロジェクトの保存先" value={path} onChange={e=>setPath(e.target.value)} placeholder="空のディレクトリ、または新しいディレクトリ"/>
        <Button aria-label="プロジェクトの保存先を選択" icon={<FolderOpenOutlined aria-hidden="true"/>} onClick={()=>void pick()}/>
      </Flex></Form.Item>
      <Form.Item label="プロジェクトID"><Input aria-label="プロジェクトID" value={id} onChange={e=>setId(e.target.value)} placeholder="game.masterdata"/></Form.Item>
      <Flex gap={12}><Form.Item label="名前" style={{flex:1}}><Input aria-label="プロジェクト名" value={name} onChange={e=>setName(e.target.value)}/></Form.Item>
        <Form.Item label="バージョン" style={{width:120}}><Input aria-label="プロジェクトのバージョン" value={version} onChange={e=>setVersion(e.target.value)}/></Form.Item></Flex>
    </Form>
    {error&&<Alert showIcon type="error" title="プロジェクトを作成できません" description={uiMessage(error)}/>}
    {result&&result.outcome!=="Success"&&<Alert showIcon type={result.outcome==="OutcomeUnknown"?"warning":"error"} title={result.outcome==="OutcomeUnknown"?"作成結果を確認できません":"作成が完了しませんでした"}
      description={<Space orientation="vertical"><Typography.Text>{uiMessage(result.message)}</Typography.Text>
        <Typography.Text code>{result.root}</Typography.Text>
        {!!result.remaining.length&&<Typography.Text>確認できた項目: {result.remaining.join(", ")}</Typography.Text>}
        {!!result.unconfirmed.length&&<Typography.Text>未確認: {result.unconfirmed.join(", ")}</Typography.Text>}
        <Typography.Text type="secondary">保存先を確認してください。別の空ディレクトリを指定して再作成できます。</Typography.Text>
        <Button disabled={running} icon={<FolderOpenOutlined aria-hidden="true"/>} onClick={()=>{returnFromAction();void desktop.resolveProjectCreation(result.root).catch(e=>setError(errorText(e)));}}>保存先を開く…</Button>
        {s.projectCreationUncertain&&s.inventory&&<Button disabled={running} onClick={()=>{returnFromAction();void desktop.resolveProjectCreation(s.inventory!.root).catch(e=>setError(errorText(e)));}}>前のプロジェクトを開く…</Button>}
      </Space>}/>}
  </Modal>;
}
