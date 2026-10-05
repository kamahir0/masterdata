import {useEffect, useRef, useState} from "react";
import {Alert, Button, Flex, Form, Input, Modal, Select, Space, Typography} from "antd";
import {desktop} from "./workspace";

type Choices = {root:string;folders:string[];filename:string};
type Review = {token:string;source:string;destination:string};
type Result = {source:string;outcome:string;message:string};
type Draft = {id:number;source:string;folder:string;filename:string;choices:Choices;review:Review|null;pending:boolean;committing:boolean;result:Result|null;error:string|null};
const parent = (source:string) => source.includes("/")?source.slice(0,source.lastIndexOf("/")):".";
const join = (folder:string,filename:string) => folder && folder!=="."?`${folder}/${filename}`:filename;
const errorText = (error:unknown) => error&&typeof error==="object"&&"message" in error?`${"code" in error?String(error.code)+": ":""}${String(error.message)}`:String(error);

// The form owns only an explicit destination and a current opaque review token.
// Rust checks roots, aliases, actual identity, exclusive commit and observation.
export function useSourcePath(epoch:number,onMoved:(source:string,destination:string,inputIntent:number)=>void) {
  const [draft,setDraft]=useState<Draft|null>(null);
  const live=useRef(draft);live.current=draft;
  const version=useRef(0),ids=useRef(0),running=useRef(false),queued=useRef<{draft:Draft;version:number}|null>(null);
  const filename=useRef<HTMLInputElement|null>(null),origin=useRef<HTMLElement|null>(null);
  const protection=`source-path:${epoch}`;
  const patch=(change:Partial<Draft>)=>{if(live.current){live.current={...live.current,...change};setDraft(live.current);}};
  useEffect(()=>{version.current++;queued.current=null;live.current=null;setDraft(null);},[epoch]);
  async function begin(source:string) {
    const id=++ids.current;
    origin.current=document.activeElement as HTMLElement;
    try {
      if(!(await desktop.guardSource(source,epoch))||id!==ids.current)return;
      const reply=await desktop.rpc<Choices>({kind:"pathMoveChoices",epoch,source});
      if(epoch!==desktop.surface.status.epoch||id!==ids.current)return;
      const next:Draft={id,source,folder:parent(source),filename:reply.data.filename,choices:reply.data,review:null,pending:true,committing:false,result:null,error:null};
      live.current=next;setDraft(next);review(next);
    } catch(error) {if(epoch===desktop.surface.status.epoch)desktop.showError(error);}
  }
  function review(next=live.current) {
    if(!next)return;
    const mine=++version.current;
    patch({review:null,pending:true,result:null,error:null});
    queued.current={draft:next,version:mine};void drain();
  }
  async function drain() {
    if(running.current)return;
    running.current=true;
    try {
      while(queued.current) {
        const request=queued.current;queued.current=null;
        try {
          const reply=await desktop.rpc<Review>({kind:"pathMovePreview",epoch,source:request.draft.source,destination:join(request.draft.folder,request.draft.filename)});
          if(request.version===version.current&&epoch===desktop.surface.status.epoch&&live.current?.id===request.draft.id)patch({review:reply.data,pending:false});
        } catch(error) {
          if(request.version===version.current&&epoch===desktop.surface.status.epoch&&live.current?.id===request.draft.id)patch({error:errorText(error),pending:false});
        }
      }
    } finally {running.current=false;}
  }
  function change(change:Partial<Draft>) {if(live.current){patch(change);review(live.current);}}
  function cancel() {
    if(live.current?.committing||live.current?.result?.outcome==="OutcomeUnknown")return;
    version.current++;ids.current++;queued.current=null;live.current=null;setDraft(null);
    if(origin.current?.isConnected)origin.current.focus();
  }
  async function apply() {
    const current=live.current;
    if(!current||current.pending||current.committing||!current.review)return;
    const token=current.review.token,inputIntent=desktop.inputIntent;
    const recheck=current.result?.outcome==="OutcomeUnknown";
    version.current++;queued.current=null;desktop.protectWriteView(protection,true);patch({committing:true,error:null});
    try {
      const reply=await desktop.rpc<Result>({kind:recheck?"recheckMove":"moveSource",epoch,token});
      if(epoch!==desktop.surface.status.epoch||live.current?.id!==current.id)return;
      desktop.protectWriteView(protection,reply.data.outcome==="OutcomeUnknown");
      patch({committing:false,result:reply.data,review:reply.data.outcome==="OutcomeUnknown"?current.review:null});
      if(reply.data.outcome==="Success") {
        live.current=null;setDraft(null);
        await desktop.sourceMoved(current.source,current.review.destination,epoch);
        await desktop.refreshInventory();
        if(epoch===desktop.surface.status.epoch)onMoved(current.source,current.review.destination,inputIntent);
      } else await desktop.refreshInventory();
    } catch(error) {
      if(epoch===desktop.surface.status.epoch&&live.current?.id===current.id) {
        const knownRejection=error&&typeof error==="object"&&"code" in error;
        desktop.protectWriteView(protection,recheck||!knownRejection);
        patch({committing:false,review:recheck||!knownRejection?current.review:null,error:errorText(error),result:!knownRejection?{source:current.source,outcome:"OutcomeUnknown",message:"結果を受信できませんでした。old / new pathをRecheckしてください。"}:current.result});
      }
    }
  }
  const uncertain=draft?.result?.outcome==="OutcomeUnknown";
  const modal=draft&&<Modal title="Rename / Move source" open width={520} focusTriggerAfterClose={false} afterOpenChange={open=>{if(open)filename.current?.focus();}} onCancel={cancel} keyboard={!draft.committing&&!uncertain} mask={{closable:!draft.committing&&!uncertain}} footer={<Space>
    <Button onClick={cancel} disabled={draft.committing||uncertain}>Cancel</Button>
    {!uncertain&&!draft.review&&!draft.pending&&<Button onClick={()=>review()}>Review current source</Button>}
    <Button type="primary" loading={draft.committing||draft.pending} disabled={draft.pending||!draft.review} onClick={()=>void apply()}>{uncertain?"Recheck both paths":"Move"}</Button>
  </Space>}>
    <Typography.Paragraph type="secondary" className="source-path-origin">{draft.source}</Typography.Paragraph>
    <Form layout="vertical" disabled={draft.committing||uncertain}>
      <Form.Item label="Folder"><Select aria-label="Move source folder" value={draft.folder} options={draft.choices.folders.map(folder=>({value:folder||".",label:folder||"."}))} onChange={folder=>change({folder})}/></Form.Item>
      <Form.Item label="Filename"><Input aria-label="Move source filename" ref={input=>{filename.current=input?.input??null;}} value={draft.filename} onChange={event=>change({filename:event.target.value})} onKeyDown={event=>{if(event.nativeEvent.isComposing)return;if(event.key==="Enter"){event.preventDefault();void apply();}}}/></Form.Item>
    </Form>
    <Flex vertical gap={8}>
      <Typography.Text type="secondary">{draft.choices.root} · {join(draft.folder,draft.filename)}</Typography.Text>
      {draft.error&&<Alert type="error" showIcon title={draft.error}/>}
      {draft.result&&<Alert type={uncertain?"warning":"error"} showIcon title={uncertain?"Outcome Unknown":draft.result.outcome} description={draft.result.message}/>}
    </Flex>
  </Modal>;
  return {begin,modal,open:!!draft};
}
