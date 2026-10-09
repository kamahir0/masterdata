import {invoke} from "@tauri-apps/api/core";
import {desktop} from "./workspace";
const SOURCE="sources/catalog-data.yaml",SCHEMA="sources/catalog-schema.yaml";
const frame=()=>new Promise<void>(resolve=>requestAnimationFrame(()=>resolve()));
async function until(test:()=>boolean,message:string){const deadline=performance.now()+8000;while(!test()){if(performance.now()>deadline)throw new Error(message);await frame();}}
function assert(test:unknown,message:string):asserts test{if(!test)throw new Error(message);}
function source(path:string){const e=document.querySelector<HTMLElement>(`#explorer [data-path="${path}"]`);assert(e,`source not visible: ${path}`);return e;}
function tree(){const e=document.querySelector<HTMLElement>('#explorer [role="tree"]');assert(e,'Explorer tree missing');return e;}
function key(e:HTMLElement,key:string,isComposing=false){const code:Record<string,number>={Enter:13,ArrowDown:40};e.dispatchEvent(new KeyboardEvent('keydown',{key,keyCode:code[key]??0,which:code[key]??0,bubbles:true,isComposing}));}
export async function run({startup}:{startup:Record<string,unknown>}){
  const checks:string[]=[];
  try {
    await until(()=>document.hasFocus()&&!!desktop.surface.inventory,'Project / foreground unavailable');
    desktop.inputCapture=true;
    const programmatic=await desktop.selectTarget(SOURCE,'programmatic');
    desktop.viewport!.focus();desktop.setSelection(0,1);
    assert(programmatic.inputOrigin==='programmatic'&&!programmatic.firstAccepted,'programmatic focus / selection masqueraded as OS interaction');
    checks.push('programmatic-is-not-os-input');

    source(SCHEMA).click();await until(()=>desktop.surface.target===SCHEMA&&!desktop.surface.pending,'synthetic source click not accepted');await frame();
    const synthetic=desktop.currentSample!;
    assert(synthetic.inputOrigin==='synthetic','synthetic click mislabeled');
    desktop.viewport!.focus();key(desktop.viewport!,'ArrowRight');
    assert(!synthetic.firstAccepted&&synthetic.probes?.at(-1)?.origin==='synthetic','synthetic Arrow accepted as OS input');
    checks.push('synthetic-is-not-os-input');

    source(SOURCE).click();await until(()=>desktop.surface.target===SOURCE&&!desktop.surface.pending,'source not selected');
    tree().focus();key(tree(),'ArrowDown');await frame();
    const beforeComposition=desktop.samples.length;
    key(tree(),'Enter',true);await frame();
    assert(tree().contains(document.activeElement)&&desktop.samples.length===beforeComposition&&desktop.surface.target===SOURCE,'composition Enter opened source / moved pane focus');
    source(SOURCE).click();tree().focus();
    key(tree(),'Enter');await until(()=>desktop.surface.target===SOURCE&&!desktop.surface.pending&&document.activeElement===desktop.viewport,'Enter did not open / focus selected editor');
    const selected=desktop.interaction.selection,top=desktop.viewport!.scrollTop;
    key(desktop.viewport!,'F6');assert(tree().contains(document.activeElement),'F6 did not restore Explorer');
    key(tree(),'F6');assert(document.activeElement===desktop.viewport,'F6 did not restore editor');
    assert(desktop.interaction.selection===selected&&desktop.viewport!.scrollTop===top,'pane focus changed selection / scroll');
    checks.push('enter-composition-pane-focus-and-return');

    source(SCHEMA).click();tree().focus();key(tree(),'Enter');
    key(document.getElementById('editor-pane')!,'F6');
    await until(()=>desktop.surface.target===SCHEMA&&!desktop.surface.pending,'source selection did not finish');await frame();
    assert(tree().contains(document.activeElement),'old projection completion stole newer pane focus');
    checks.push('completion-preserves-newer-pane-focus');

    source(SOURCE).click();tree().focus();key(tree(),'Enter');
    source(SCHEMA).click();tree().focus();key(tree(),'Enter');
    await until(()=>desktop.surface.target===SCHEMA&&!desktop.surface.pending&&document.activeElement===desktop.viewport,'latest keyboard-open target did not win');
    assert(desktop.surface.projection?.clicked===SCHEMA,'old content / focus won rapid keyboard open');
    checks.push('latest-keyboard-open-wins');

    let asynchronous:ReturnType<typeof desktop.selectTarget>|null=null;
    const clicked=source(SOURCE);
    clicked.addEventListener('click',()=>queueMicrotask(()=>{asynchronous=desktop.selectTarget(SCHEMA,'post-event');}),{once:true});
    clicked.click();await until(()=>asynchronous!==null,'post-event read not scheduled');
    const later=await asynchronous!;
    assert(later.inputOrigin==='programmatic','event trust leaked into asynchronous selection');
    assert(desktop.samples.every(sample=>!sample.firstAccepted),'controlled evidence forged first OS interaction');
    assert(desktop.surface.status.dirty.length===0,'keyboard focus dirtied source');
    checks.push('event-trust-expires-after-dispatch');
    await invoke('evidence_write',{report:{kind:'focus',checks,startup,visibility:document.visibilityState,focused:document.hasFocus(),dirty:desktop.surface.status.dirty,
      samples:desktop.samples,active:document.activeElement?.outerHTML.slice(0,300)}});
  }catch(error){await invoke('evidence_write',{report:{kind:'focus',error:String(error),checks,startup,status:desktop.surface.status,target:desktop.surface.target,
    samples:desktop.samples,active:document.activeElement?.outerHTML.slice(0,1000)}});}
}
