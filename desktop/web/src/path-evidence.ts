import {invoke} from "@tauri-apps/api/core";
import {desktop} from "./workspace";

const SOURCE="sources/catalog-data.yaml",SCHEMA="sources/catalog-schema.yaml";
const frame=()=>new Promise<void>(resolve=>requestAnimationFrame(()=>resolve()));
async function until(test:()=>boolean,message:string) {const deadline=performance.now()+8000;while(!test()){if(performance.now()>deadline)throw new Error(message);await frame();}}
function assert(test:unknown,message:string):asserts test {if(!test)throw new Error(message);}
function findButton(label:string) {return [...document.querySelectorAll<HTMLButtonElement>('.ant-modal:not(.ant-modal-hidden) button, #explorer button')].find(button=>button.getAttribute('aria-label')===label||button.textContent===label);}
function button(label:string) {const value=findButton(label);assert(value,`missing button ${label}`);return value;}
function input() {const value=document.querySelector<HTMLInputElement>('input[aria-label="Move source filename"]');assert(value,"Move filename missing");return value;}
function text(value:string) {const field=input();Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value')!.set!.call(field,value);field.dispatchEvent(new Event('input',{bubbles:true}));}
async function begin(choice?:string) {
  button('Source actions').click();
  await until(()=>[...document.querySelectorAll<HTMLElement>('.ant-dropdown:not(.ant-dropdown-hidden) [role="menuitem"]')].some(item=>item.textContent==='Rename / Move source'),"Source menu missing");
  [...document.querySelectorAll<HTMLElement>('.ant-dropdown:not(.ant-dropdown-hidden) [role="menuitem"]')].find(item=>item.textContent==='Rename / Move source')!.click();
  if(choice) {
    await until(()=>!!desktop.surface.choice&&!!findButton(choice),"rendered scoped guard missing");
    button(choice).click();
    if(choice==='Cancel'){await until(()=>!desktop.surface.choice,"guard Cancel pending");return;}
  }
  await until(()=>!!document.querySelector('input[aria-label="Move source filename"]'),"Move dialog missing");
  await until(()=>!button('Move').disabled,"initial Move review missing");
}
async function ready() {await until(()=>!button('Move').disabled&&!button('Move').classList.contains('ant-btn-loading'),"Move review missing");}
async function moved(destination:string) {button('Move').click();await until(()=>desktop.surface.target===destination&&!desktop.surface.pending&&desktop.surface.projection?.source===destination,"Move did not follow physical source");}
async function folder(value:string) {
  const combo=document.querySelector<HTMLInputElement>('input[aria-label="Move source folder"]');assert(combo,"Move folder missing");
  combo.focus();combo.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowDown',keyCode:40,which:40,bubbles:true}));
  await until(()=>[...document.querySelectorAll<HTMLElement>('.ant-select-item-option-content')].some(item=>item.textContent===value),"Move destination choice missing");
  [...document.querySelectorAll<HTMLElement>('.ant-select-item-option-content')].find(item=>item.textContent===value)!.click();
}
export async function run({startup}:{startup:Record<string,unknown>}) {
  const checks:string[]=[];
  try {
    await until(()=>!!desktop.surface.inventory,"Project not open");
    await desktop.selectTarget(SOURCE,"path-setup",false);
    let p=desktop.surface.projection!;
    await desktop.rpc({kind:'schema',source:SCHEMA,revision:p.schemaRevision,generation:p.generation,field:'name',nullable:true,array:false,typeName:'string'});
    await desktop.selectTarget(SOURCE,'path-setup',false);
    const gridTop=desktop.viewport!.getBoundingClientRect().top;
    await begin();text('cancelled.yaml');
    input().dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',isComposing:true,bubbles:true}));await frame();
    assert(desktop.surface.target===SOURCE,"composition Enter moved source");
    input().dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}));
    await until(()=>!document.querySelector('input[aria-label="Move source filename"]'),"Escape did not cancel Move");
    checks.push('path-cancel-composition');

    p=desktop.surface.projection!;
    await desktop.rpc({kind:'editText',source:SOURCE,revision:p.revision,generation:p.generation,row:p.rows[0].id,field:'name',text:'Saved before move'});
    await desktop.selectTarget(SOURCE,'path-dirty',false);
    await begin('Cancel');
    assert(desktop.surface.projection!.dirty&&desktop.surface.projection!.canUndo,"guard Cancel lost target draft/history");
    checks.push('target-dirty-cancel');
    await begin('Save');
    assert(desktop.surface.status.dirty.includes(SCHEMA)&&!desktop.surface.status.dirty.includes(SOURCE),"Move guard saved unrelated schema");
    text('renamed-data.yml');await ready();await moved('sources/renamed-data.yml');
    assert(desktop.surface.projection!.table.name==='item',"filename changed logical Table binding");
    checks.push('target-only-save-rename');

    await begin();text('Renamed-data.yml');await ready();await moved('sources/Renamed-data.yml');
    checks.push('case-only-rename');
    p=desktop.surface.projection!;
    await desktop.rpc({kind:'editText',source:p.source,revision:p.revision,generation:p.generation,row:p.rows[0].id,field:'name',text:'Discard this draft only'});
    await desktop.selectTarget(p.source!,'path-dirty',false);
    await begin("Don't Save");
    await folder('sources/moved');await ready();await moved('sources/moved/Renamed-data.yml');
    assert(desktop.surface.projection!.rows[0].cells.find((_cell,i)=>desktop.surface.projection!.columns[i].field.name==='name')!.display==='Saved before move',"Don't Save did not discard only target draft");
    assert(desktop.surface.status.dirty.includes(SCHEMA)&&desktop.surface.projection!.schemaCanUndo,"Move lost unrelated schema/history");
    assert(desktop.viewport!.getBoundingClientRect().top===gridTop,"Move shifted working surface");
    checks.push('target-only-discard-move');
    await begin();await folder('sources');text('catalog-schema.yaml');
    await until(()=>!!document.querySelector('.ant-modal .ant-alert'),"existing destination did not fail preflight");
    assert(button('Move').disabled,"destination collision offered overwrite");button('Cancel').click();
    checks.push('destination-conflict-no-overwrite');
    assert((startup.browserErrors as string[]).length===0,"browser errors");
    await invoke('evidence_write',{report:{kind:'path',checks,destination:desktop.surface.projection!.source,dirty:desktop.surface.status.dirty,visibility:document.visibilityState,focused:document.hasFocus(),startup}});
  } catch(error) {
    await invoke('evidence_write',{report:{kind:'path',error:String(error),checks,startup,status:desktop.surface.status,projection:desktop.surface.projection,surfaceError:desktop.surface.error,active:document.activeElement?.outerHTML.slice(0,1500),dialog:document.querySelector('[role="dialog"]')?.outerHTML}});
  }
}
