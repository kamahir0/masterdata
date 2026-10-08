import {invoke} from "@tauri-apps/api/core";
import {desktop} from "./workspace";
import {delivery} from "./delivery-state";
import type {Reply} from "./types";
const frame=()=>new Promise<void>(resolve=>requestAnimationFrame(()=>resolve()));
async function until(test:()=>boolean,message:string,timeout=30000){const deadline=performance.now()+timeout;while(!test()){if(performance.now()>deadline)throw new Error(message);await frame();}}
function assert(value:unknown,message:string):asserts value {if(!value)throw new Error(message);}
function button(label:string){const value=[...document.querySelectorAll<HTMLButtonElement>('button')].find(b=>b.getClientRects().length&&(b.getAttribute('aria-label')===label||b.textContent===label));assert(value,`missing ${label}`);return value;}
async function settled(){await until(()=>!delivery.view.pending&&delivery.view.snapshot?.running===false,"delivery did not settle",180000);}
async function phase(name:string){await invoke('evidence_write',{report:{kind:"delivery",phase:name}});const deadline=performance.now()+10000;while(performance.now()<deadline){const observed=await invoke<{phase:string;complete:boolean}|null>('evidence_phase');if(observed?.phase===name&&observed.complete)return;await new Promise(resolve=>setTimeout(resolve,100));}throw new Error(`external phase not acknowledged: ${name}`);}
export async function run({startup}:{startup:Record<string,unknown>}) {
 const checks:string[]=[];
 try {
  await until(()=>!!desktop.surface.inventory,"Project unavailable");await desktop.selectTarget("sources/catalog-data.yaml","delivery-setup",false);
  const top=desktop.viewport!.getBoundingClientRect().top;
  desktop.viewport!.scrollLeft=11*160;await frame();await frame();desktop.setSelection(0,11);desktop.beginEditor();await until(()=>!!document.querySelector("input[aria-label=\"name 行 1\"]"),"active scalar editor unavailable");
  const input=document.querySelector<HTMLInputElement>("input[aria-label=\"name 行 1\"]")!;
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value')!.set!.call(input,"Unsaved delivery draft");input.dispatchEvent(new Event('input',{bubbles:true}));input.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',keyCode:13,which:13,bubbles:true}));
  await until(()=>desktop.surface.projection?.rows[0].cells[11].display==="Unsaved delivery draft","dirty edit unavailable");
  button("プロジェクトメニュー").click();await until(()=>[...document.querySelectorAll<HTMLElement>('.ant-dropdown:not(.ant-dropdown-hidden) [role="menuitem"]')].some(el=>el.textContent==="ビルド・配布…"),"delivery menu unavailable");
  [...document.querySelectorAll<HTMLElement>('.ant-dropdown:not(.ant-dropdown-hidden) [role="menuitem"]')].find(el=>el.textContent==="ビルド・配布…")!.click();
  await until(()=>!!delivery.view.snapshot?.context&&!delivery.view.snapshot.running,"delivery context unavailable");assert(delivery.view.profile===null,"initial Profile was filtered");assert(desktop.viewport!.getBoundingClientRect().top===top,"Drawer pushed the working grid");checks.push('contextual-drawer-saved-input');
  button("ビルドして配布…").click();await until(()=>delivery.view.snapshot?.kind==='build'&&delivery.view.snapshot.running,"Build not started");
  assert(button("保存済みソースをビルド").disabled,"repeated Build was enabled");assert(button("最後に成功したビルド成果物を配布").disabled,"concurrent Publish was enabled");
  for(const source of ['sources/item-id.yaml','sources/catalog-data.yaml']){await desktop.selectTarget(source,'delivery-live-navigation',false);const sample=desktop.samples.at(-1);assert(sample?.host?.work.projectYamlParse===0&&sample.host.work.projectValidation===0&&sample.host.work.projectDiscovery===0&&sample.host.work.projectEnumeration===0,"delivery made warm navigation do broad work");}
  assert(desktop.surface.projection!.rows[0].cells[11].display==='Unsaved delivery draft',"Build discarded draft");checks.push('build-live-navigation-draft-and-warm-zero');
  await until(()=>!!delivery.view.snapshot?.preview&&!delivery.view.snapshot.running,"combined Build did not reach explicit preview",180000);
  const built=delivery.view.snapshot!.lastBuild!;assert(built.status==='succeeded'&&built.result?.outcome==='Success',"Build failed");assert(!!built.result.nativeEvidence,"actual MasterMemory reload evidence missing");assert(delivery.view.snapshot!.preview!.detail.targets.length===0,"0-target preview wrong");
  assert(button("配布を実行").getClientRects().length,"Publish skipped Confirm");button("配布をキャンセル").click();await settled();await until(()=>document.activeElement===button("ビルドして配布…"),"Cancel did not return focus to the start action");assert(delivery.view.snapshot!.lastBuild!.id===built.id,"Cancel lost successful Build");checks.push('native-build-then-preview-cancel-retains-artifacts');
  // A named Profile disappears outside the app. Refresh must keep its missing
  // selection, while receipt-only Publish remains eligible.
  const combo=document.querySelector<HTMLInputElement>("input[aria-label=\"ビルドプロファイル\"]")!;combo.focus();combo.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowDown',keyCode:40,which:40,bubbles:true}));
  await until(()=>[...document.querySelectorAll<HTMLElement>('.ant-select-item-option-content')].some(el=>el.textContent==='development'),"named Profile missing");[...document.querySelectorAll<HTMLElement>('.ant-select-item-option-content')].find(el=>el.textContent==='development')!.click();
  assert(delivery.view.profile==='development',"Profile selection not retained");
  await phase('delivery-target');button("ビルド・配布の情報を更新").click();await settled();assert(delivery.view.profile==='development'&&!delivery.view.snapshot!.context!.profiles.includes('development'),"missing Profile fell back");assert(button("保存済みソースをビルド").disabled&&!button("最後に成功したビルド成果物を配布").disabled,"Profile selection contaminated Publish eligibility");checks.push('missing-profile-publish-independent');
  button("最後に成功したビルド成果物を配布").click();await until(()=>!!delivery.view.snapshot?.preview&&!delivery.view.snapshot.running,"fresh Publish preview missing");assert(Number(delivery.view.snapshot!.preview!.detail.targets.length)===2,"current saved targets missing");
  const artifactIdentity=delivery.view.snapshot!.preview!.detail.artifactIdentity;
  await phase('delivery-stale');button("配布を実行").click();await settled();assert(delivery.view.snapshot!.status==='failed'&&delivery.view.snapshot!.error?.code==='E-PUBLISH-STALE',"stale preview was applied");assert(!delivery.view.snapshot!.preview,"stale preview retained authorization");checks.push('stale-preview-fresh-confirm-no-write');
  button("最後に成功したビルド成果物を配布").click();await until(()=>!!delivery.view.snapshot?.preview&&!delivery.view.snapshot.running,"cancel preview unavailable");
  const transport=desktop.rpc;let observed=false,release!:()=>void;const paused=new Promise<void>(resolve=>release=resolve);
  desktop.rpc=async<T>(intent:Record<string,unknown>)=>{
    const reply=await transport.call(desktop,intent) as Reply<T>;
    if(intent.kind==='deliveryStart'&&(intent.request as {operation?:string})?.operation==='cancelPreview'){observed=true;await paused;}
    return reply;
  };
  try {
    button("配布をキャンセル").click();await until(()=>observed,"Cancel reply not observed");
    const newerFocus=document.querySelector<HTMLElement>('#explorer [role="tree"]');assert(newerFocus?.isConnected,"current Explorer unavailable");
    newerFocus.focus();newerFocus.dispatchEvent(new KeyboardEvent('keydown',{key:'Shift',bubbles:true}));
    assert(document.activeElement===newerFocus,"newer focus was not accepted before completion");
    release();await settled();await frame();assert(document.activeElement===newerFocus,"late Cancel stole newer Explorer focus");
    checks.push('late-completion-preserves-newer-focus');
  } finally {release();desktop.rpc=transport;}
  button("最後に成功したビルド成果物を配布").click();await until(()=>!!delivery.view.snapshot?.preview&&!delivery.view.snapshot.running,"new preview unavailable");assert(delivery.view.snapshot!.preview!.detail.artifactIdentity===artifactIdentity,"failed Publish replaced successful canonical artifact");button("配布を実行").click();await settled();await until(()=>document.activeElement===button("最後に成功したビルド成果物を配布"),"Publish completion did not return focus to the start action");assert(String(delivery.view.snapshot!.status)==='succeeded'&&delivery.view.snapshot!.result?.outcome==='Success',"confirmed Publish failed");
  assert(delivery.view.snapshot!.lastBuild!.id===built.id,"Publish replaced Build history");assert(desktop.surface.status.dirty.includes('sources/catalog-data.yaml'),"Publish discarded draft");checks.push('confirmed-receipt-publish-invalid-source');
  assert((startup.browserErrors as string[]).length===0,"browser error");assert(desktop.viewport!.getBoundingClientRect().top===top,"delivery status shifted grid");
  await invoke('evidence_write',{report:{kind:'delivery',checks,state:delivery.view.snapshot,dirty:desktop.surface.status.dirty,visibility:document.visibilityState,focused:document.hasFocus(),startup}});
 }catch(error){await invoke('evidence_write',{report:{kind:'delivery',error:String(error),checks,state:delivery.view.snapshot,status:desktop.surface.status,surfaceError:desktop.surface.error,deliveryError:delivery.view.error,active:document.activeElement?.outerHTML.slice(0,1000),dialog:document.querySelector('[role="dialog"]')?.outerHTML.slice(0,4000),startup}});}
}
