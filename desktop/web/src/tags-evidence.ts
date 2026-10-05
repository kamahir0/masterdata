import { invoke } from "@tauri-apps/api/core";
import { desktop } from "./workspace";
const SOURCE = "sources/catalog-data.yaml";
const frame = () => new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
function assert(test: unknown, message: string): asserts test { if(!test) throw new Error(message); }
async function until(test: () => boolean, message: string) {
  const end = performance.now() + 5000;
  while(!test()) { if(performance.now() >= end) throw new Error(message); await frame(); }
}
function find(selector: string) {
  const node = document.querySelector<HTMLElement>(selector);
  assert(node, `missing ${selector}`); return node;
}
function key(element: HTMLElement, key: string, extra: KeyboardEventInit = {}) {
  element.dispatchEvent(new KeyboardEvent("keydown", {key, bubbles: true, ...extra}));
}
function text(input: HTMLInputElement, value: string) {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
  input.dispatchEvent(new Event("input", {bubbles: true}));
}
export async function run({startup}: {startup: Record<string, unknown>}) {
  const checks: string[] = [];
  const bytes = async () => (await desktop.rpc<{before: string; after: string}>({kind: "compare", source: SOURCE})).data;
  const ready = async () => {
    await until(() => !desktop.surface.busy && !desktop.surface.pending && !!document.querySelector('.tag-panel input[aria-label="New Tag"]:not([readonly])'), "Tag editor did not settle");
    assert(!desktop.surface.error, desktop.surface.error ?? "Tag operation failed");
  };
  const changed = async (revision: number) => {
    await until(() => desktop.surface.projection?.revision !== revision, "Tag operation did not commit"); await ready();
  };
  const open = async (row = 0) => {
    desktop.viewport!.focus();
    desktop.setSelection(row, 0);
    key(desktop.viewport!, "F10", {shiftKey: true});
    await until(() => !![...document.querySelectorAll<HTMLElement>('.ant-dropdown-menu-item')].find(item => item.textContent === "Tags…"), "keyboard row menu lacks Tags");
    [...document.querySelectorAll<HTMLElement>('.ant-dropdown-menu-item')].find(item => item.textContent === "Tags…")!.click();
    await ready();
  };
  const close = async () => { key(find('.tag-panel input'), "Escape"); await until(() => !desktop.interaction.tags, "Tag editor did not close"); await frame(); };
  const undo = async (redo = false) => {
    const revision = desktop.surface.projection!.revision;
    await desktop.undo(redo);
    await until(() => desktop.surface.projection?.revision !== revision && !desktop.surface.busy, "Tag history did not advance");
  };
  try {
    await until(() => document.hasFocus(), "native window was not focused");
    await desktop.selectTarget(SOURCE, "tags", false);
    const base = await bytes();
    await open();
    assert((await bytes()).after === base.after && !desktop.surface.projection!.dirty, "opening Tags dirtied source");
    assert(!desktop.surface.projection!.columns.some(column => column.field.name === "$tags"), "Tags became a domain column");
    assert(document.activeElement?.closest('.tag-panel'), "Tag editor lacks keyboard focus");
    const descriptor = await desktop.tagView(desktop.interaction.tags!);
    assert(descriptor?.known.includes("development") && descriptor.known.includes("production"), "shared loaded/profile Tag candidates missing");
    checks.push("keyboard-context-open-clean-known-tags");
    let input = find('input[aria-label="New Tag"]') as HTMLInputElement;
    const revision = desktop.surface.projection!.revision;
    input.focus(); input.dispatchEvent(new CompositionEvent("compositionstart", {bubbles: true}));
    text(input, "日本語"); key(input, "Enter", {isComposing: true}); await frame();
    assert((await bytes()).after === base.after, "Tag IME composition Enter committed");
    input.dispatchEvent(new CompositionEvent("compositionend", {bubbles: true}));
    text(input, " Debug "); key(input, "Enter"); await changed(revision);
    assert((await bytes()).after === base.after.replace("$tags: [development]", '$tags: [development, " Debug "]'), "Tag add trimmed or changed non-target bytes");
    assert(find('[data-tag-index="1"] button[aria-invalid="true"]'), "invalid lexical Tag lacks feedback");
    checks.push("composition-and-lossless-invalid-tag");
    const invalid = (await bytes()).after;
    await close(); assert((await bytes()).after === invalid, "closing rolled back a committed Tag");
    await undo(); assert((await bytes()).after === base.after && !desktop.surface.projection!.dirty, "Tag Add is not one Undo");
    await undo(true); assert((await bytes()).after === invalid, "Redo lost exact Tag spelling");
    checks.push("close-retains-one-undo-redo");
    await open();
    find('button[aria-label="Edit Tag 2"]').click();
    await until(() => !!document.querySelector('input[aria-label="Tag 2"]'), "Tag replace editor missing");
    input = find('input[aria-label="Tag 2"]') as HTMLInputElement;
    const replaceRevision = desktop.surface.projection!.revision;
    text(input, "development"); key(input, "Enter"); await changed(replaceRevision);
    assert(document.querySelectorAll('.tag-entry-value[aria-invalid="true"]').length === 2, "duplicate Tags were deduplicated or not diagnosed");
    const addRevision = desktop.surface.projection!.revision;
    find('button[aria-label="Add Tag"]').click(); await changed(addRevision);
    assert(find('[data-tag-index="2"] .tag-entry-value').textContent?.includes("空のTag"), "explicit empty Tag was discarded");
    checks.push("replace-duplicate-and-explicit-empty");
    await close();
    await until(() => !desktop.surface.status.diagnosticsPending, "Tag diagnostics incomplete");
    const diagnostics = (await desktop.rpc<{problems: {code: string; source: string; fieldPath: string[]}[]}>({kind: "problems", start: 0, count: 256})).data.problems;
    const problem = diagnostics.find(problem => problem.code === "E-RECORD-TAGS" && problem.source === SOURCE);
    assert(problem?.fieldPath[0] === "$tags", "shared Tag diagnostic missing");
    find('#problems-bar button').click();
    await until(() => !![...document.querySelectorAll<HTMLButtonElement>('.problem-item')].find(item => item.textContent?.includes("E-RECORD-TAGS")), "Tag Problem missing");
    [...document.querySelectorAll<HTMLButtonElement>('.problem-item')].find(item => item.textContent?.includes("E-RECORD-TAGS"))!.click();
    await ready(); assert(desktop.interaction.tags?.row === desktop.surface.projection!.rows[0].id, "Problems opened the wrong Tag occurrence");
    checks.push("problems-resolves-tag-editor");
    for(const index of [2, 1]) { const before = desktop.surface.projection!.revision; find(`button[aria-label="Remove Tag ${index + 1}"]`).click(); await changed(before); }
    await close();
    for(let i = 0; i < 5; i++) await undo();
    assert((await bytes()).after === base.after, "inverse Tag history failed exact restoration");
    const p = desktop.surface.projection!;
    await desktop.addRow(null, p);
    const added = desktop.surface.projection!.rows.find(row => row.added)!;
    assert(added, "Added Row missing");
    await open(added.viewIndex);
    input = find('input[aria-label="New Tag"]') as HTMLInputElement;
    const addedRevision = desktop.surface.projection!.revision;
    text(input, "draft-only"); key(input, "Enter"); await changed(addedRevision);
    await close();
    const deleteRevision = desktop.surface.projection!.revision;
    await desktop.rowAction("delete", desktop.surface.projection!, added.id);
    await until(() => desktop.surface.projection!.revision !== deleteRevision && !desktop.surface.busy, "Added Row deletion failed");
    assert((await bytes()).after === base.after, "Added Tag cancellation did not restore bytes");
    checks.push("added-row-tags-delete-composition");
    await invoke("evidence_write", {report: {format: 1, kind: "tags", checks, startup, source: await bytes(),
      visibility: document.visibilityState, focused: document.hasFocus(), mountedRows: document.querySelectorAll('.grid-row').length}});
  } catch(error) {
    await invoke("evidence_write", {report: {format: 1, kind: "tags", error: String(error), checks, startup,
      surfaceError: desktop.surface.error, interaction: desktop.interaction,
      active: document.activeElement?.outerHTML.slice(0, 2000), dom: document.querySelector('.tag-panel')?.outerHTML.slice(0, 8000),
      visibility: document.visibilityState, focused: document.hasFocus()}});
  }
}
