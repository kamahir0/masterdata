import {invoke} from "@tauri-apps/api/core";
import {desktop} from "./workspace";

const SOURCE="sources/catalog-data.yaml";
const frame=()=>new Promise<void>(resolve=>requestAnimationFrame(()=>resolve()));
async function until(test:()=>boolean,message:string) {const deadline=performance.now()+8000;while(!test()){if(performance.now()>deadline)throw new Error(message);await frame();}}
function assert(test:unknown,message:string):asserts test {if(!test)throw new Error(message);}
function key(e:HTMLElement,key:string,extra:KeyboardEventInit={}) {e.dispatchEvent(new KeyboardEvent("keydown",{key,bubbles:true,...extra}));}
function input(label:string) {const e=document.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`);assert(e,`missing input: ${label}`);return e;}
function text(e:HTMLInputElement,value:string) {Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value")!.set!.call(e,value);e.dispatchEvent(new Event("input",{bubbles:true}));}
function button(label:string) {const e=[...document.querySelectorAll<HTMLButtonElement>("button")].find(e=>e.getAttribute("aria-label")===label||e.textContent===label);assert(e,`missing action: ${label}`);return e;}
async function begin(category:string,filename:string) {
  await until(()=>!button("New source artifact").disabled,"New remained unavailable");
  button("New source artifact").click();
  await until(()=>[...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].some(e=>e.textContent===category),"New menu missing");
  [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find(e=>e.textContent===category)!.click();
  await until(()=>!!document.querySelector('input[aria-label="New artifact filename"]'),"temporary Explorer node missing");
  const file=input("New artifact filename");
  await until(()=>document.activeElement===file,"creation filename was not focused once");
  text(file,filename);
  await until(()=>file.getAttribute("value")===filename,"filename input was not published");
  await ready();
}
async function ready(){await until(()=>!document.querySelector('.creation-inline .ant-spin')&&!document.querySelector('.creation-inline .ant-alert'),"creation preflight did not become valid");}
async function focusedSource(label:string) {
  await until(()=>{
    const tree=document.activeElement;
    const target=tree?.getAttribute('aria-activedescendant');
    return tree?.getAttribute('role')==='tree'&&!!target&&!!document.getElementById(target)?.textContent?.includes(label);
  },`created ${label} was not the active keyboard target`);
}
async function choose(label:string,value:string) {
  const combo=input(label);combo.focus();key(combo,"ArrowDown",{keyCode:40,which:40});
  await until(()=>[...document.querySelectorAll<HTMLElement>('.ant-select-item-option-content')].some(e=>e.textContent===value),"choice missing");
  const option=[...document.querySelectorAll<HTMLElement>('.ant-select-item-option-content')].find(e=>e.textContent===value)!;option.click();
}
export async function run({startup}:{startup:Record<string,unknown>}) {
  const checks:string[]=[],created:string[]=[];
  try {
    await until(()=>!!desktop.surface.inventory,"Project not open");
    await desktop.selectTarget(SOURCE,"creation-setup",false);
    const p=desktop.surface.projection!;
    await desktop.rpc({kind:"editText",source:SOURCE,revision:p.revision,generation:p.generation,row:p.rows[0].id,field:"name",text:"Existing draft"});
    await desktop.selectTarget(SOURCE,"after-operation");
    const gridTop=desktop.viewport!.getBoundingClientRect().top;
    await begin("Table","cancelled.yaml");
    const file=input("New artifact filename");
    key(file,"Enter",{isComposing:true});await frame();
    assert(!!document.querySelector('.creation-inline'),"composition Enter created an artifact");
    key(file,"Escape");await until(()=>!document.querySelector('.creation-inline'),"Escape did not cancel");
    assert(desktop.surface.status.dirty.includes(SOURCE),"cancel discarded existing draft");
    assert(!desktop.surface.inventory!.sources.some(s=>s.path==='sources/cancelled.yaml'),"cancel created source");
    checks.push("inline-cancel-composition");

    await begin("Table","fresh-table.yaml");key(input("New artifact filename"),"Enter");
    await until(()=>desktop.surface.target==='sources/fresh-table.yaml'&&!desktop.surface.pending&&!!desktop.surface.projection,"created Table not selected");
    created.push('sources/fresh-table.yaml');
    await focusedSource('fresh-table.yaml');
    assert(desktop.surface.projection!.totalRows===0&&desktop.surface.projection!.canAdd,"new inline Table did not start empty");
    assert(desktop.surface.status.dirty.includes(SOURCE),"creation saved or discarded other draft");
    assert(desktop.viewport!.getBoundingClientRect().top===gridTop,"creation moved working grid");
    checks.push("inline-table-exclusive-create");

    await begin("Folder","catalog-new");key(input("New artifact filename"),"Enter");
    await until(()=>desktop.surface.inventory!.folders.includes('sources/catalog-new')&&!document.querySelector('.creation-inline'),"folder not published");
    assert(document.querySelector('[data-path="folder:sources/catalog-new"]'),"empty folder missing from Explorer");
    await focusedSource('catalog-new');
    checks.push("empty-folder-selection");
    await begin("Data","storage-name.yml");
    assert(document.querySelector('.creation-destination')!.textContent!.includes('fresh-table'),"Data did not expose explicit Table");
    key(input("New artifact filename"),"Enter");
    await until(()=>desktop.surface.target==='sources/catalog-new/storage-name.yml'&&!desktop.surface.pending&&!!desktop.surface.projection,"created Data not routed");
    created.push('sources/catalog-new/storage-name.yml');
    await focusedSource('storage-name.yml');
    assert(desktop.surface.projection!.table.name==='fresh-table'&&desktop.surface.projection!.totalRows===0,"filename changed Table identity or implicit rows appeared");
    checks.push("explicit-data-binding-yml");

    await begin("Enum","huge-token.yaml");button("Advanced creation").click();
    await until(()=>!!document.querySelector('[role="dialog"] input[aria-label="Artifact identity"]'),"Advanced dialog missing");
    text(input("Artifact identity"),"HugeToken");
    await choose("Underlying type","ulong");
    text(input("Member 1 name"),"Maximum");text(input("Member 1 value"),"18446744073709551615");
    await until(()=>!button("Create").disabled,"lossless Advanced declaration invalid");button("Create").click();
    await until(()=>desktop.surface.inventory!.types.includes('HugeToken')&&!document.querySelector('.creation-inline'),"Advanced Enum not published");
    created.push('sources/catalog-new/huge-token.yaml');
    await focusedSource('huge-token.yaml');
    checks.push("advanced-lossless-member");

    await desktop.selectTarget(SOURCE,"creation-return");
    assert(desktop.surface.projection!.rows[0].cells.find((_c,i)=>desktop.surface.projection!.columns[i].field.name==='name')!.display==='Existing draft',"creation lost source-local draft");
    assert(desktop.surface.projection!.canUndo,"creation lost source-local history");
    checks.push("dirty-revisit-after-creation");
    assert((startup.browserErrors as string[]).length===0,"browser errors");
    await invoke("evidence_write",{report:{kind:"creation",checks,created,dirty:desktop.surface.status.dirty,visibility:document.visibilityState,focused:document.hasFocus(),startup}});
  }catch(error){await invoke("evidence_write",{report:{kind:"creation",error:String(error),checks,created,startup,status:desktop.surface.status,active:document.activeElement?.outerHTML.slice(0,2000),creation:document.querySelector('.creation-inline')?.outerHTML,dialog:document.querySelector('[role="dialog"]')?.outerHTML}});}
}
