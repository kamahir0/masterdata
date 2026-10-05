import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { desktop } from "./workspace";

const SOURCE = "sources/catalog-data.yaml";
const frame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
async function until(test: () => boolean, message: string) {
  const deadline = performance.now() + 8000;
  while (!test()) {
    if (performance.now() > deadline) throw new Error(message);
    await frame();
  }
}
function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}
function text(input: HTMLInputElement, value: string) {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}
const name = () => desktop.surface.projection?.rows[0]?.cells.find((_cell, i) => desktop.surface.projection?.columns[i].field.name === "name")?.display;
async function phase(phase: string) {
  await invoke("evidence_write", { report: { phase } });
}
export async function run({ startup }: { startup: Record<string, unknown> }) {
  const checks: string[] = [];
  try {
    await until(() => !!desktop.surface.inventory, "Project not open");
    await desktop.selectTarget(SOURCE, "external-setup", false);
    const gridTop = desktop.viewport!.getBoundingClientRect().top;
    await phase("clean");
    await until(() => name() === "Outside clean" && !desktop.surface.pending, "clean source did not automatically refresh");
    assert(!desktop.surface.projection!.dirty, "clean external refresh created dirty state");
    checks.push("clean-auto-refresh");

    desktop.setSelection(0, desktop.surface.projection!.columns.findIndex(c => c.field.name === "name"));
    desktop.beginEditor();
    await until(() => !!document.querySelector(".active-cell-editor input"), "scalar input missing");
    const input = document.querySelector<HTMLInputElement>(".active-cell-editor input")!;
    input.focus();
    text(input, "unfinished input");
    const version = desktop.surface.status.externalVersion;
    await phase("unrelated");
    await until(() => desktop.surface.status.externalVersion > version && !desktop.surface.pending, "unrelated source refresh missing");
    assert(document.querySelector<HTMLInputElement>(".active-cell-editor input")?.value === "unfinished input", "background refresh lost typing");
    assert(!desktop.surface.projection!.dirty, "background refresh committed typing");
    input.dispatchEvent(new KeyboardEvent("keydown", {key: "Enter", bubbles: true}));
    await until(() => desktop.surface.projection?.dirty === true && !desktop.surface.busy, "unchanged authoring context rejected input");
    assert(name() === "unfinished input", "observed input was not committed exactly");
    checks.push("unrelated-change-preserves-input");

    await phase("dirty");
    await until(() => desktop.surface.projection?.conflict === true && !desktop.surface.pending, "dirty source did not become Conflict");
    assert(name() === "unfinished input" && desktop.surface.projection!.canUndo, "Conflict discarded local draft/history");
    checks.push("dirty-conflict");
    desktop.setSelection(0, desktop.surface.projection!.columns.findIndex(c => c.field.name === "name"));
    desktop.beginEditor();
    await until(() => !!document.querySelector(".active-cell-editor input"), "temporary input missing");
    text(document.querySelector<HTMLInputElement>(".active-cell-editor input")!, "18446744073709551615 · 未確定");
    await phase("invalid");
    await until(() => !desktop.surface.pending && !desktop.surface.projection && !!desktop.surface.error && desktop.surface.heldInputs.length === 1, "invalid external source remained editable or lost input");
    assert(desktop.surface.status.dirty.includes(SOURCE), "invalid source discarded physical draft");
    assert(desktop.surface.heldInputs[0].text === "18446744073709551615 · 未確定", "changed context lost exact temporary text");
    assert(!desktop.interaction.editor, "obsolete input remained a current editor");
    checks.push("changed-context-holds-input");
    await phase("restore");
    await until(() => !!desktop.surface.projection && !desktop.surface.pending, "repaired source did not return");
    assert(name() === "unfinished input" && desktop.surface.projection!.canUndo, "repair lost draft/history");
    assert(desktop.viewport!.getBoundingClientRect().top === gridTop, "external feedback shifted the grid");
    checks.push("invalid-and-repair-preserve-draft");

    await phase("delete-schema");
    await until(() => !desktop.surface.pending && !desktop.surface.projection && !!desktop.surface.error, "missing dependency remained current");
    await phase("restore-schema");
    await until(() => !!desktop.surface.projection && !desktop.surface.pending, "dependency repair did not return");
    assert(name() === "unfinished input", "dependency repair changed local values");
    checks.push("missing-dependency-refresh");
    await getCurrentWindow().close();
    await until(() => !!desktop.surface.choice, "native window close bypassed authoring guard");
    const cancelButton = () => [...document.querySelectorAll<HTMLButtonElement>(".ant-modal button")].find(button => button.textContent === "Cancel");
    await until(() => !!cancelButton(), "close guard did not render");
    const cancel = cancelButton();
    assert(cancel, "close guard Cancel missing");
    cancel.click();
    await until(() => !desktop.surface.choice, "window guard did not cancel");
    assert(await getCurrentWindow().isVisible(), "Cancel closed native window");
    assert(desktop.surface.heldInputs[0].text === "18446744073709551615 · 未確定", "close guard lost held input");
    checks.push("native-close-cancel-preserves-input");
    assert((startup.browserErrors as string[])?.length === 0, "browser event errors");
    await invoke("evidence_write", { report: {kind: "external", checks, startup, visibility: document.visibilityState, focused: document.hasFocus(), mountedRows: document.querySelectorAll(".grid-row").length, dirty: desktop.surface.status.dirty, conflict: desktop.surface.projection!.conflict} });
  } catch (error) {
    await invoke("evidence_write", {report: {kind: "external", error: String(error), checks, startup, status: desktop.surface.status, surfaceError: desktop.surface.error, projection: desktop.surface.projection, interaction: desktop.interaction, active: document.activeElement?.outerHTML.slice(0, 1500)}});
  }
}
