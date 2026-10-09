import { invoke } from "@tauri-apps/api/core";
import { desktop, GRID } from "./workspace";
import type { Reply } from "./types";

// GUI-PERF-005. Actual native WebView handlers; OS input and the independent
// all-candidate-cell/byte oracle are measured separately.
type Controls = Pick<typeof import("./main"), "startup" | "selectTarget" | "acceptedInteraction" | "samples">;
const frame = () => new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
function assert(value: unknown, message: string): asserts value {
  if (!value) throw new Error(message);
}
async function until(test: () => boolean, message: string) {
  const deadline = performance.now() + 15_000;
  while (!test()) {
    assert(performance.now() < deadline, message);
    await frame();
  }
}
function text(input: HTMLInputElement, value: string) {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

export async function run({ startup, selectTarget, acceptedInteraction, samples }: Controls) {
  const source = "sources/data00.yaml";
  const stages: Record<string, number> = {}, checks: string[] = [];
  const operations: { kind: string; input: number; ipcReturn: number; host: Reply<unknown>["host"] }[] = [];
  const frames: number[] = [];
  const bounds = { mountedRows: 0, mountedCells: 0, activeEditors: 0 };
  let monitoring = true, previousFrame: number | null = null;
  const monitor = (now: number) => {
    if (!monitoring) return;
    if (previousFrame !== null) frames.push(now - previousFrame);
    previousFrame = now;
    bounds.mountedRows = Math.max(bounds.mountedRows, document.querySelectorAll(".grid-row").length);
    bounds.mountedCells = Math.max(bounds.mountedCells, document.querySelectorAll('[role="gridcell"]').length);
    bounds.activeEditors = Math.max(bounds.activeEditors, document.querySelectorAll(".active-cell-editor input").length);
    requestAnimationFrame(monitor);
  };
  const request = desktop.rpc.bind(desktop);
  desktop.rpc = async <T>(intent: Record<string, unknown>): Promise<Reply<T>> => {
    const input = performance.now(), result = await request<T>(intent);
    if (["paste", "undo", "search"].includes(String(intent.kind)))
      operations.push({ kind: String(intent.kind), input, ipcReturn: performance.now(), host: result.host });
    return result;
  };
  const settle = async () => {
    await until(() => !desktop.surface.busy && !desktop.surface.pending && !desktop.surface.queryPending, "operation did not settle");
    assert(!desktop.surface.error, desktop.surface.error ?? "operation failed");
    await frame();
  };
  const scroll = async (row: number, column: number) => {
    const viewport = desktop.viewport!;
    viewport.scrollTop = row * GRID.row;
    viewport.scrollLeft = column * GRID.column;
    viewport.dispatchEvent(new Event("scroll"));
    await until(() => !!document.getElementById(`cell-${row}-${column}`), "long/wide target did not mount");
  };
  const projected = (row: number, column: number) => desktop.surface.projection?.rows.find(item => item.viewIndex === row)?.cells[column].display;
  if (!document.hasFocus()) await new Promise<void>(resolve => window.addEventListener("focus", () => resolve(), { once: true }));
  requestAnimationFrame(monitor);
  try {
    for (let n = 0; n < 110; n++) {
      const sample = await selectTarget(`sources/data${String(n % 10).padStart(2, "0")}.yaml`, n < 10 ? "firstSource" : "capacityWarm", false);
      acceptedInteraction(sample);
      assert(desktop.surface.projection?.totalRows === 10_000, "capacity source was truncated");
    }
    await selectTarget(source, "capacitySetup", false);
    let started = performance.now();
    await scroll(9999, 19);
    assert(projected(9999, 19) === "10018", "last row/column value was incorrect");
    stages.longWideScrollMs = performance.now() - started;
    checks.push("long-wide-bounded-projection");
    await scroll(0, 0);
    const search = document.getElementById("search") as HTMLInputElement;
    started = performance.now();
    text(search, "99999");
    await until(() => desktop.surface.projection?.viewState.search === "99999" && !desktop.surface.queryPending, "Find did not return the capacity result");
    await settle();
    assert(desktop.surface.projection!.totalRows === 0 && !desktop.surface.projection!.dirty, "Find changed bytes or returned matches");
    stages.findMs = performance.now() - started;
    text(search, "");
    await until(() => desktop.surface.projection?.totalRows === 10_000 && !desktop.surface.queryPending, "Find clear did not restore the source");
    await settle();
    checks.push("find-exact-empty-without-dirty");
    const viewport = desktop.viewport!;
    viewport.focus();
    // Ordinary keyboard navigation selects the exact paste occurrence.
    desktop.setSelection(0, 0);
    viewport.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }));
    assert(desktop.interaction.selection.field === "field01" && desktop.interaction.selection.id, "paste anchor was not usable");
    const originalRevision = desktop.surface.projection!.revision;
    const clipboard = new DataTransfer();
    clipboard.setData("text/plain", Array.from({ length: 1000 }, (_, row) =>
      Array.from({ length: 10 }, (_, column) => String(1_000_000 + row * 10 + column)).join("\t")).join("\n"));
    started = performance.now();
    viewport.dispatchEvent(new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData: clipboard }));
    await until(() => desktop.surface.projection!.revision !== originalRevision, "ordinary paste was not accepted");
    await settle();
    stages.paste10000CellsMs = performance.now() - started;
    assert(projected(0, 1) === "1000000" && projected(0, 0) === "0", "paste changed the wrong first target");
    await until(() => desktop.surface.status.dirty.length === 1, "paste dirty ownership was not published");
    assert(desktop.surface.status.dirty.length === 1 && desktop.surface.status.dirty[0] === source, "paste dirtied a dependent source");
    await scroll(999, 10);
    assert(projected(999, 10) === "1009999" && projected(999, 11) === "1010", "paste changed the wrong last target or adjacent cell");
    checks.push("ordinary-paste-exact-visible-targets");
    await scroll(0, 0);
    started = performance.now();
    viewport.focus();
    viewport.dispatchEvent(new KeyboardEvent("keydown", { key: "z", ctrlKey: true, bubbles: true }));
    await until(() => projected(0, 1) === "1", "one logical Undo did not restore the source");
    await settle();
    stages.undoMs = performance.now() - started;
    await until(() => !desktop.surface.status.dirty.length, "Undo dirty restoration was not published");
    assert(!desktop.surface.projection!.dirty && Number(desktop.surface.status.dirty.length) === 0, "Undo failed physical dirty restoration");
    checks.push("paste-one-undo-restores-clean-source");
    started = performance.now();
    viewport.dispatchEvent(new KeyboardEvent("keydown", { key: "z", ctrlKey: true, shiftKey: true, bubbles: true }));
    await until(() => projected(0, 1) === "1000000", "Redo did not restore the paste candidate");
    await settle();
    stages.redoMs = performance.now() - started;
    checks.push("redo-restores-paste-candidate");
    viewport.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    await until(() => !!document.querySelector(".active-cell-editor input"), "active cell editor did not mount");
    const editor = document.querySelector<HTMLInputElement>(".active-cell-editor input")!;
    assert(editor === document.activeElement, "capacity editor did not receive focus");
    await frame();
    editor.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await settle();
    assert(!document.querySelector(".active-cell-editor"), "Escape retained a heavy editor");
    await until(() => !desktop.surface.status.diagnosticsPending, "capacity diagnostics did not complete");
    assert(desktop.surface.status.problemCount === 0, "capacity candidate had diagnostics");
    assert(bounds.mountedRows <= 64 && bounds.mountedCells <= 64 * 20 && bounds.activeEditors === 1, "capacity rendering was unbounded");
    await invoke("evidence_write", { report: { format: 1, kind: "desktopCapacity", visibility: document.visibilityState, focused: document.hasFocus(), checks, samples, startup, stages, operations, frames, bounds, dirty: desktop.surface.status.dirty, diagnostics: [], clock: "performance.now; frame timestamps are paint opportunities, not GPU presentation" } });
  } catch (error) {
    await invoke("evidence_write", { report: { format: 1, kind: "desktopCapacity", error: String(error), visibility: document.visibilityState, focused: document.hasFocus(), checks, samples, startup, stages, operations, frames, bounds } });
  } finally {
    monitoring = false;
    desktop.rpc = request;
  }
}
