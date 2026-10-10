import {invoke} from "@tauri-apps/api/core";
import {desktop,GRID} from "./workspace";
const SOURCE='sources/a-1.yaml';
const frame=()=>new Promise<void>(resolve=>requestAnimationFrame(()=>resolve()));
async function until(test:()=>boolean,message:string){const deadline=performance.now()+8000;while(!test()){if(performance.now()>deadline)throw new Error(message);await frame();}}
function assert(test:unknown,message:string):asserts test{if(!test)throw new Error(message);}
function opaque(element:HTMLElement){
  const style=getComputedStyle(element),context=document.createElement('canvas').getContext('2d')!;
  context.fillStyle=style.backgroundColor;context.fillRect(0,0,1,1);
  assert(style.opacity==='1'&&context.getImageData(0,0,1,1).data[3]===255,
    `sticky ${element.getAttribute('role')} has a translucent background: ${style.backgroundColor}`);
}
function unobstructed(element:HTMLElement,role:string){
  opaque(element);
  const rect=element.getBoundingClientRect();
  for(const x of [.2,.5,.8])for(const y of [.2,.5,.8]) {
    const found=document.elementFromPoint(rect.left+rect.width*x,rect.top+rect.height*y);
    assert(found?.closest(`[role="${role}"]`)===element,`sticky ${role} at ${rect.left+rect.width*x},${rect.top+rect.height*y} obstructed by ${found?.outerHTML.slice(0,300)}`);
  }
}
export async function run({startup}:{startup:Record<string,unknown>}){
    const checks:string[]=[];
  try {
    await until(()=>!!desktop.surface.inventory&&document.hasFocus(),'Project / foreground unavailable');
    await desktop.selectTarget(SOURCE,'geometry-setup',false);
    const viewport=desktop.viewport!,header=viewport.querySelector<HTMLElement>('#grid-header')!,corner=header.querySelector<HTMLElement>('[role=columnheader]')!;
    const width=header.querySelector<HTMLElement>('[data-field="id"]')!.getBoundingClientRect().width;
    const grip=header.querySelector<HTMLElement>("[aria-label=\"列を並べ替え id\"]")!.getBoundingClientRect();
    const name=header.querySelector<HTMLElement>('[aria-label="名前を変更: id"]')!.getBoundingClientRect();
    const action=header.querySelector<HTMLElement>("[aria-label=\"id の操作\"]")!.getBoundingClientRect();
    assert(grip.right<=name.left&&name.right<=action.left,'field name, handle and action hit areas overlap');
    checks.push('header-name-handle-action-separate');
    for(const theme of ['light','dark'] as const){
      await desktop.setTheme(theme);
      await until(()=>document.documentElement.dataset.theme===theme,'theme did not reach the native surface');
      for(const fraction of [.25,.5,.75,2.3]){
        viewport.scrollLeft=width*fraction;
        unobstructed(corner,'columnheader');
      }
    }
    checks.push('sticky-corner-occludes-scrolled-controls');
    viewport.scrollLeft=0;desktop.setSelection(0,0);desktop.beginEditor();
    await until(()=>!!document.querySelector('.active-cell-editor input'),'single active editor missing');
    // The single overlay can mount before the virtualized display column has
    // returned from the previous horizontal scroll. Compare after both exist.
    await until(()=>!!document.getElementById('cell-0-0'),'editing display cell did not return after horizontal scroll');
    const input=document.querySelector<HTMLInputElement>('.active-cell-editor input')!;
    const valueStyle=getComputedStyle(document.getElementById('cell-0-0')!),editorStyle=getComputedStyle(input);
    assert(valueStyle.fontSize===editorStyle.fontSize&&valueStyle.fontFamily===editorStyle.fontFamily,'inline editor changed typography');
    checks.push('inline-editor-matches-display-typography');
    viewport.scrollLeft=width*.6;
    const row=viewport.querySelector<HTMLElement>('[aria-rowindex="2"] [role="rowheader"]')!;
    unobstructed(row,'rowheader');
    assert(input===document.activeElement&&document.querySelectorAll('.active-cell-editor').length===1,'scroll lost or duplicated active input');
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value')!.set!.call(input,'123456');
    input.dispatchEvent(new Event('input',{bubbles:true}));
    const editor=desktop.interaction.editor;
    viewport.scrollLeft=0;viewport.scrollTop=100*GRID.row;
    await until(()=>!!document.getElementById('cell-100-0'),'scrolling with temporary input left a blank viewport');
    assert(document.activeElement===input&&input.value==='123456'&&desktop.interaction.editor===editor&&
      document.querySelectorAll('.active-cell-editor').length===1&&viewport.querySelectorAll('.grid-row').length<=64,
      'scrolling lost temporary input / occurrence / focus or exceeded bounded rendering');
    assert(!desktop.surface.status.dirty.length&&!desktop.surface.projection!.canUndo,'scrolling committed temporary input');
    viewport.scrollTop=0;
    await until(()=>!!document.getElementById('cell-0-0'),'returning to the editing occurrence failed');
    await frame();
    const cellRect=document.getElementById('cell-0-0')!.getBoundingClientRect(),inputRect=input.getBoundingClientRect();
    assert(Math.abs(cellRect.left-inputRect.left)<=2&&Math.abs(cellRect.top-inputRect.top)<=2&&document.activeElement===input,
      'active input did not follow its cell / preserve focus after scroll');
    input.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}));
    await until(()=>!document.querySelector('.active-cell-editor'),'Escape retained the editor');
    await frame();
    assert(!desktop.surface.status.dirty.length&&!desktop.surface.projection!.canUndo&&
      document.getElementById('cell-0-0')!.textContent==='2000','Escape / blur committed temporary input');
    checks.push('sticky-row-context-occludes-active-editor');
    checks.push('scroll-preserves-temporary-input-and-escape-discards');
    const menuButton=viewport.querySelector<HTMLButtonElement>('.row-actions')!;
    const menuVisible=()=>[...document.querySelectorAll<HTMLElement>('.grid-context-menu')].some(e=>e.getClientRects().length&&getComputedStyle(e).visibility!=='hidden');
    menuButton.click();await until(menuVisible,'row menu did not open');
    document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}));await until(()=>!menuVisible(),'Escape did not dismiss row menu');
    assert(document.activeElement===viewport,'Escape did not restore grid focus');
    menuButton.click();await until(menuVisible,'row menu did not reopen');
    menuButton.click();await until(()=>!menuVisible(),'second button click did not close menu');
    menuButton.click();await until(menuVisible,'row menu did not open for outside click');
    const search=document.querySelector<HTMLInputElement>('#search')!;search.focus();
    search.dispatchEvent(new PointerEvent('pointerdown',{bubbles:true,button:0}));await until(()=>!menuVisible(),'outside click did not dismiss row menu');
    assert(document.activeElement===search,'outside menu dismissal stole focus');
    checks.push('row-menu-dismiss-and-focus');
    const addColumn=document.querySelector<HTMLElement>('.add-column')!,addRow=document.querySelector<HTMLElement>('.add-row')!;
    const columnPosition=addColumn.getBoundingClientRect(),rowPosition=addRow.getBoundingClientRect();
    viewport.scrollLeft=viewport.scrollWidth-viewport.clientWidth;
    viewport.scrollTop=viewport.scrollHeight-viewport.clientHeight;
    await until(()=>!!document.getElementById('cell-1999-19'),'last row / column did not become visible');
    const view=viewport.getBoundingClientRect(),head=header.getBoundingClientRect();
    const columnScrolled=addColumn.getBoundingClientRect(),rowScrolled=addRow.getBoundingClientRect();
    assert(Math.abs(columnPosition.right-columnScrolled.right)<1&&Math.abs(columnPosition.top-columnScrolled.top)<1&&
      Math.abs(rowPosition.left-rowScrolled.left)<1&&Math.abs(rowPosition.bottom-rowScrolled.bottom)<1&&
      rowScrolled.left<view.left+view.width/2&&columnScrolled.right<=view.right&&rowScrolled.bottom<=view.bottom,'Add actions did not stay at viewport right / bottom-left');
    for(const action of [addColumn,addRow]){const r=action.getBoundingClientRect();assert(action.contains(document.elementFromPoint(r.left+r.width/2,r.top+r.height/2)),'viewport-fixed Add action obstructed');}
    checks.push('viewport-fixed-add-actions');
    const last=document.getElementById('cell-1999-19')!;
    const identity=last.closest('[role="row"]')!.querySelector<HTMLElement>('[role="rowheader"]')!;
    assert(Math.abs(head.top-view.top)<=1&&Math.abs(identity.getBoundingClientRect().left-view.left)<=1,'sticky context detached from viewport');
    unobstructed(corner,'columnheader');
    // Native overlay scrollbars can own the bottommost row's pointer area.
    // Probe a complete visible row above that chrome, while the exact last
    // occurrence / value and its sticky position are checked independently.
    const visible=[...viewport.querySelectorAll<HTMLElement>('[role="rowheader"]')].find(element=>{
      const rect=element.getBoundingClientRect();return rect.top>=head.bottom&&rect.bottom<=view.bottom-rect.height;
    });
    assert(visible,'no complete row context visible');unobstructed(visible,'rowheader');
    assert(last.textContent==='4018'&&viewport.querySelectorAll('.grid-row').length<=64,'last value / bounded projection mismatch');
    checks.push('long-wide-sticky-context-and-exact-value');
    const combo=header.querySelector<HTMLInputElement>('input[aria-label="field19 の型"]')!;
    assert(combo,'last field control missing');combo.focus();combo.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowDown',keyCode:40,which:40,bubbles:true}));
    await until(()=>!!document.querySelector('.ant-select-dropdown:not(.ant-select-dropdown-hidden) .ant-select-item-option'),'type popup not visible');
    const option=document.querySelector<HTMLElement>('.ant-select-dropdown:not(.ant-select-dropdown-hidden) .ant-select-item-option')!;
    // Portal mounting precedes placement / interactive motion. DOM existence
    // alone cannot prove that the option has reached its usable hit area.
    await until(()=>{
      const rect=option.getBoundingClientRect(),found=document.elementFromPoint(rect.left+rect.width/2,rect.top+rect.height/2);
      return found?.closest('.ant-select-dropdown')===option.closest('.ant-select-dropdown');
    },'sticky layer obstructed contextual popup');
    combo.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',keyCode:27,which:27,bubbles:true}));
    await until(()=>!document.querySelector('.ant-select-dropdown:not(.ant-select-dropdown-hidden)'),'Escape did not close popup');
    checks.push('contextual-popup-remains-interactive');
    assert(!desktop.surface.status.dirty.length,'geometry / cancel mutated source');
    await invoke('evidence_write',{report:{kind:'geometry',checks,startup,dirty:desktop.surface.status.dirty,mountedRows:viewport.querySelectorAll('.grid-row').length,
      visibility:document.visibilityState,focused:document.hasFocus()}});
  }catch(error){await invoke('evidence_write',{report:{kind:'geometry',error:String(error),checks,startup,dirty:desktop.surface.status.dirty,
    active:document.activeElement?.outerHTML.slice(0,500),scroll:{left:desktop.viewport?.scrollLeft,top:desktop.viewport?.scrollTop}}});}
}
