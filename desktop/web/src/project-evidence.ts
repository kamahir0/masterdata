import {invoke} from "@tauri-apps/api/core";
import {desktop} from "./workspace";
const SOURCE="sources/catalog-data.yaml";
const frame=()=>new Promise<void>(resolve=>requestAnimationFrame(()=>resolve()));
async function until(test:()=>boolean,message:string){const deadline=performance.now()+8000;while(!test()){if(performance.now()>deadline)throw new Error(message);await frame();}}
function assert(test:unknown,message:string):asserts test {if(!test)throw new Error(message);}
const shown=(e:HTMLElement)=>!!e.getClientRects().length&&getComputedStyle(e).visibility!=="hidden";
function button(label:string){const e=[...document.querySelectorAll<HTMLButtonElement>('button')].find(e=>shown(e)&&(e.getAttribute('aria-label')===label||e.textContent===label));assert(e,`missing action: ${label}`);return e;}
function input(label:string){const e=[...document.querySelectorAll<HTMLInputElement>(`input[aria-label="${label}"]`)].find(shown);assert(e,`missing input: ${label}`);return e;}
function projectButton(label:string){const e=[...input('Project destination').closest('.ant-modal')!.querySelectorAll<HTMLButtonElement>('button')].find(e=>shown(e)&&e.textContent===label);assert(e,`missing Project action: ${label}`);return e;}
function text(e:HTMLInputElement,value:string){Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value')!.set!.call(e,value);e.dispatchEvent(new Event('input',{bubbles:true}));}
function key(e:HTMLElement,key:string,extra:KeyboardEventInit={}){e.dispatchEvent(new KeyboardEvent('keydown',{key,bubbles:true,...extra}));}
async function startCreate(){button('Project menu').click();await until(()=>[...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].some(e=>shown(e)&&e.textContent==='Create Project…'),'Create Project menu missing');[...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find(e=>shown(e)&&e.textContent==='Create Project…')!.click();}
async function guard(answer:string){await until(()=>desktop.surface.choice!==null&&[...document.querySelectorAll<HTMLButtonElement>('button')].some(e=>shown(e)&&e.textContent===answer),'all-dirty guard action not rendered');button(answer).click();}
async function fill(path:string,name='New Project'){
  await until(()=>!!desktop.surface.projectCreation,'Create Project form missing');
  text(input('Project destination'),path);text(input('Project ID'),'new.masterdata');text(input('Project name'),name);
    await until(()=>!projectButton('Create Project').disabled,'Project form not ready');
}
async function readyArtifact(){await until(()=>!!document.querySelector('.creation-inline')&&!document.querySelector('.creation-inline .ant-spin')&&!document.querySelector('.creation-inline .ant-alert'),'artifact preview did not become valid');}
export async function run({startup}:{startup:Record<string,unknown>}){
  const checks:string[]=[];let oldRoot='',newRoot='',fixtureRoot='';
  try {
    await until(()=>document.hasFocus(),'native window not focused');
    assert(!desktop.surface.inventory&&desktop.surface.recentProjects.length===2,'Welcome auto-opened a Project or lost recent entries');
    // Use the caller-selected test path for new destinations. Windows native
    // canonical roots are verbatim paths, where '/..' is not a path component.
    fixtureRoot=desktop.surface.recentProjects[0].root;
    button('Create Project').click();await fill(`${fixtureRoot}/../welcome-cancel`);
    key(input('Project name'),'ArrowLeft');projectButton('Cancel').click();
    await until(()=>!desktop.surface.projectCreation&&document.activeElement?.getAttribute('aria-label')==='Create Project','Welcome Cancel did not return keyboard focus to Create Project');
    assert(!desktop.surface.inventory&&desktop.surface.recentProjects.length===2,'Welcome Cancel changed Project / recent state');
    checks.push('welcome-create-cancel-focus');
    button('Remove recent Remove entry').click();await until(()=>desktop.surface.recentProjects.length===1,'Recent removal did not commit');
    await invoke('evidence_bad_project_reply',{opening:true});button('Workspace').click();
    await until(()=>!!desktop.surface.projectOpenUncertain&&!desktop.surface.openingProject,'lost Open reply did not stop the stale view');
    assert(!desktop.surface.inventory&&document.querySelector<HTMLElement>('#workbench')!.inert,'unknown Open exposed an unaccepted workspace');
    button('Open destination…').click();
    await until(()=>!!desktop.surface.inventory&&!desktop.surface.openingProject,'Recent Project did not open');oldRoot=desktop.surface.inventory!.root;
    checks.push('welcome-recents-explicit-open-remove');checks.push('unknown-open-reply-explicit-reopen');

    await desktop.selectTarget(SOURCE,'project-setup',false);
    desktop.setSelection(0,desktop.surface.projection!.columns.findIndex(c=>c.field.name==='name'));desktop.beginEditor();
    await until(()=>!!document.querySelector('.active-cell-editor input'),'scalar editor missing');
    text(document.querySelector<HTMLInputElement>('.active-cell-editor input')!,'Keep this old draft');
    await startCreate();await guard('Cancel');await frame();
    assert(!desktop.surface.projectCreation&&desktop.surface.status.dirty.includes(SOURCE),'guard Cancel opened creation or discarded the committed draft');
    await until(()=>document.activeElement?.getAttribute('role')==='grid','guard Cancel did not restore authoring keyboard focus');
    checks.push('create-all-dirty-guard-cancel');

    await startCreate();await guard("Don't Save");await fill(`${fixtureRoot}/../cancelled`);
    key(input('Project name'),'Enter',{isComposing:true});await frame();assert(!desktop.surface.openingProject,'composition Enter created Project');
    projectButton('Cancel').click();await until(()=>!desktop.surface.projectCreation,'creation Cancel failed');
    await until(()=>document.activeElement?.getAttribute('aria-label')==='Project menu'||document.activeElement?.getAttribute('role')==='grid','form Cancel did not restore a usable authoring/action focus');
    assert(desktop.surface.status.dirty.includes(SOURCE),'form Cancel dropped the old authoring draft');
    checks.push('create-form-cancel-composition');

    await startCreate();await guard("Don't Save");await fill(oldRoot);projectButton('Create Project').click();
    await until(()=>!!document.querySelector('.ant-modal .ant-alert')&&!desktop.surface.openingProject,'nonempty creation failure missing');
    assert(desktop.surface.inventory!.root===oldRoot&&desktop.surface.status.dirty.includes(SOURCE),'creation failure replaced the old workspace');
    checks.push('nonempty-failure-retains-workspace');

    text(input('Project destination'),`${fixtureRoot}/../created`);await until(()=>!projectButton('Create Project').disabled,'new destination not ready');
    await invoke('evidence_bad_project_reply');projectButton('Create Project').click();
    await until(()=>desktop.surface.projectCreationUncertain&&!desktop.surface.openingProject,'lost creation reply did not become uncertain');
    await until(()=>[...document.querySelectorAll<HTMLButtonElement>('button')].some(e=>shown(e)&&e.textContent==='Open destination…'),'unknown creation report did not reach the rendered dialog');
    assert(projectButton('Cancel').disabled&&projectButton('Create Project').disabled&&document.querySelector<HTMLElement>('#workbench')!.inert,'unknown result allowed retry/cancel or exposed stale editing');
    button('Open destination…').click();await until(()=>!desktop.surface.projectCreation&&desktop.surface.inventory?.project.name==='New Project'&&!desktop.surface.openingProject,'explicit fresh Open did not resolve the creation outcome');
    newRoot=desktop.surface.inventory!.root;
    assert(desktop.surface.inventory!.sources.length===0&&!desktop.surface.status.dirty.length,'Project creation fabricated data');
    checks.push('unknown-creation-reply-explicit-open');

    await until(()=>!button('New Table').disabled,'empty Project lacks guided Table action');button('New Table').click();
    await readyArtifact();button('Advanced creation').click();
    await until(()=>[...document.querySelectorAll<HTMLLabelElement>('.ant-modal label')].some(e=>shown(e)&&e.textContent==='Empty inline records'),'inline storage choice missing');
    [...document.querySelectorAll<HTMLLabelElement>('.ant-modal label')].find(e=>shown(e)&&e.textContent==='Empty inline records')!.click();
    await until(()=>!button('Create').disabled,'schema-only preview not ready');button('Create').click();
    await until(()=>!!desktop.surface.projection&&!desktop.surface.pending&&!document.querySelector('.creation-inline'),'guided Table did not open');
    assert(Number(desktop.surface.inventory!.sources.length)===1&&desktop.surface.projection!.totalRows===0&&!desktop.surface.projection!.canAdd,'schema creation invented a record destination');
    const table=desktop.surface.inventory!.logicalTables[0];assert(table&&table.name===desktop.surface.projection!.table.name,'logical Table identity was derived from filename');
    checks.push('guided-schema-explicit-no-record-source');

    button('New source artifact').click();await until(()=>[...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].some(e=>shown(e)&&e.textContent==='Data'),'Data menu missing');
    [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find(e=>shown(e)&&e.textContent==='Data')!.click();
    await readyArtifact();key(input('New artifact filename'),'Enter');
    await until(()=>desktop.surface.inventory!.sources.length===2&&desktop.surface.projection!.canAdd&&!desktop.surface.pending,'explicit Data creation did not bind to Table');
    const source=desktop.surface.projection!.source!;await desktop.addRow();
    await until(()=>desktop.surface.status.dirty.includes(source),'Add Row did not create its source-local draft');
    const logical=[...document.querySelectorAll<HTMLElement>('.source')].find(e=>e.getAttribute('data-path')===table.source&&e.textContent?.startsWith(table.name));assert(logical,'logical Table navigation missing');logical.click();
    await until(()=>desktop.surface.target===table.source&&!desktop.surface.pending,'logical Table navigation failed');
    assert(desktop.surface.projection!.source===source&&desktop.surface.projection!.rows.some(row=>row.added),'logical navigation did not share the physical draft');
    checks.push('explicit-data-logical-navigation-shared-draft');

    const recent=desktop.surface.recentProjects;assert(recent.length===2&&recent[0].root===newRoot&&recent[1].root===oldRoot,'recents were not canonical-root deduplicated in successful order');
    checks.push('recent-canonical-root-order-dedup');
  }catch(error){await invoke('evidence_write',{report:{error:String(error),checks,startup,visibleDialogs:[...document.querySelectorAll<HTMLElement>('[role="dialog"]')].filter(shown).map(e=>e.textContent?.slice(0,600)),focus:document.activeElement?.getAttribute('aria-label')}});return;}
  await invoke('evidence_write',{report:{checks,oldRoot,newRoot,visibility:document.visibilityState,focused:document.hasFocus(),startup}});
}
