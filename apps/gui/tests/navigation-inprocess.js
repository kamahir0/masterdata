// Legacy adapter: synthetic DOM input in a real native Tauri WebView.
// rAF is a paint opportunity, never proof of completed GPU presentation.
void (async () => {
  if (window.__navigationEvidenceStarted) return;
  window.__navigationEvidenceStarted = true;
  window.__navigationTrace = [];
  const evidence = { method: 'real-tauri-webview-synthetic-dom-input', userAgent: navigator.userAgent, samples: [], runs: 3, samplesPerCasePerRun: 100 };
  const checkpoint = () => window.__TAURI_INTERNALS__.invoke('plugin:navigation-evidence|report', { report:evidence, done:false });
  evidence.phase = 'injected';
  void checkpoint();
  const frame = () => new Promise(resolve => requestAnimationFrame(resolve));
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
    for (let run = 1; run <= 3; run++) {
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
    for (let run = 1; run <= 3; run++) for (const name of ['revisit','same-table','cross-table','schema-selection']) {
      const samples = evidence.samples.filter(x => x.run===run && x.case===name);
      for (const boundary of ['selectedMs','paintOpportunityMs','acceptedInteractionMs']) {
        const values = samples.map(x => x[boundary]);
        evidence.distributions.push({ run, case:name, boundary, count:values.length, median:percentile(values,.5), p95:percentile(values,.95), max:Math.max(...values) });
      }
    }
  } catch (error) { evidence.error = String(error); }
  await window.__TAURI_INTERNALS__.invoke('plugin:navigation-evidence|report', { report:evidence, done:true });
})();
