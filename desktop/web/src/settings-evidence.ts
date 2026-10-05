import {invoke} from "@tauri-apps/api/core";
import {getCurrentWindow} from "@tauri-apps/api/window";
import {desktop} from "./workspace";
const SOURCE="sources/catalog-data.yaml";
const frame=()=>new Promise<void>(resolve=>requestAnimationFrame(()=>resolve()));
function assert(test:unknown,message:string):asserts test {if(!test)throw new Error(message);}
async function until(test:()=>boolean,message:string){const end=performance.now()+8000;while(!test()){if(performance.now()>=end)throw new Error(message);await frame();}}
function find(selector:string){const node=document.querySelector<HTMLElement>(selector);assert(node,`missing ${selector}`);return node;}
function key(node:HTMLElement,key:string,extra:KeyboardEventInit={}){node.dispatchEvent(new KeyboardEvent("keydown",{key,bubbles:true,...extra}));}
function text(input:HTMLInputElement,value:string){Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value")!.set!.call(input,value);input.dispatchEvent(new Event("input",{bubbles:true}));}
function visible(node:Element){return node.getBoundingClientRect().width>0&&node.getBoundingClientRect().height>0;}
function shown(selector:string){return [...document.querySelectorAll<HTMLElement>(selector)].find(visible);}
function button(label:string,scope:ParentNode=document){const node=[...scope.querySelectorAll<HTMLButtonElement>('button')].find(b=>visible(b)&&b.textContent?.trim()===label);assert(node,`missing button ${label}`);return node;}
async function choose(label:string){await until(()=>!!desktop.surface.choice,`missing ${label} confirmation`);const title=desktop.surface.choice!.title;
  let modal:HTMLElement|undefined;await until(()=>{modal=[...document.querySelectorAll<HTMLElement>('.ant-modal')].find(m=>visible(m)&&m.querySelector('.ant-modal-title')?.textContent===title);return !!modal&&[...modal.querySelectorAll<HTMLButtonElement>('button')].some(b=>visible(b)&&b.textContent?.trim()===label);},`${label} modal not rendered`);
  button(label,modal).click();await until(()=>!desktop.surface.choice,"confirmation did not close");}
async function settings(){
  find('button[aria-label="Project menu"]').click();
  await until(()=>[...document.querySelectorAll<HTMLElement>('.ant-dropdown-menu-item')].some(n=>n.textContent?.startsWith("Project Settings…")),"Project Settings command missing");
  [...document.querySelectorAll<HTMLElement>('.ant-dropdown-menu-item')].find(n=>n.textContent?.startsWith("Project Settings…"))!.click();
  await until(()=>desktop.surface.settingsOpen&&!!document.querySelector('.settings-drawer [role="tab"]'),"settings did not open");
  await ready();
}
async function ready(){await until(()=>!!document.querySelector('.settings-drawer')&&!document.querySelector('.settings-pending .ant-spin')&&!document.querySelector('.settings-drawer button[aria-label="Save Project Settings"]:disabled'),"settings did not settle");}
const config=async()=> (await desktop.rpc<{before:string;after:string}>({kind:"configCompare",external:false})).data;
const yaml=async()=> (await desktop.rpc<{before:string;after:string}>({kind:"compare",source:SOURCE})).data;
export async function run({startup}:{startup:Record<string,unknown>}){
  const checks:string[]=[];
  try {
    await until(()=>document.hasFocus(),"native window was not focused");
    await desktop.selectTarget(SOURCE,"settings-setup",false);
    const base=await config(),originalYaml=await yaml(),root=desktop.surface.inventory!.root;
    desktop.setSelection(0,desktop.surface.projection!.columns.findIndex(c=>c.field.name==="name"));desktop.beginEditor();
    await until(()=>!!document.querySelector('.active-cell-editor input'),"YAML scalar editor missing");
    let input=find('.active-cell-editor input') as HTMLInputElement;text(input,"Settings keep this draft");key(input,"Enter");
    await until(()=>!!desktop.surface.projection?.dirty&&!desktop.surface.busy,"YAML draft missing");
    const draft=(await yaml()).after;
    await settings();assert((await config()).after===base.after&&!desktop.surface.status.configDirty,"opening Settings dirtied config");
    assert(desktop.surface.status.dirty.includes(SOURCE),"Settings lost YAML ownership");checks.push("config-open-clean-yaml-independent");
    find('button[aria-label="New Build Profile"]').click();await until(()=>!!document.querySelector('input[aria-label="New Profile name"]'),"Profile input missing");
    input=find('input[aria-label="New Profile name"]') as HTMLInputElement;input.dispatchEvent(new CompositionEvent("compositionstart",{bubbles:true}));text(input,"日本語");key(input,"Enter",{isComposing:true});await frame();
    assert((await config()).after===base.after,"composition created Profile");input.dispatchEvent(new CompositionEvent("compositionend",{bubbles:true}));
    text(input,"Debug");key(input,"Enter");await until(()=>!!document.querySelector('.ant-modal .ant-alert-error'),"invalid Profile name not reported");
    assert(input.value==="Debug"&&(await config()).after===base.after,"invalid new Profile identity was repaired or lost");
    text(input,"production");key(input,"Enter");await until(()=>!shown('.ant-modal input[aria-label="New Profile name"]'),"Profile form did not commit");await ready();
    await until(()=>document.activeElement?.getAttribute('aria-label')==="New Build Profile","Profile creation lost keyboard focus");
    assert((await config()).after===base.after+"\n[build.profiles.production]\n","Profile addition rewrote unrelated bytes");checks.push("profile-compose-explicit-identity");
    find('button[aria-label="Add Exclude tags"]').click();await until(()=>!!document.querySelector('input[aria-label="New Exclude tags"]'),"Tag input missing");
    input=find('input[aria-label="New Exclude tags"]') as HTMLInputElement;text(input," Debug ");key(input,"s",{ctrlKey:true});
    await until(()=>!desktop.surface.status.configDirty&&!desktop.surface.settingsInputDirty&&!!desktop.surface.status.environmentError,"active settings Save did not retain invalid domain input");
    await ready();
    await until(()=>document.activeElement?.getAttribute('aria-label')==="Add Exclude tags","active Save lost keyboard focus");
    assert((await config()).before.includes('exclude_tags = [" Debug "]'),"active text was lost or trimmed on Save");assert((await yaml()).after===draft,"config Save changed YAML draft");
    checks.push("settings-active-save-domain-invalid");
    [...document.querySelectorAll<HTMLElement>('.settings-drawer [role="tab"]')].find(n=>n.textContent==="Publish Targets")!.click();await until(()=>!!document.querySelector('button[aria-label="New Publish target"]'),"Target section missing");
    find('button[aria-label="New Publish target"]').click();await until(()=>!!document.querySelector('input[aria-label="New target path"]'),"Target form missing");
    input=find('input[aria-label="New target path"]') as HTMLInputElement;text(input,"delivery/masterdata.bytes");key(input,"Enter");
    await until(()=>!![...document.querySelectorAll<HTMLElement>('.ant-modal .ant-alert')].find(n=>n.textContent?.includes("kindを選択")),"target kind was silently inferred");
    const kind=find('.ant-modal input[role="combobox"]');kind.focus();key(kind,"ArrowDown",{keyCode:40,which:40});
    await until(()=>!![...document.querySelectorAll<HTMLElement>('.ant-select-item-option')].find(n=>n.textContent==="MasterMemory binary"),"target kind options missing");
    [...document.querySelectorAll<HTMLElement>('.ant-select-item-option')].find(n=>n.textContent==="MasterMemory binary")!.click();
    button("Add",input.closest('.ant-modal')!).click();await until(()=>!shown('input[aria-label="New target path"]'),"target addition did not commit");await ready();
    await until(()=>document.activeElement?.getAttribute('aria-label')==="New Publish target","target creation lost keyboard focus");
    await until(()=>!!document.querySelector('button[aria-label="Publish target 1 path"]'),"new target occurrence missing");
    assert(!(await config()).before.includes('publish.targets')&&(await config()).after.includes('path = "delivery/masterdata.bytes"'),"target form unexpectedly saved config");
    [...document.querySelectorAll<HTMLElement>('.settings-drawer [role="tab"]')].find(n=>n.textContent==="Profiles")!.click();
    await until(()=>!!document.querySelector('button[aria-label="Edit Exclude tags 1"]'),"Profile draft was lost during section switch");
    find('button[aria-label="Edit Exclude tags 1"]').click();await until(()=>!!document.querySelector('input[aria-label="Edit Exclude tags 1"]'),"Tag replace missing");
    input=find('input[aria-label="Edit Exclude tags 1"]') as HTMLInputElement;text(input,"debug");key(input,"Enter");
    await until(()=>!shown('input[aria-label="Edit Exclude tags 1"]')&&shown('button[aria-label="Edit Exclude tags 1"]')?.textContent==="debug","repaired tag did not commit");await ready();
    find('button[aria-label="Save Project Settings"]').click();await until(()=>!desktop.surface.status.configDirty&&!desktop.surface.status.environmentError,"config repair did not restore service");
    await ready();
    [...document.querySelectorAll<HTMLElement>('.settings-drawer [role="tab"]')].find(n=>n.textContent==="Publish Targets")!.click();await until(()=>!!document.querySelector('button[aria-label="Publish target 1 path"]'),"saved target missing");
    find('button[aria-label="Publish target 1 path"]').click();await until(()=>!!document.querySelector('input[aria-label="Publish target 1 path"]'),"target path editor missing");
    input=find('input[aria-label="Publish target 1 path"]') as HTMLInputElement;text(input,"delivery/latest.bytes");find('.settings-drawer .ant-drawer-close').click();
    await until(()=>!desktop.surface.settingsOpen,"Settings did not close");await desktop.selectTarget("sources/catalog-schema.yaml","settings-away",false);await settings();
    assert((find('input[aria-label="Publish target 1 path"]') as HTMLInputElement).value==="delivery/latest.bytes","source navigation discarded temporary config input");
    key(find('input[aria-label="Publish target 1 path"]'),"s",{ctrlKey:true});await until(()=>!desktop.surface.status.configDirty&&!desktop.surface.settingsInputDirty,"revisited active config Save missing");
    await ready();
    assert((await config()).before.includes('path = "delivery/latest.bytes"'),"revisited active input was not saved");assert((await yaml()).after===draft,"revisit config Save lost YAML draft");checks.push("section-target-draft-navigation");
    find('button[aria-label="Publish target 1 path"]').click();await until(()=>!!document.querySelector('input[aria-label="Publish target 1 path"]'),"conflict input missing");
    input=find('input[aria-label="Publish target 1 path"]') as HTMLInputElement;text(input,"delivery/conflict.bytes");key(input,"Enter");await until(()=>desktop.surface.status.configDirty,"conflict draft missing");
    await invoke("evidence_write",{report:{phase:"settings-external"}});
    await until(()=>!![...document.querySelectorAll<HTMLElement>('.settings-drawer .ant-alert')].find(n=>n.textContent?.includes("外部で変更")),"config external Conflict not observed");
    button("Compare",find('.settings-drawer')).click();await until(()=>!!document.querySelector('textarea[aria-label="Config comparison before"]'),"config Compare missing");
    assert((find('textarea[aria-label="Config comparison before"]') as HTMLTextAreaElement).value.includes("External settings change"),"Compare reused cached config");
    assert((find('textarea[aria-label="Config comparison after"]') as HTMLTextAreaElement).value.includes("delivery/conflict.bytes"),"Compare lost draft target occurrence");
    assert(![...document.querySelectorAll<HTMLButtonElement>('.ant-modal button')].some(n=>n.textContent?.includes("Overwrite")),"config exposes Overwrite");
    button("Close",find('textarea[aria-label="Config comparison before"]').closest('.ant-modal')!).click();
    button("Reload…",find('.settings-drawer')).click();await choose("Cancel");assert((await config()).after.includes("delivery/conflict.bytes"),"Reload Cancel lost config draft");
    button("Reload…",find('.settings-drawer')).click();await choose("Reload");
    await until(()=>!desktop.surface.status.configDirty&&!desktop.surface.status.environmentError,"config Reload did not restore fresh service");
    await ready();
    assert(!(await config()).after.includes("delivery/conflict.bytes")&&(await yaml()).after===draft,"config-only Reload discarded YAML draft");checks.push("config-conflict-compare-reload");
    find('button[aria-label="Publish target 1 path"]').click();await until(()=>!!shown('input[aria-label="Publish target 1 path"]'),"uncertain input missing");
    input=find('input[aria-label="Publish target 1 path"]') as HTMLInputElement;text(input,"delivery/uncertain.bytes");
    await invoke('evidence_bad_config_reply');key(input,"s",{ctrlKey:true});
    await until(()=>!![...document.querySelectorAll<HTMLElement>('.settings-drawer .ant-alert')].find(n=>n.textContent?.includes("Recheck actual config")),"unusable commit reply was not Outcome Unknown");
    assert((await config()).before.includes('delivery/uncertain.bytes'),"native config commit was not actually completed");
    assert((find('button[aria-label="Save Project Settings"]') as HTMLButtonElement).disabled,"unknown config Save remained enabled");
    await desktop.saveAll();assert((await yaml()).before===originalYaml.before,"unknown config allowed automatic Save All retry");
    button('Recheck actual config',find('.settings-drawer')).click();await ready();
    find('button[aria-label="Publish target 1 path"]').click();await until(()=>!!shown('input[aria-label="Publish target 1 path"]'),"rechecked input missing");
    input=find('input[aria-label="Publish target 1 path"]') as HTMLInputElement;text(input,"delivery/latest.bytes");key(input,"s",{ctrlKey:true});
    await until(()=>!desktop.surface.status.configDirty&&!desktop.surface.settingsInputDirty,"explicit config edit after Recheck did not save");await ready();
    assert((await config()).before.includes('delivery/latest.bytes'),"config did not restore after actual-byte Recheck");checks.push('config-unknown-reply-recheck');
    find('.settings-drawer .ant-drawer-close').click();await until(()=>!desktop.surface.settingsOpen,"Settings did not close");
    await settings();find('button[aria-label="New Publish target"]').click();await until(()=>!!document.querySelector('input[aria-label="New target path"]'),"guard form missing");
    input=find('input[aria-label="New target path"]') as HTMLInputElement;text(input,"unfinished target");
    await getCurrentWindow().close();await choose("Cancel");assert(input.value==="unfinished target"&&(await yaml()).after===draft,"native close Cancel lost config/YAML input");
    button("Cancel",input.closest('.ant-modal')!).click();
    const reload=desktop.reloadProject();await choose("Don't Save");await reload;
    await until(()=>desktop.surface.inventory?.root===root&&!desktop.surface.settingsOpen&&!desktop.surface.status.dirty.length,"global Don't Save did not replace authoring session");
    await desktop.selectTarget(SOURCE,"settings-clean",false);assert((await yaml()).after===originalYaml.after,"global discard changed YAML disk");checks.push("all-dirty-guard-cancel-discard");
    await invoke("evidence_write",{report:{format:1,kind:"settings",checks,startup,config:await config(),source:await yaml(),visibility:document.visibilityState,focused:document.hasFocus(),mountedRows:document.querySelectorAll('.grid-row').length}});
  }catch(error){await invoke("evidence_write",{report:{format:1,kind:"settings",error:String(error),checks,startup,active:document.activeElement?.outerHTML.slice(0,2000),dom:document.querySelector('.settings-drawer')?.outerHTML.slice(0,16000),modals:[...document.querySelectorAll('.ant-modal')].map(n=>n.outerHTML.slice(0,5000)),surfaceError:desktop.surface.error}});}
}
