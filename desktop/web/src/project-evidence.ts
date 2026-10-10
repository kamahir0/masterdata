import {invoke} from "@tauri-apps/api/core";
import {desktop} from "./workspace";
const SOURCE="sources/catalog-data.yaml";
const frame=()=>new Promise<void>(resolve=>requestAnimationFrame(()=>resolve()));
async function until(test:()=>boolean,message:string){const deadline=performance.now()+8000;while(!test()){if(performance.now()>deadline)throw new Error(message);await frame();}}
function assert(test:unknown,message:string):asserts test {if(!test)throw new Error(message);}
const shown=(e:HTMLElement)=>!!e.getClientRects().length&&getComputedStyle(e).visibility!=="hidden";
function button(label:string){const e=[...document.querySelectorAll<HTMLButtonElement>('button')].find(e=>shown(e)&&(e.getAttribute('aria-label')===label||e.textContent===label));assert(e,`missing action: ${label}`);return e;}
function input(label:string){const e=[...document.querySelectorAll<HTMLInputElement>(`input[aria-label="${label}"]`)].find(shown);assert(e,`missing input: ${label}`);return e;}
function projectButton(label:string){const e=[...input("プロジェクトの保存先").closest('.ant-modal')!.querySelectorAll<HTMLButtonElement>('button')].find(e=>shown(e)&&e.textContent===label);assert(e,`missing Project action: ${label}`);return e;}
function text(e:HTMLInputElement,value:string){Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value')!.set!.call(e,value);e.dispatchEvent(new Event('input',{bubbles:true}));}
function key(e:HTMLElement,key:string,extra:KeyboardEventInit={}){e.dispatchEvent(new KeyboardEvent('keydown',{key,bubbles:true,...extra}));}
async function startCreate(){button("プロジェクトメニュー").click();await until(()=>[...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].some(e=>shown(e)&&e.textContent==="プロジェクトを作成…"),'Create Project menu missing');[...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find(e=>shown(e)&&e.textContent==="プロジェクトを作成…")!.click();}
async function guard(answer:string){await until(()=>desktop.surface.choice!==null&&[...document.querySelectorAll<HTMLButtonElement>('button')].some(e=>shown(e)&&e.textContent===answer),'all-dirty guard action not rendered');button(answer).click();}
async function fill(path:string,name='New Project'){
  await until(()=>!!desktop.surface.projectCreation,'Create Project form missing');
  text(input("プロジェクトの保存先"),path);text(input("プロジェクトID"),'new.masterdata');text(input("プロジェクト名"),name);
    await until(()=>!projectButton("プロジェクトを作成").disabled,'Project form not ready');
}
async function readyArtifact(){await until(()=>!!document.querySelector('.creation-inline')&&!document.querySelector('.creation-inline .ant-spin')&&!document.querySelector('.creation-inline .ant-alert'),'artifact preview did not become valid');}
async function openFind(){
  button("ソースの操作").click();
  await until(()=>[...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].some(e=>shown(e)&&e.textContent==="テーブル・型を検索…"),'contextual Find action missing');
  [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find(e=>shown(e)&&e.textContent==="テーブル・型を検索…")!.click();
  await until(()=>!!document.querySelector("input[aria-label=\"テーブルまたは型を検索\"]"),'Find input missing');
}
async function findTarget(query:string,label:string){
  await openFind();
  text(input("テーブルまたは型を検索"),query);
  await until(()=>[...document.querySelectorAll<HTMLElement>('.ant-select-item-option-content')].some(e=>shown(e)&&e.textContent===label),'matching logical target missing');
  [...document.querySelectorAll<HTMLElement>('.ant-select-item-option-content')].find(e=>shown(e)&&e.textContent===label)!.click();
}
export async function run({startup}:{startup:Record<string,unknown>}){
  const checks:string[]=[];let oldRoot='',newRoot='',fixtureRoot='';
  try {
    await until(()=>document.hasFocus(),'native window not focused');
    assert(!desktop.surface.inventory&&desktop.surface.recentProjects.length===2,'Welcome auto-opened a Project or lost recent entries');
    assert(document.querySelector('.recent-projects')?.textContent?.includes('最近開いたプロジェクト'),'Welcome recent section missing');
    button('アプリケーション設定').click();
    await until(()=>!!document.querySelector('.ant-radio-group'),'Welcome lacks appearance settings');
    assert(document.querySelectorAll('.ant-radio-button-wrapper').length===3,'Light / Dark / System choices missing');
    button('完了').click();await until(()=>!desktop.surface.appearance,'appearance settings did not close');
    checks.push('welcome-recent-and-appearance-entry');
    // Use the caller-selected test path for new destinations. Windows native
    // canonical roots are verbatim paths, where '/..' is not a path component.
    fixtureRoot=desktop.surface.recentProjects[0].root;
    button("プロジェクトを作成").click();await fill(`${fixtureRoot}/../welcome-cancel`);
    key(input("プロジェクト名"),'ArrowLeft');projectButton("キャンセル").click();
    await until(()=>!desktop.surface.projectCreation&&document.activeElement?.getAttribute('aria-label')==="プロジェクトを作成",'Welcome Cancel did not return keyboard focus to Create Project');
    assert(!desktop.surface.inventory&&desktop.surface.recentProjects.length===2,'Welcome Cancel changed Project / recent state');
    checks.push('welcome-create-cancel-focus');
    button('最近開いた項目を削除: Remove entry').click();await until(()=>desktop.surface.recentProjects.length===1,'Recent removal did not commit');
    await invoke('evidence_bad_project_reply',{opening:true});button('Workspace').click();
    await until(()=>!!desktop.surface.projectOpenUncertain&&!desktop.surface.openingProject,'lost Open reply did not stop the stale view');
    assert(!desktop.surface.inventory&&document.querySelector<HTMLElement>('#workbench')!.inert,'unknown Open exposed an unaccepted workspace');
    button("保存先を開く…").click();
    await until(()=>!!desktop.surface.inventory&&!desktop.surface.openingProject,'Recent Project did not open');oldRoot=desktop.surface.inventory!.root;
    checks.push('welcome-recents-explicit-open-remove');checks.push('unknown-open-reply-explicit-reopen');

    await desktop.selectTarget(SOURCE,'project-setup',false);
    const treeLabels=[...document.querySelectorAll<HTMLElement>('#explorer .source')].map(e=>e.textContent!.trim()).sort();
    const physicalLabels=desktop.surface.inventory!.sources.map(source=>source.path.split('/').at(-1)!).sort();
    assert(JSON.stringify(treeLabels)===JSON.stringify(physicalLabels),'Explorer mixed domain groups into the physical source hierarchy');
    const selection=desktop.interaction.selection;
    const retainedTree=document.querySelector('#explorer [role="tree"]'),sourceProjection=desktop.surface.projection;
    const beforeWidth=document.getElementById('editor-pane')!.getBoundingClientRect().width;
    button('エクスプローラーを閉じる').click();await frame();
    assert(!shown(document.getElementById('explorer')!)&&document.getElementById('editor-pane')!.getBoundingClientRect().width>beforeWidth,'sidebar close did not release working space');
    key(document.getElementById('editor-pane')!,'b',{ctrlKey:true});await frame();
    assert(shown(document.getElementById('explorer')!)&&retainedTree===document.querySelector('#explorer [role="tree"]')&&desktop.surface.projection===sourceProjection&&desktop.interaction.selection===selection,'sidebar toggle discarded tree / projection / selection');
    checks.push('sidebar-whole-pane-state-preserved');
    await openFind();text(input("テーブルまたは型を検索"),'not a target');
    input("テーブルまたは型を検索").closest('.ant-modal')!.querySelector<HTMLButtonElement>("button[aria-label=\"閉じる\"]")!.click();
    await until(()=>!document.querySelector("input[aria-label=\"テーブルまたは型を検索\"]")&&!!document.activeElement?.closest('[role="tree"]'),'Find Cancel did not restore Explorer keyboard focus');
    assert(desktop.surface.target===SOURCE&&desktop.interaction.selection===selection&&!desktop.surface.status.dirty.length,'Find Cancel changed source / selection / draft');
    checks.push('source-only-tree-contextual-find-cancel');
    await findTarget('ItemId','型 · ItemId — sources/item-id.yaml');
    await until(()=>desktop.surface.typeProjection?.name==='ItemId'&&!desktop.surface.pending,'Find did not open the shared typed source');
    const work=desktop.surface.typeProjection!.measurement.work;
    assert(work.projectDiscovery===0&&work.projectEnumeration===0&&work.projectYamlParse===0&&work.projectValidation===0,'logical Find reopened / rebuilt the Project');
    await findTarget('item','テーブル · item — sources/catalog-schema.yaml');
    await until(()=>desktop.surface.target==='sources/catalog-schema.yaml'&&desktop.surface.projection?.source===SOURCE&&!desktop.surface.pending,'logical Table did not use the same physical record context');
    assert(document.querySelector('#explorer [data-path="sources/catalog-schema.yaml"]')!.closest('[aria-selected="true"]'),'logical Find did not select its physical source row');
    checks.push('contextual-find-type-table-warm-zero');
    await desktop.selectTarget(SOURCE,'project-edit',false);
    desktop.setSelection(0,desktop.surface.projection!.columns.findIndex(c=>c.field.name==='name'));desktop.beginEditor();
    await until(()=>!!document.querySelector('.active-cell-editor input'),'scalar editor missing');
    text(document.querySelector<HTMLInputElement>('.active-cell-editor input')!,'Keep this old draft');
    await startCreate();await guard("キャンセル");await frame();
    assert(!desktop.surface.projectCreation&&desktop.surface.status.dirty.includes(SOURCE),'guard Cancel opened creation or discarded the committed draft');
    await until(()=>document.activeElement?.getAttribute('role')==='grid','guard Cancel did not restore authoring keyboard focus');
    checks.push('create-all-dirty-guard-cancel');

    await startCreate();await guard("保存しない");await fill(`${fixtureRoot}/../cancelled`);
    key(input("プロジェクト名"),'Enter',{isComposing:true});await frame();assert(!desktop.surface.openingProject,'composition Enter created Project');
    projectButton("キャンセル").click();await until(()=>!desktop.surface.projectCreation,'creation Cancel failed');
    await until(()=>document.activeElement?.getAttribute('aria-label')==="プロジェクトメニュー"||document.activeElement?.getAttribute('role')==='grid','form Cancel did not restore a usable authoring/action focus');
    assert(desktop.surface.status.dirty.includes(SOURCE),'form Cancel dropped the old authoring draft');
    checks.push('create-form-cancel-composition');

    await startCreate();await guard("保存しない");await fill(oldRoot);projectButton("プロジェクトを作成").click();
    await until(()=>!!document.querySelector('.ant-modal .ant-alert')&&!desktop.surface.openingProject,'nonempty creation failure missing');
    assert(desktop.surface.inventory!.root===oldRoot&&desktop.surface.status.dirty.includes(SOURCE),'creation failure replaced the old workspace');
    checks.push('nonempty-failure-retains-workspace');

    text(input("プロジェクトの保存先"),`${fixtureRoot}/../created`);await until(()=>!projectButton("プロジェクトを作成").disabled,'new destination not ready');
    await invoke('evidence_bad_project_reply');projectButton("プロジェクトを作成").click();
    await until(()=>desktop.surface.projectCreationUncertain&&!desktop.surface.openingProject,'lost creation reply did not become uncertain');
    await until(()=>[...document.querySelectorAll<HTMLButtonElement>('button')].some(e=>shown(e)&&e.textContent==="保存先を開く…"),'unknown creation report did not reach the rendered dialog');
    assert(projectButton("キャンセル").disabled&&projectButton("プロジェクトを作成").disabled&&document.querySelector<HTMLElement>('#workbench')!.inert,'unknown result allowed retry/cancel or exposed stale editing');
    button("保存先を開く…").click();await until(()=>!desktop.surface.projectCreation&&desktop.surface.inventory?.project.name==='New Project'&&!desktop.surface.openingProject,'explicit fresh Open did not resolve the creation outcome');
    newRoot=desktop.surface.inventory!.root;
    assert(desktop.surface.inventory!.sources.length===0&&!desktop.surface.status.dirty.length,'Project creation fabricated data');
    checks.push('unknown-creation-reply-explicit-open');

    await until(()=>!button("テーブルを作成").disabled,'empty Project lacks guided Table action');button("テーブルを作成").click();
    await readyArtifact();button("作成内容の詳細").click();
    await until(()=>[...document.querySelectorAll<HTMLLabelElement>('.ant-modal label')].some(e=>shown(e)&&e.textContent==="空のインラインレコード"),'inline storage choice missing');
    [...document.querySelectorAll<HTMLLabelElement>('.ant-modal label')].find(e=>shown(e)&&e.textContent==="空のインラインレコード")!.click();
    await until(()=>!button("作成").disabled,'schema-only preview not ready');button("作成").click();
    await until(()=>!!desktop.surface.projection&&!desktop.surface.pending&&!document.querySelector('.creation-inline'),'guided Table did not open');
    assert(Number(desktop.surface.inventory!.sources.length)===1&&desktop.surface.projection!.totalRows===0&&!desktop.surface.projection!.canAdd,'schema creation invented a record destination');
    const table=desktop.surface.inventory!.logicalTables[0];assert(table&&table.name===desktop.surface.projection!.table.name,'logical Table identity was derived from filename');
    checks.push('guided-schema-explicit-no-record-source');

    button("ソースを作成").click();await until(()=>[...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].some(e=>shown(e)&&e.textContent==="データ"),'Data menu missing');
    [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find(e=>shown(e)&&e.textContent==="データ")!.click();
    await readyArtifact();key(input("新しいソースのファイル名"),'Enter');
    await until(()=>desktop.surface.inventory!.sources.length===2&&desktop.surface.projection!.canAdd&&!desktop.surface.pending,'explicit Data creation did not bind to Table');
    const source=desktop.surface.projection!.source!;await desktop.addRow();
    await until(()=>desktop.surface.status.dirty.includes(source),'Add Row did not create its source-local draft');
    await findTarget(table.name,`テーブル · ${table.name} — ${table.source}`);
    await until(()=>desktop.surface.target===table.source&&!desktop.surface.pending,'logical Table navigation failed');
    assert(desktop.surface.projection!.source===source&&desktop.surface.projection!.rows.some(row=>row.added),'logical navigation did not share the physical draft');
    checks.push('explicit-data-logical-navigation-shared-draft');

    const recent=desktop.surface.recentProjects;assert(recent.length===2&&recent[0].root===newRoot&&recent[1].root===oldRoot,'recents were not canonical-root deduplicated in successful order');
    checks.push('recent-canonical-root-order-dedup');
  }catch(error){await invoke('evidence_write',{report:{error:String(error),checks,startup,oldRoot,newRoot,recentProjects:desktop.surface.recentProjects,visibleDialogs:[...document.querySelectorAll<HTMLElement>('[role="dialog"]')].filter(shown).map(e=>e.textContent?.slice(0,600)),focus:document.activeElement?.getAttribute('aria-label')}});return;}
  await invoke('evidence_write',{report:{checks,oldRoot,newRoot,visibility:document.visibilityState,focused:document.hasFocus(),startup}});
}
