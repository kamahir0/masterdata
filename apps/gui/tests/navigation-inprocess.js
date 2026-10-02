// Legacy adapter: synthetic DOM input in a real native Tauri WebView.
// rAF is a paint opportunity, never proof of completed GPU presentation.
// Standalone build: tauri build --features navigation-evidence,tauri/custom-protocol
// --bundles app --config tests/navigation-tauri-config.json (from apps/gui).
// Launch the disposable bundle with MASTERDATA_PROJECT_PATH and
// MASTERDATA_NAVIGATION_EVIDENCE_OUTPUT; dirty runs also set MODE=dirty-rapid
// under the MASTERDATA_NAVIGATION_EVIDENCE_ prefix. Keep the window foreground.
void (async () => {
  if (window.__navigationEvidenceStarted) return;
  window.__navigationEvidenceStarted = true;
  window.__navigationTrace = [];
  const evidence = { method: 'real-tauri-webview-synthetic-dom-input', userAgent: navigator.userAgent, samples: [], runs: 3, samplesPerCasePerRun: 100 };
  const checkpoint = () => window.__TAURI_INTERNALS__.invoke('plugin:navigation-evidence|report', { report:evidence, done:false });
  evidence.phase = 'injected';
  void checkpoint();
  const frame = () => new Promise((resolve,reject) => {
    const timer = setTimeout(() => reject(Error('paint opportunity unavailable for 5s; check foreground/occlusion')),5000);
    requestAnimationFrame(() => {clearTimeout(timer);resolve();});
  });
  const wait = async predicate => {
    const deadline = performance.now() + 30000;
    while (!predicate()) { if (performance.now() > deadline) throw Error(`condition timeout: ${evidence.phase}, active=${active()}, body=${document.body.textContent.slice(0,200)}`); await Promise.race([frame(), new Promise(resolve => setTimeout(resolve,100))]); }
  };
  const active = () => document.querySelector('.editor-area')?.dataset.activeSource;
  const grid = path => active() === path && document.querySelector('.editor-area [role=gridcell]') && !document.querySelector('.editor-area .placeholder-editor');
  const select = async (path, expected = path) => {
    window.__navigationTrace = [];
    const start = performance.now();
    const item = document.querySelector(`[data-tree-path="${path}"]`);
    if (!item) throw Error(`missing source ${path}`);
    item.click();
    await wait(() => document.querySelector(`[data-tree-path="${path}"][aria-selected=true]`));
    const selectedMs = performance.now() - start;
    await wait(() => grid(expected));
    await frame();
    const paintOpportunityMs = performance.now() - start;
    const cell = document.querySelector('.editor-area [role=gridcell]');
    cell.click();
    await frame();
    cell.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }));
    await wait(() => document.activeElement?.getAttribute('role') === 'gridcell' && document.activeElement !== cell);
    const acceptedInteractionMs = performance.now() - start;
    if (active() !== expected) throw Error('stale active target');
    const rows = document.querySelectorAll('tr[data-grid-row-index]').length;
    if (rows > 100) throw Error('unbounded mounted rows');
    return { path, expected, selectedMs, paintOpportunityMs, acceptedInteractionMs, rows, trace: [...window.__navigationTrace] };
  };
  try {
    const boot = performance.now();
    evidence.phase = 'waiting-for-initial-grid';
    await wait(() => document.querySelector('.editor-area [role=gridcell]'));
    evidence.coldFromHarnessInjectionMs = performance.now() - boot;
    evidence.phase = 'first-projection';
    void checkpoint();
    evidence.firstProjection = await select('sources/a-2.yaml');
    if (window.__navigationEvidenceMode === 'dirty-rapid') {
      evidence.phase = 'create-dirty-overlay';
      await select('sources/a-1.yaml');
      const cell = document.querySelector('.editor-area [role=gridcell]');
      const original = cell.textContent;
      cell.focus();
      cell.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));
      await wait(() => document.querySelector('input[aria-label="record 1 id"]'));
      const input = document.querySelector('input[aria-label="record 1 id"]');
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(input,'999999');
      input.dispatchEvent(new Event('input',{bubbles:true}));
      await frame();
      input.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));
      await wait(() => document.querySelector('[data-tree-path="sources/a-1.yaml"]').getAttribute('aria-label').includes('unsaved'));
      evidence.phase = 'dirty-revisit';
      for (let run=1;run<=3;run++) for (let index=0;index<100;index++) {
        await select('sources/a-2.yaml');
        const sample = await select('sources/a-1.yaml');
        if (!document.querySelector('.editor-area [role=gridcell]').textContent.includes('999999')) throw Error('dirty overlay lost');
        evidence.samples.push({run,case:'dirty-revisit',...sample});
        if (index===99) await checkpoint();
      }
      evidence.phase = 'rapid';
      window.__navigationTrace=[];
      const paths=['sources/a-2.yaml','sources/b-schema.yaml','sources/c-2.yaml','sources/a-1.yaml'];
      for(let index=0;index<50;index++) {
        const path=index===49 ? 'sources/a-1.yaml' : paths[index%4];
        document.querySelector(`[data-tree-path="${path}"]`).click();
        await new Promise(resolve => setTimeout(resolve,10));
      }
      await wait(() => grid('sources/a-1.yaml') && document.querySelector('.editor-area [role=gridcell]').textContent.includes('999999'));
      await wait(() => window.__navigationTrace.filter(x=>x.phase==='request-start').length===window.__navigationTrace.filter(x=>x.phase==='ipc-return').length);
      evidence.rapid={selections:50,active:active(),trace:[...window.__navigationTrace],dirtyRetained:true};
      const undoCell=document.querySelector('.editor-area [role=gridcell]');undoCell.focus();
      undoCell.dispatchEvent(new KeyboardEvent('keydown',{key:'z',metaKey:true,bubbles:true}));
      await wait(() => document.querySelector('.editor-area [role=gridcell]').textContent===original);
      evidence.rapid.historyRetained=true;
    } else for (let run = 1; run <= 3; run++) {
      for (const [name, path, alternate, expected] of [
        ['revisit', 'sources/a-1.yaml', 'sources/c-2.yaml', 'sources/a-1.yaml'],
        ['same-table', 'sources/a-2.yaml', 'sources/a-1.yaml', 'sources/a-2.yaml'],
        ['cross-table', 'sources/b-schema.yaml', 'sources/a-1.yaml', 'sources/b-schema.yaml'],
        ['schema-selection', 'sources/c-schema.yaml', 'sources/a-1.yaml', 'sources/c-1.yaml'],
      ]) {
        for (let index = 0; index < 100; index++) {
          await select(alternate);
          evidence.samples.push({ run, case: name, ...await select(path, expected) });
        }
        await checkpoint();
      }
    }
    // Report distributions per run; a single first projection is not p95.
    const percentile = (values, fraction) => [...values].sort((a,b) => a-b)[Math.ceil(values.length*fraction)-1];
    evidence.distributions = [];
    for (let run = 1; run <= 3; run++) for (const name of ['revisit','same-table','cross-table','schema-selection','dirty-revisit']) {
      const samples = evidence.samples.filter(x => x.run===run && x.case===name);
      if (!samples.length) continue;
      for (const boundary of ['selectedMs','paintOpportunityMs','acceptedInteractionMs']) {
        const values = samples.map(x => x[boundary]);
        evidence.distributions.push({ run, case:name, boundary, count:values.length, median:percentile(values,.5), p95:percentile(values,.95), max:Math.max(...values) });
      }
    }
  } catch (error) { evidence.error = String(error); }
  await window.__TAURI_INTERNALS__.invoke('plugin:navigation-evidence|report', { report:evidence, done:true });
})();
