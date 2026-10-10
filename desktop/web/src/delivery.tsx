import { uiMessage, statusLabel, uiTerms } from "./language";
import {useEffect,useLayoutEffect,useRef,useState,useSyncExternalStore} from "react";
import {Alert,Button,Collapse,Descriptions,Divider,Drawer,Empty,Flex,List,Select,Space,Spin,Tag,Tooltip,Typography} from "antd";
import {BuildOutlined,CloudUploadOutlined,FileSearchOutlined,ReloadOutlined,SaveOutlined} from "@ant-design/icons";
import {delivery,type BuildResult,type DeliveryError,type PublishResult,type Snapshot} from "./delivery-state";
import {desktop,type Surface} from "./workspace";
import type {Diagnostic} from "./types";

export function DeliveryDrawer({s}:{s:Surface}) {
  const v=useSyncExternalStore(delivery.subscribe,delivery.snapshot);
  const [start,setStart]=useState(0),[page,setPage]=useState<DeliveryError|null>(null);
  const previewOrigin=useRef<HTMLElement|null>(null);
  const previewFocus=useRef<{origin:HTMLElement|null;epoch:number;intent:number;kind:"publish"|"cancelPreview"}|null>(null);
  useEffect(()=>{delivery.project(s.status.epoch);},[s.status.epoch]);
  useEffect(()=>{setStart(0);setPage(null);},[v.snapshot?.lastBuild?.id]);
  const state=v.snapshot,context=state?.context,preview=state?.preview,attempt=state?.lastBuild;
  const busy=v.pending||!!state?.running;
  useLayoutEffect(()=>{
    const restore=previewFocus.current;
    if(!restore)return;
    if(!v.open||restore.epoch!==s.status.epoch||restore.intent!==desktop.inputIntent){previewFocus.current=null;return;}
    if(busy||state?.kind!==restore.kind)return;
    // The native request can return while the job is still running. Restore
    // only after React enables the start action; newer user input owns focus.
    previewFocus.current=null;
    if(restore.origin?.isConnected)restore.origin.focus();
  },[busy,state?.kind,state?.id,state?.status,v.open,s.status.epoch]);
  useEffect(()=>{delivery.invalidateConfig(s.status.configIdentity);},[s.status.configIdentity,busy]);
  const missing=v.profile!==null&&!!context&&!context.profiles.includes(v.profile);
  const buildReason=s.status.recoveryRequired?"ソースの復旧を完了してからBuildしてください。":
    context?.buildCapability.available===false?context.buildCapability.reason:missing?"選択したプロファイルがありません。別のプロファイルを明示的に選択してください。":null;
  const publishUnavailable=context?.receipt.eligible===false;
  const publishReason=publishUnavailable?context?.receipt.reason:null;
  const dirty=s.status.dirty.length;
  async function diagnostics(next:number){const result=await delivery.problems(next).catch(desktop.showError);if(result){setStart(next);setPage(result);}}
  async function focus(problem:Diagnostic,index:number){
    const id=attempt?.id;if(!id)return;
    delivery.close();await desktop.focusBuildProblem(id,index,problem.source);
  }
  function beginPreview(origin:HTMLElement){previewOrigin.current=origin;void delivery.preview();}
  function finishPreview(kind:"publish"|"cancelPreview"){
    previewFocus.current={origin:previewOrigin.current,epoch:s.status.epoch,intent:desktop.inputIntent,kind};
    void (kind==="publish"?delivery.confirm():delivery.cancel());
  }
  const result=state?.kind==="publish"?state.result as PublishResult|null:null;
  const displayedError=page??attempt?.error;
  return <Drawer title={<Space><BuildOutlined aria-hidden="true"/>{uiTerms.buildPublish}</Space>} open={v.open} onClose={delivery.close}
    size={540} mask={false} focusable={{trap:false}} className="delivery-drawer" destroyOnHidden
    afterOpenChange={open=>{if(open)document.querySelector<HTMLButtonElement>("button[aria-label=\"Build（保存済みソース）\"]")?.focus();}}
    extra={<Tooltip title="保存済み設定と成果物を確認"><Button type="text" icon={<ReloadOutlined aria-hidden="true"/>} aria-label={`${uiTerms.buildPublish}の情報を更新`} disabled={busy} onClick={()=>void delivery.refresh()}/></Tooltip>}>
    <Flex vertical gap={14}>
      <Descriptions column={1} size="small" items={[
        {key:"project",label:"プロジェクト",children:context?.project.name??s.inventory?.project.name},
        {key:"dirty",label:"未保存ソース",children:<Space><Tag>{dirty}</Tag><Typography.Text type="secondary">Buildには含めません</Typography.Text><Button icon={<SaveOutlined aria-hidden="true"/>} disabled={s.busy||s.deliveryCapturing||s.status.recoveryRequired||!!s.status.uncertain.length} onClick={()=>void desktop.saveAll()}>すべて保存</Button></Space>},
        {key:"config",label:"設定",children:<Typography.Text>保存済みmasterdata.toml{(s.status.configDirty||s.settingsInputDirty)&&" · 未保存設定は使用しません"}</Typography.Text>},
      ]}/>
      <div className="delivery-profile" aria-busy={busy}><Typography.Text className="delivery-label">{uiTerms.buildProfile}</Typography.Text><Select aria-label={uiTerms.buildProfile} style={{width:"100%"}} value={v.profile??""} status={missing?"error":undefined} disabled={busy}
        options={[{value:"",label:"すべてのレコード"},...(context?.profiles??[]).map(value=>({value,label:value})),...(missing?[{value:v.profile!,label:`${v.profile} — 見つかりません`}]:[])]}
        onChange={value=>delivery.profile(value||null)}/></div>
      {buildReason&&<Alert type="warning" showIcon title="Buildを開始できません" description={uiMessage(buildReason)}/>}
      <Space wrap><Button type="primary" icon={<BuildOutlined aria-hidden="true"/>} aria-label="Build（保存済みソース）" disabled={busy||!!buildReason||s.busy||!context} onClick={()=>void delivery.build()}>{uiTerms.build}</Button>
        <Button icon={<CloudUploadOutlined aria-hidden="true"/>} aria-label="Publish…（最後に成功した成果物）" disabled={busy||publishUnavailable||!context} onClick={e=>beginPreview(e.currentTarget)}>{uiTerms.publish}…</Button>
        <Tooltip title="保存済み入力でBuildし、成功した成果物のPublish内容を確認します"><Button disabled={busy||!!buildReason||s.busy||!context} onClick={e=>{previewOrigin.current=e.currentTarget;void delivery.build(true);}}>Build → Publish…</Button></Tooltip></Space>
      {publishUnavailable&&<div><Typography.Text type="secondary">Publishには確認済みのBuild成果物が必要です。</Typography.Text>
        {publishReason&&<Collapse ghost size="small" items={[{key:"receipt",label:"利用できない理由",children:<Typography.Paragraph className="delivery-path">{uiMessage(publishReason)}</Typography.Paragraph>}]}/>}</div>}
      {state?.kind?.startsWith("publish")&&<Typography.Text type="secondary">ソースの最新状態: 未確認 · Unity実機検証: 未実施</Typography.Text>}
      {busy&&<div role="status" aria-live="polite" className="delivery-progress"><Spin size="small"/><div><Typography.Text>{state?.kind==="build"?state.capturing?"保存済み入力を取得中":"Build実行中":state?.kind==="publish"?"Publish実行中":"確認中"}</Typography.Text>
        <div className="delivery-caption">{state?.captured?.project.name??s.inventory?.project.name} · {state?.kind==="build"?(state.profile??"すべてのレコード"):""}</div><Typography.Text type="secondary">編集とソースの移動は続けられます。別のBuild / Publishとプロジェクト切替は完了後に再操作してください。</Typography.Text></div></div>}
      {v.error&&<Alert type="warning" showIcon title="結果の確認が必要です" description={<Space direction="vertical"><span>{uiMessage(v.error)}</span><Button onClick={()=>void delivery.recheck()}>操作結果を再確認</Button></Space>}/>}
      {state?.error&&state.kind!=="build"&&<Alert type="error" showIcon title={uiMessage(`${state.error.code}: ${state.error.message}`)} description={(state.kind==="publishPreview"||state.kind==="publish")?"すべての配布先は未実行です。取得できなかった項目は未確認です。Publish内容を確認し直してから再操作してください。":undefined}/>}
      {preview&&<section aria-label="Publish内容の確認" className="delivery-preview">
        <Flex justify="space-between" align="center"><Typography.Title level={5}>Publish内容の確認</Typography.Title><Tag>未実行</Tag></Flex>
        <Typography.Paragraph type="secondary">ソースの最新状態は確認していません。現在のYAMLやプロファイルはPublishに使用しません。</Typography.Paragraph>
        {v.combinedBuild&&attempt?.captured?.configIdentity!==preview.detail.configIdentity&&<Alert type="warning" showIcon title="Build後に設定が変わりました" description="現在の設定による新しい対象と影響を確認してください。"/>}
        <ArtifactIdentity root={preview.detail.artifactRoot} identity={preview.detail.artifactIdentity}/>
        {preview.detail.targets.length?<List size="small" dataSource={preview.detail.targets} renderItem={target=><List.Item><div className="delivery-target"><Typography.Text strong>{statusLabel(target.kind)}</Typography.Text><Typography.Paragraph className="delivery-path" copyable>{target.destination}</Typography.Paragraph>
          <Space wrap><Tag>追加 {target.additions.length}</Tag><Tag>更新 {target.updates.length}</Tag><Tag>削除 {target.removals.length}</Tag>{target.binaryReplacement&&<Tag>バイナリを更新</Tag>}</Space>
          <Collapse ghost size="small" items={[{key:"files",label:"変更するファイル",children:<Space direction="vertical">{[["追加",target.additions],["更新",target.updates],["削除",target.removals]].map(([label,paths])=><div key={String(label)}><Typography.Text type="secondary">{String(label)}</Typography.Text>{(paths as string[]).map(path=><div key={path}>{path}</div>)}</div>)}</Space>}]}/></div></List.Item>}/>:<Alert type="info" showIcon title="配布先は0件です" description="実行してもファイルは配置されません。"/>}
        <Space className="delivery-confirm"><Button disabled={busy} onClick={()=>finishPreview("cancelPreview")} aria-label="Publishをキャンセル">キャンセル</Button><Button type="primary" icon={<CloudUploadOutlined aria-hidden="true"/>} disabled={busy} onClick={()=>finishPreview("publish")} aria-label="Publishを実行">{uiTerms.publish}</Button></Space>
      </section>}
      {result&&<section aria-label="Publish結果"><Divider/><Typography.Title level={5}>{uiTerms.publish} · {statusLabel(result.outcome)}</Typography.Title>
        {result.noOp&&<Typography.Paragraph>配布先0件 · 完了（配置なし）</Typography.Paragraph>}
        <Typography.Paragraph type="secondary">ソースの最新状態: 未確認<br/>Unity実機検証: 未実施</Typography.Paragraph>
        {!!result.targets.length&&<List size="small" dataSource={result.targets} renderItem={target=><List.Item><div className="delivery-target"><Space><Tag color={target.outcome==="Success"?"success":"warning"}>{statusLabel(target.status)}</Tag><Typography.Text>{statusLabel(target.kind)}</Typography.Text></Space><div className="delivery-path">{target.destination}</div><Typography.Text type="secondary">{uiMessage(target.message)}</Typography.Text>{target.recoveryDirectory&&<Typography.Paragraph copyable className="delivery-path">{target.recoveryDirectory}</Typography.Paragraph>}</div></List.Item>}/>}</section>}
      {state?.kind==="cancelPreview"&&!busy&&<Typography.Text role="status" type="secondary">Publishは実行していません。成功済みBuild成果物を保持しています。</Typography.Text>}
      {attempt&&<section aria-label="最後のBuild結果"><Divider/><Flex align="center" justify="space-between"><Typography.Title level={5}>Build · {statusLabel(attempt.status)}</Typography.Title><Tag>{attempt.profile??"すべてのレコード"}</Tag></Flex>
        <Typography.Paragraph type="secondary">保存済み入力 · {attempt.captured?.project.name??s.inventory?.project.name}{v.combinedBuild&&" · Publishは別の確認操作です"}</Typography.Paragraph>
        {attempt.result&&<BuildOutcome result={attempt.result}/>}
        {attempt.error&&<Alert type="error" showIcon title={uiMessage(`${attempt.error.code}: ${attempt.error.message}`)}/>}
        {displayedError?.diagnosticTotal?<><Typography.Paragraph>Buildの問題 · 保存済みスナップショット · {attempt.profile??"すべてのレコード"} · {displayedError.diagnosticTotal}件</Typography.Paragraph>
          <List size="small" dataSource={displayedError.diagnostics} renderItem={(problem,index)=><List.Item><Button type="text" className="delivery-problem" onClick={()=>void focus(problem,start+index)}><WarningIcon kind={problem.kind}/><span><strong>{problem.code}</strong> {uiMessage(problem.message)}<small>{problem.source}{problem.occurrence?` · #${problem.occurrence}`:""}{problem.fieldPath.length?` · ${problem.fieldPath.join(".")}`:""}</small></span></Button></List.Item>}/>
          {displayedError.diagnosticTotal>200&&<Space><Button disabled={start===0} onClick={()=>void diagnostics(Math.max(0,start-200))}>前へ</Button><Typography.Text>{start+1}–{Math.min(start+200,displayedError.diagnosticTotal)}</Typography.Text><Button disabled={start+200>=displayedError.diagnosticTotal} onClick={()=>void diagnostics(start+200)}>次へ</Button></Space>}</>:null}
        {attempt.captured&&<Collapse ghost size="small" items={[{key:"snapshot",label:"取得した入力",children:<><Typography.Paragraph className="delivery-path">設定 {attempt.captured.configIdentity}</Typography.Paragraph>{Object.entries(attempt.captured.inputIdentity).map(([path,identity])=><Typography.Paragraph className="delivery-path" key={path}>{path}<small>{identity}</small></Typography.Paragraph>)}</>}]}/>}
      </section>}
      {state?.kind==="recheckArtifacts"&&!busy&&!state.error&&<Alert type="info" showIcon title="成果物を確認しました" description="成果物一式のレシートとすべてのハッシュを確認しました。ソースの最新状態は未確認、Unity実機検証は未実施です。"/>}
      {!busy&&(!state||state.status==="idle")&&!context&&<Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="Build / Publishの情報を確認中"/>}
      <Tooltip title="成果物一式のレシートとハッシュを確認します。ソースの最新状態やUnityの動作は検証しません。"><Button type="text" icon={<FileSearchOutlined aria-hidden="true"/>} disabled={busy} onClick={()=>void delivery.artifacts()} aria-label="成果物を再確認（レシートとハッシュ）">成果物を再確認</Button></Tooltip>
    </Flex>
  </Drawer>;
}
function ArtifactIdentity({root,identity,label="成果物"}:{root:string;identity:string;label?:string}){return <div className="delivery-artifact"><Typography.Paragraph copyable className="delivery-path">{root}</Typography.Paragraph><Typography.Text type="secondary" className="delivery-path">{label} {identity}</Typography.Text></div>;}
function BuildOutcome({result}:{result:BuildResult}){return <><Typography.Paragraph>{statusLabel(result.outcome)}: {uiMessage(result.message)}</Typography.Paragraph><ArtifactIdentity root={result.artifactRoot} identity={result.configIdentity} label="設定"/><Space wrap>{Object.entries(result.rowCounts).map(([table,count])=><Tag key={table}>{table} · {count} レコード</Tag>)}</Space><Typography.Paragraph type="secondary">MasterMemoryバイナリを読み直し、選択したすべてのレコードを確認しました。</Typography.Paragraph><Collapse ghost size="small" items={[{key:"native",label:"ネイティブBuildの詳細",children:<Typography.Paragraph className="delivery-path" style={{whiteSpace:"pre-wrap"}}>{result.nativeEvidence}</Typography.Paragraph>}]}/>{result.outcome==="Success"&&!result.dryRun&&<Typography.Text type="secondary">生成したC#・バイナリ・レシートを一式保存しました。Unity実機検証: 未実施</Typography.Text>}{result.retainedBackup&&<Alert type="warning" title="成果物を再確認してください" description={result.retainedBackup}/>}</>;}
function WarningIcon({kind}:{kind:string}){return <Tag color={kind==="error"?"error":"warning"}>{statusLabel(kind)}</Tag>;}
