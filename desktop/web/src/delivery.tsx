import {useEffect,useRef,useState,useSyncExternalStore} from "react";
import {Alert,Button,Collapse,Descriptions,Divider,Drawer,Empty,Flex,List,Select,Space,Spin,Tag,Tooltip,Typography} from "antd";
import {BuildOutlined,CloudUploadOutlined,FileSearchOutlined,ReloadOutlined,SaveOutlined} from "@ant-design/icons";
import {delivery,type BuildResult,type DeliveryError,type PublishResult,type Snapshot} from "./delivery-state";
import {desktop,type Surface} from "./workspace";
import type {Diagnostic} from "./types";

export function DeliveryDrawer({s}:{s:Surface}) {
  const v=useSyncExternalStore(delivery.subscribe,delivery.snapshot);
  const [start,setStart]=useState(0),[page,setPage]=useState<DeliveryError|null>(null);
  const previewOrigin=useRef<HTMLElement|null>(null);
  useEffect(()=>{delivery.project(s.status.epoch);},[s.status.epoch]);
  useEffect(()=>{setStart(0);setPage(null);},[v.snapshot?.lastBuild?.id]);
  const state=v.snapshot,context=state?.context,preview=state?.preview,attempt=state?.lastBuild;
  const busy=v.pending||!!state?.running;
  const missing=v.profile!==null&&!!context&&!context.profiles.includes(v.profile);
  const buildReason=s.status.recoveryRequired?"sourceのRecoveryを完了してからBuildしてください。":
    context?.buildCapability.available===false?context.buildCapability.reason:missing?"選択したProfileがありません。別のProfileを明示的に選択してください。":null;
  const publishReason=context?.receipt.eligible===false?context.receipt.reason:null;
  const dirty=s.status.dirty.length;
  async function diagnostics(next:number){const result=await delivery.problems(next).catch(desktop.showError);if(result){setStart(next);setPage(result);}}
  async function focus(problem:Diagnostic,index:number){
    const id=attempt?.id;if(!id)return;
    delivery.close();await desktop.focusBuildProblem(id,index,problem.source);
  }
  function beginPreview(origin:HTMLElement){previewOrigin.current=origin;void delivery.preview();}
  function returnPreviewFocus(){if(previewOrigin.current?.isConnected)previewOrigin.current.focus();}
  const result=state?.kind==="publish"?state.result as PublishResult|null:null;
  const displayedError=page??attempt?.error;
  return <Drawer title={<Space><BuildOutlined/>Build / Publish</Space>} open={v.open} onClose={delivery.close}
    size={540} mask={false} focusable={{trap:false}} className="delivery-drawer" destroyOnHidden
    afterOpenChange={open=>{if(open)document.querySelector<HTMLButtonElement>('button[aria-label="Build saved sources"]')?.focus();}}
    extra={<Tooltip title="保存済みconfigとartifactを確認"><Button type="text" icon={<ReloadOutlined/>} aria-label="Refresh delivery context" disabled={busy} onClick={()=>void delivery.refresh()}/></Tooltip>}>
    <Flex vertical gap={14}>
      <Descriptions column={1} size="small" items={[
        {key:"project",label:"Project",children:context?.project.name??s.inventory?.project.name},
        {key:"dirty",label:"未保存source",children:<Space><Tag>{dirty}</Tag><Typography.Text type="secondary">Buildには含めません</Typography.Text><Button icon={<SaveOutlined/>} disabled={s.busy||s.deliveryCapturing||s.status.recoveryRequired||!!s.status.uncertain.length} onClick={()=>void desktop.saveAll()}>Save All</Button></Space>},
        {key:"config",label:"Config",children:"保存済みmasterdata.toml"},
      ]}/>
      <div><Typography.Text className="delivery-label">Build Profile</Typography.Text><Select aria-label="Build Profile" style={{width:"100%"}} value={v.profile??""} status={missing?"error":undefined} disabled={busy}
        options={[{value:"",label:"Unfiltered — 全record"},...(context?.profiles??[]).map(value=>({value,label:value})),...(missing?[{value:v.profile!,label:`${v.profile} — missing`}]:[])]}
        onChange={value=>delivery.profile(value||null)}/></div>
      {buildReason&&<Alert type="warning" showIcon title="Buildを開始できません" description={buildReason}/>}
      <Space wrap><Button type="primary" icon={<BuildOutlined/>} aria-label="Build saved sources" disabled={busy||!!buildReason||s.busy||!context} onClick={()=>void delivery.build()}>Build</Button>
        <Button icon={<CloudUploadOutlined/>} aria-label="Publish last successful artifacts" disabled={busy||!!publishReason||!context} onClick={e=>beginPreview(e.currentTarget)}>Publish last artifacts</Button>
        <Tooltip title="保存済みinputでBuildし、成功artifactのPublish previewへ進みます"><Button disabled={busy||!!buildReason||s.busy||!context} onClick={e=>{previewOrigin.current=e.currentTarget;void delivery.build(true);}}>Build and Publish…</Button></Tooltip></Space>
      {publishReason&&<Typography.Text type="secondary">Publish: {publishReason}</Typography.Text>}
      {state?.kind?.startsWith("publish")&&<Typography.Text type="secondary">source freshness: この操作では未確認 · Unity verification: not_observed</Typography.Text>}
      {busy&&<div role="status" aria-live="polite" className="delivery-progress"><Spin size="small"/><div><Typography.Text>{state?.kind==="build"?state.capturing?"保存済みinputを取得中":"Build実行中":state?.kind==="publish"?"Publish実行中":"確認中"}</Typography.Text>
        <div className="delivery-caption">{state?.captured?.project.name??s.inventory?.project.name} · {state?.kind==="build"?(state.profile??"Unfiltered"):""}</div><Typography.Text type="secondary">編集とsourceの移動は続けられます。別のBuild / PublishとProject切替は完了後に再操作してください。</Typography.Text></div></div>}
      {v.error&&<Alert type="warning" showIcon title="結果の確認が必要です" description={<Space direction="vertical"><span>{v.error}</span><Button onClick={()=>void delivery.recheck()}>Recheck operation</Button></Space>}/>}
      {state?.error&&state.kind!=="build"&&<Alert type="error" showIcon title={`${state.error.code}: ${state.error.message}`} description={(state.kind==="publishPreview"||state.kind==="publish")?"全targetはNotAttemptedです。取得できなかった項目は未確認です。新しいpreviewから再操作してください。":undefined}/>}
      {preview&&<section aria-label="Publish preview" className="delivery-preview">
        <Flex justify="space-between" align="center"><Typography.Title level={5}>Publish preview</Typography.Title><Tag>NotAttempted</Tag></Flex>
        <Typography.Paragraph type="secondary">source freshness: この操作では未確認。現在のYAMLやProfileをPublishの対象には含めません。</Typography.Paragraph>
        {v.combinedBuild&&attempt?.captured?.configIdentity!==preview.detail.configIdentity&&<Alert type="warning" showIcon title="Build後にconfigが変わりました" description="現在のconfigによる新しい対象と影響を確認してください。"/>}
        <ArtifactIdentity root={preview.detail.artifactRoot} identity={preview.detail.artifactIdentity}/>
        {preview.detail.targets.length?<List size="small" dataSource={preview.detail.targets} renderItem={target=><List.Item><div className="delivery-target"><Typography.Text strong>{target.kind}</Typography.Text><Typography.Paragraph className="delivery-path" copyable>{target.destination}</Typography.Paragraph>
          <Space wrap><Tag>追加 {target.additions.length}</Tag><Tag>更新 {target.updates.length}</Tag><Tag>削除 {target.removals.length}</Tag>{target.binaryReplacement&&<Tag>binaryを更新</Tag>}</Space>
          <Collapse ghost size="small" items={[{key:"files",label:"変更するfile",children:<Space direction="vertical">{[["追加",target.additions],["更新",target.updates],["削除",target.removals]].map(([label,paths])=><div key={String(label)}><Typography.Text type="secondary">{String(label)}</Typography.Text>{(paths as string[]).map(path=><div key={path}>{path}</div>)}</div>)}</Space>}]}/></div></List.Item>}/>:<Alert type="info" showIcon title="Publish targetは0件です" description="Confirmはsuccessful no-opです。fileの配置は行いません。"/>}
        <Space className="delivery-confirm"><Button disabled={busy} onClick={()=>{void delivery.cancel().then(returnPreviewFocus);}}>Cancel Publish</Button><Button type="primary" icon={<CloudUploadOutlined/>} disabled={busy} onClick={()=>{void delivery.confirm().then(returnPreviewFocus);}}>Confirm Publish</Button></Space>
      </section>}
      {result&&<section aria-label="Publish result"><Divider/><Typography.Title level={5}>Publish · {result.outcome}</Typography.Title>
        {result.noOp&&<Typography.Paragraph>target 0件 · successful no-op（配置なし）</Typography.Paragraph>}
        <Typography.Paragraph type="secondary">source freshness: この操作では未確認<br/>Unity verification: not_observed</Typography.Paragraph>
        <List size="small" dataSource={result.targets} renderItem={target=><List.Item><div className="delivery-target"><Space><Tag color={target.outcome==="Success"?"success":"warning"}>{target.status}</Tag><Typography.Text>{target.kind}</Typography.Text></Space><div className="delivery-path">{target.destination}</div><Typography.Text type="secondary">{target.message}</Typography.Text>{target.recoveryDirectory&&<Typography.Paragraph copyable className="delivery-path">{target.recoveryDirectory}</Typography.Paragraph>}</div></List.Item>}/></section>}
      {state?.kind==="cancelPreview"&&!busy&&<Typography.Text role="status" type="secondary">Publishは実行していません。成功済みBuild artifactを保持しています。</Typography.Text>}
      {attempt&&<section aria-label="Last Build result"><Divider/><Flex align="center" justify="space-between"><Typography.Title level={5}>Build · {attempt.status}</Typography.Title><Tag>{attempt.profile??"Unfiltered"}</Tag></Flex>
        <Typography.Paragraph type="secondary">保存済みinput · {attempt.captured?.project.name??s.inventory?.project.name}{v.combinedBuild&&" · Publishは別の確認操作です"}</Typography.Paragraph>
        {attempt.result&&<BuildOutcome result={attempt.result}/>}
        {attempt.error&&<Alert type="error" showIcon title={`${attempt.error.code}: ${attempt.error.message}`}/>}
        {displayedError?.diagnosticTotal?<><Typography.Paragraph>Build Problems · 保存済みsnapshot · {attempt.profile??"Unfiltered"} · {displayedError.diagnosticTotal}件</Typography.Paragraph>
          <List size="small" dataSource={displayedError.diagnostics} renderItem={(problem,index)=><List.Item><Button type="text" className="delivery-problem" onClick={()=>void focus(problem,start+index)}><WarningIcon kind={problem.kind}/><span><strong>{problem.code}</strong> {problem.message}<small>{problem.source}{problem.occurrence?` · #${problem.occurrence}`:""}{problem.fieldPath.length?` · ${problem.fieldPath.join(".")}`:""}</small></span></Button></List.Item>}/>
          {displayedError.diagnosticTotal>200&&<Space><Button disabled={start===0} onClick={()=>void diagnostics(Math.max(0,start-200))}>Previous</Button><Typography.Text>{start+1}–{Math.min(start+200,displayedError.diagnosticTotal)}</Typography.Text><Button disabled={start+200>=displayedError.diagnosticTotal} onClick={()=>void diagnostics(start+200)}>Next</Button></Space>}</>:null}
        {attempt.captured&&<Collapse ghost size="small" items={[{key:"snapshot",label:"取得したinput",children:<><Typography.Paragraph className="delivery-path">config {attempt.captured.configIdentity}</Typography.Paragraph>{Object.entries(attempt.captured.inputIdentity).map(([path,identity])=><Typography.Paragraph className="delivery-path" key={path}>{path}<small>{identity}</small></Typography.Paragraph>)}</>}]}/>}
      </section>}
      {state?.kind==="recheckArtifacts"&&!busy&&!state.error&&<Alert type="info" showIcon title="actual artifact setのreceiptと全hashを確認しました" description="source freshnessは未確認、Unity verificationはnot_observedです。"/>}
      {!busy&&(!state||state.status==="idle")&&!context&&<Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="Build / Publish contextを確認中"/>}
      <Button type="text" icon={<FileSearchOutlined/>} disabled={busy} onClick={()=>void delivery.artifacts()}>Recheck actual artifacts</Button>
    </Flex>
  </Drawer>;
}
function ArtifactIdentity({root,identity,label="artifact"}:{root:string;identity:string;label?:string}){return <div className="delivery-artifact"><Typography.Paragraph copyable className="delivery-path">{root}</Typography.Paragraph><Typography.Text type="secondary" className="delivery-path">{label} {identity}</Typography.Text></div>;}
function BuildOutcome({result}:{result:BuildResult}){return <><Typography.Paragraph>{result.outcome}: {result.message}</Typography.Paragraph><ArtifactIdentity root={result.artifactRoot} identity={result.configIdentity} label="config"/><Space wrap>{Object.entries(result.rowCounts).map(([table,count])=><Tag key={table}>{table} · {count} records</Tag>)}</Space><Typography.Paragraph type="secondary">MasterMemory binaryのreloadと全選択recordを確認しました。</Typography.Paragraph><Collapse ghost size="small" items={[{key:"native",label:"Native Build details",children:<Typography.Paragraph className="delivery-path" style={{whiteSpace:"pre-wrap"}}>{result.nativeEvidence}</Typography.Paragraph>}]}/>{result.outcome==="Success"&&!result.dryRun&&<Typography.Text type="secondary">生成C#・binary・receiptを一式保存しました。Unity verification: not_observed</Typography.Text>}{result.retainedBackup&&<Alert type="warning" title="artifactを再確認してください" description={result.retainedBackup}/>}</>;}
function WarningIcon({kind}:{kind:string}){return <Tag color={kind==="error"?"error":"warning"}>{kind}</Tag>;}
