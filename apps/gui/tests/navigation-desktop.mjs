// Real Tauri/WebKit navigation evidence, using navigation_performance's input.
import fs from 'node:fs';
const [binary, project, output] = process.argv.slice(2);
const url = process.env.WEBDRIVER_URL ?? 'http://127.0.0.1:4444';
let session;
async function request(method, endpoint, body) {
  const response = await fetch(url + endpoint, {method, headers:{'content-type':'application/json'},body: body===undefined ? undefined : JSON.stringify(body)});
  const payload = await response.json(); if (!response.ok || payload.value?.error) throw new Error(JSON.stringify(payload)); return payload.value;
}
const execute = (script,args=[]) => request('POST',`/session/${session}/execute/sync`,{script,args});
async function waitFor(script, timeout=60000) {
  const end=Date.now()+timeout;
  while(Date.now()<end) { const result=await execute(script); if(result) return result; await new Promise(resolve=>setTimeout(resolve,40)); }
  throw new Error('navigation condition timed out: '+script);
}
async function element(xpath) { return (await request('POST',`/session/${session}/element`,{using:'xpath',value:xpath}))['element-6066-11e4-a52e-4f735466cecf']; }
async function clickPath(path) { const id=await element(`//*[@data-tree-path='${path}']`); await request('POST',`/session/${session}/element/${id}/click`,{}); }
const evidence={candidate:process.env.CANDIDATE_SHA,project,platform:process.platform,samples:[]};
try {
  const started=Date.now();
  session=(await request('POST','/session',{capabilities:{alwaysMatch:{'tauri:options':{application:binary}}}})).sessionId;
  await waitFor("return document.querySelector('.editor-area [role=gridcell]') && document.querySelector('.unified-column-header')");
  evidence.coldProjectMs=Date.now()-started;
  await execute(`window.__navigationTrace=[]; window.__navigationHeartbeat={timer:[],frames:[]};
    let timer=performance.now(), frame=timer;
    window.__navigationTimer=setInterval(()=>{const now=performance.now();window.__navigationHeartbeat.timer.push(now-timer);timer=now},50);
    function paint(now){window.__navigationHeartbeat.frames.push(now-frame);frame=now;window.__navigationFrame=requestAnimationFrame(paint)}
    window.__navigationFrame=requestAnimationFrame(paint);`);
  for (const [label,path] of [['B-first-source','sources/a-2.yaml'],['C-revisit','sources/a-1.yaml'],['D-same-table','sources/a-2.yaml'],['E-cross-table','sources/b-schema.yaml'],['F-schema-selection','sources/c-schema.yaml'],['C-cross-table-revisit','sources/a-1.yaml']]) {
    await execute('window.__navigationTrace=[]; window.__navigationHeartbeat.timer=[];window.__navigationHeartbeat.frames=[]');
    const started=Date.now(); await clickPath(path);
    const expected=path==='sources/c-schema.yaml'?'sources/c-1.yaml':path;
    await waitFor(`return document.querySelector('.editor-area[data-active-source="${expected}"] [role=gridcell]') && document.querySelector('.unified-column-header') && !document.querySelector('.editor-area .placeholder-editor')`);
    const gridMs=Date.now()-started;
    const id=await element("//section[contains(@class,'editor-area')]//*[@role='gridcell'][1]");
    await request('POST',`/session/${session}/element/${id}/click`,{});
    await request('POST',`/session/${session}/element/${id}/value`,{text:'\uE014',value:['\uE014']});
    const metrics=await execute(`return {trace:window.__navigationTrace, timerGapMs:Math.max(0,...window.__navigationHeartbeat.timer),frameGapMs:Math.max(0,...window.__navigationHeartbeat.frames),active:document.querySelector('.editor-area').dataset.activeSource,selected:document.querySelector('[data-tree-path][aria-selected=true]')?.dataset.treePath,focused:document.activeElement?.getAttribute('role'),rows:document.querySelectorAll('tr[data-grid-row-index]').length}`);
    if(metrics.active!==expected || metrics.selected!==path || metrics.rows>100 || metrics.focused!=='gridcell') throw new Error('navigation identity/focus/bounded rendering failed: '+JSON.stringify(metrics));
    evidence.samples.push({label,path,gridMs,usableMs:Date.now()-started,...metrics});
  }
  // Dirty navigation must stay responsive while Core validates the local overlay.
  const cell=await element("//section[contains(@class,'editor-area')]//*[@role='gridcell'][1]");
  await request('POST',`/session/${session}/element/${cell}/value`,{text:'\uE007',value:['\uE007']});
  const input=await element("//input[@aria-label='record 1 id']");
  await request('POST',`/session/${session}/element/${input}/value`,{text:'\uE009a\uE000',value:['\uE009','a','\uE000']});
  await request('POST',`/session/${session}/element/${input}/value`,{text:'999999',value:[...'999999']});
  await request('POST',`/session/${session}/element/${input}/value`,{text:'\uE007',value:['\uE007']});
  await waitFor("return document.querySelector('[data-tree-path=\"sources/a-1.yaml\"]').getAttribute('aria-label').includes('unsaved')");
  await execute(`window.__navigationTrace=[];window.__navigationHeartbeat.timer=[];window.__navigationHeartbeat.frames=[];window.__rapidComplete=false;
    const paths=['sources/a-2.yaml','sources/b-schema.yaml','sources/c-2.yaml','sources/a-1.yaml'];let index=0;
    function step(){document.querySelector('[data-tree-path="'+paths[index%4]+'"]').click();if(++index<40)setTimeout(step,10);else window.__rapidComplete=true}step();`);
  await waitFor(`return window.__rapidComplete && document.querySelector('.editor-area[data-active-source="sources/a-1.yaml"] [role=gridcell]')?.textContent.includes('999999') && document.querySelector('[data-tree-path="sources/a-1.yaml"][aria-selected=true]')`);
  // Wait for all requests to settle: obsolete completion must not roll selection back.
  await waitFor("return window.__navigationTrace.filter(x=>x.phase==='request-start').length===window.__navigationTrace.filter(x=>x.phase==='ipc-return').length",120000);
  evidence.rapid=await execute("return {selections:40,active:document.querySelector('.editor-area').dataset.activeSource,dirty:document.querySelector('[data-tree-path=\"sources/a-1.yaml\"]').getAttribute('aria-label'),trace:window.__navigationTrace,timerGapMs:Math.max(0,...window.__navigationHeartbeat.timer),frameGapMs:Math.max(0,...window.__navigationHeartbeat.frames)}");
  if(evidence.rapid.active!=='sources/a-1.yaml' || !evidence.rapid.dirty.includes('unsaved')) throw new Error('rapid navigation lost target or dirty buffer');
  const reads=evidence.rapid.trace.filter(event=>event.phase==='request-start').length;
  if(reads>40) throw new Error('rapid selections caused duplicate read requests');
  await execute('clearInterval(window.__navigationTimer);cancelAnimationFrame(window.__navigationFrame)');
  console.log(JSON.stringify(evidence,null,2));
} finally {
  fs.writeFileSync(output,JSON.stringify(evidence,null,2));
  if(session) await request('DELETE',`/session/${session}`).catch(()=>{});
}
