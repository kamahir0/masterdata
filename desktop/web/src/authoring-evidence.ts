import { invoke } from "@tauri-apps/api/core";
import { desktop, GRID } from "./workspace";

const SOURCE = "sources/catalog-data.yaml",
  SCHEMA = "sources/catalog-schema.yaml";
// This adapter drives the actual native WebView's event handlers. It does not
// substitute for OS pointer, keyboard, IME, accessibility or visual review.
const frame = () =>
  new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
async function until(test: () => boolean, message: string) {
  const deadline = performance.now() + 5000;
  while (!test()) {
    if (performance.now() >= deadline) throw new Error(message);
    await frame();
  }
}
function assert(test: unknown, message: string): asserts test {
  if (!test) throw new Error(message);
}
function key(element: HTMLElement, key: string, extra: KeyboardEventInit = {}) {
  element.dispatchEvent(
    new KeyboardEvent("keydown", { key, bubbles: true, ...extra }),
  );
}
function text(input: HTMLInputElement, value: string) {
  Object.getOwnPropertyDescriptor(
    HTMLInputElement.prototype,
    "value",
  )!.set!.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}
const find = (selector: string) => {
  const element = document.querySelector<HTMLElement>(selector);
  assert(element, `missing ${selector}`);
  return element;
};
const point = (element: HTMLElement) => {
  const rect = element.getBoundingClientRect();
  return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
};
function pointer(
  element: EventTarget,
  kind: string,
  at: { x: number; y: number },
) {
  element.dispatchEvent(
    new PointerEvent(kind, {
      bubbles: true,
      button: 0,
      buttons: kind === "pointerup" ? 0 : 1,
      pointerId: 1,
      pointerType: "mouse",
      isPrimary: true,
      clientX: at.x,
      clientY: at.y,
    }),
  );
}
async function drag(handle: HTMLElement, destination: HTMLElement) {
  const start = point(handle),
    end = point(destination);
  pointer(handle, "pointerdown", start);
  await frame();
  await frame();
  pointer(document, "pointermove", end);
  await until(
    () => !!document.querySelector(".spatial-ghost"),
    "drag did not lift the whole object",
  );
  await frame();
  const ghost = find(".spatial-ghost");
  assert(
    ghost.inert && ghost.closest("[aria-hidden=true]"),
    "drag duplicated interactive controls",
  );
  assert(
    document.querySelectorAll(".spatial-ghost").length === 1,
    "unbounded drag previews",
  );
  assert(
    document.querySelector(".spatial-displaced"),
    "destination neighbours did not open space",
  );
  return { start, end, ghost };
}
async function settle() {
  await until(
    () =>
      !desktop.surface.busy &&
      !desktop.surface.pending &&
      !document.querySelector(".spatial-ghost"),
    "operation did not settle",
  );
  assert(!desktop.surface.error, desktop.surface.error ?? "operation failed");
  await frame();
}
async function focusCell(row: number, column: number) {
  const viewport = desktop.viewport!;
  viewport.focus();
  while (desktop.interaction.selection.row !== row) {
    key(
      viewport,
      desktop.interaction.selection.row < row ? "ArrowDown" : "ArrowUp",
    );
    await frame();
  }
  while (desktop.interaction.selection.column !== column) {
    key(
      viewport,
      desktop.interaction.selection.column < column
        ? "ArrowRight"
        : "ArrowLeft",
    );
    await frame();
  }
  await until(
    () => !!document.getElementById(`cell-${row}-${column}`),
    "selected cell did not mount",
  );
}
async function undo() {
  const revision = desktop.surface.projection!.revision,
    schema = desktop.surface.projection!.schemaRevision;
  find('button[aria-label="Undo"]').click();
  await until(
    () =>
      desktop.surface.projection!.revision !== revision ||
      desktop.surface.projection!.schemaRevision !== schema,
    "Undo was not accepted",
  );
  await settle();
}
export async function run({ startup }: { startup: Record<string, unknown> }) {
  const checks: Record<string, unknown>[] = [];
  const bytes = async (source = SOURCE) =>
    (
      await desktop.rpc<{ before: string; after: string }>({
        kind: "compare",
        source,
      })
    ).data;
  const record = (name: string, value: Record<string, unknown> = {}) =>
    checks.push({ name, ...value });
  if (!document.hasFocus())
    await new Promise<void>((resolve) =>
      window.addEventListener("focus", () => resolve(), { once: true }),
    );
  try {
    await desktop.selectTarget(SOURCE, "authoring", false);
    const base = await bytes(),
      schemaBase = await bytes(SCHEMA);
    const originalRows = desktop.surface.projection!.rows.map((row) => row.id);
    await focusCell(0, 11);
    key(desktop.viewport!, "Enter");
    await until(
      () => !!document.querySelector(".active-cell-editor input"),
      "scalar editor missing",
    );
    const input = find(".active-cell-editor input") as HTMLInputElement;
    const initialEditorFocused = input === document.activeElement;
    assert(initialEditorFocused, "scalar editor did not receive focus");
    input.dispatchEvent(
      new CompositionEvent("compositionstart", { bubbles: true }),
    );
    text(input, "日本語");
    key(input, "Enter", { isComposing: true });
    await frame();
    assert(
      (await bytes()).after === base.after && desktop.interaction.editor,
      "composition Enter committed or navigated",
    );
    input.dispatchEvent(
      new CompositionEvent("compositionend", { bubbles: true, data: "日本語" }),
    );
    key(input, "Enter");
    await until(
      () => !desktop.interaction.editor && !desktop.surface.busy,
      "scalar commit did not finish",
    );
    assert(
      (await bytes()).after ===
        base.after.replace("name: Debug Sword", "name: 日本語"),
      "scalar commit changed non-target bytes",
    );
    record("scalar-composition", {
      initialEditorFocused,
      compositionEnterPreserved: true,
      exactLocalizedBytes: true,
    });
    await undo();
    assert((await bytes()).after === base.after, "scalar Undo failed");
    await focusCell(0, 2);
    pointer(find("#cell-0-2"), "pointerdown", point(find("#cell-0-2")));
    await frame();
    await frame();
    pointer(document, "pointermove", point(find("#cell-1-3")));
    await frame();
    await frame();
    pointer(document, "pointerup", point(find("#cell-1-3")));
    await frame();
    assert(
      desktop.interaction.anchor?.id === originalRows[0] &&
        desktop.interaction.selection.id === originalRows[1] &&
        desktop.interaction.selection.column === 3,
      "range lost occurrence identity",
    );
    key(desktop.viewport!, "Escape");
    await frame();
    assert(!desktop.interaction.anchor, "Escape did not collapse range");
    assert(
      (await bytes()).after === base.after,
      "range selection mutated source",
    );
    record("pointer-range");
    await focusCell(0, 0);
    let gesture = await drag(
      find(".grid-row .row-grip"),
      find(".grid-row:nth-child(2) .row-identity"),
    );
    assert(
      gesture.ghost.querySelector(".row-identity") &&
        gesture.ghost.querySelector(".cell"),
      "row preview omitted identity or values",
    );
    assert(
      (await bytes()).after === base.after,
      "row preview mutated source before drop",
    );
    key(document.body, "Escape");
    await settle();
    assert(
      (await bytes()).after === base.after &&
        !desktop.surface.projection!.dirty,
      "cancelled row drag mutated source/history",
    );
    record("row-drag-cancel");
    gesture = await drag(
      find(".grid-row .row-grip"),
      find(".grid-row:nth-child(2) .row-identity"),
    );
    pointer(document, "pointerup", gesture.end);
    await settle();
    assert(
      desktop.surface.projection!.rows[1].id === originalRows[0] &&
        desktop.interaction.selection.id === originalRows[0],
      "row drop lost moved occurrence/focus",
    );
    await undo();
    assert(
      (await bytes()).after === base.after &&
        !desktop.surface.projection!.canUndo,
      "row drop was not one Undo",
    );
    record("row-drag-drop", { oneUndo: true });
    const originalColumns = desktop.surface.projection!.columns.map(
      (c) => c.field.name,
    );
    gesture = await drag(
      find(".column .column-grip"),
      find(".column:nth-child(4) .column-grip"),
    );
    assert(
      gesture.ghost.querySelector(".column") &&
        gesture.ghost.querySelector(".cell"),
      "column preview omitted header or values",
    );
    assert(
      (await bytes(SCHEMA)).after === schemaBase.after,
      "column preview mutated schema",
    );
    pointer(document, "pointerup", gesture.end);
    await settle();
    assert(
      desktop.surface.projection!.columns[2].field.name === originalColumns[0],
      "column drop destination wrong",
    );
    assert(
      (await bytes()).after === base.after &&
        desktop.surface.projection!.schemaDirty,
      "column drop dirtied record bytes",
    );
    await undo();
    assert(
      (await bytes(SCHEMA)).after === schemaBase.after &&
        !desktop.surface.projection!.schemaCanUndo,
      "column drop was not one schema Undo",
    );
    record("column-drag-drop", { oneUndo: true, separateDataPreserved: true });
    await focusCell(1, 15);
    key(desktop.viewport!, "Enter");
    await until(
      () => document.querySelectorAll(".complex-item").length === 3,
      "Array editor did not publish bounded controls",
    );
    assert(
      (await bytes()).after === base.after &&
        !desktop.surface.projection!.dirty,
      "opening Array made it dirty",
    );
    const firstItem = find(".complex-item").dataset.item;
    gesture = await drag(
      find(".complex-item .element-grip"),
      find(".complex-item:nth-child(2) .element-grip"),
    );
    pointer(document, "pointerup", gesture.end);
    await settle();
    await until(
      () => find(".complex-item:nth-child(2)").dataset.item === firstItem,
      "Array drop lost exact equal-value occurrence",
    );
    assert(
      (await bytes()).after ===
        base.after.replace('numbers: [1, "1", -2]', 'numbers: ["1", 1, -2]'),
      "Array drop changed lexical/non-target bytes",
    );
    key(find(".complex-item:nth-child(2) .element-value button"), "Escape");
    await until(() => !desktop.interaction.complex, "Complex close missing");
    assert(
      desktop.surface.projection!.dirty,
      "closing Complex rolled back committed drop",
    );
    await undo();
    assert(
      (await bytes()).after === base.after &&
        !desktop.surface.projection!.canUndo,
      "Array drop was not one Undo",
    );
    record("array-drag-drop", {
      occurrencePreserved: true,
      lexicalBytesPreserved: true,
      closePreservedCommit: true,
      oneUndo: true,
    });
    await focusCell(1, 15);
    key(desktop.viewport!, "Enter");
    await until(
      () => document.querySelectorAll(".complex-item").length === 3,
      "Array reopen missing",
    );
    find(".complex-item .element-value button").click();
    await until(
      () => !!document.querySelector(".complex-input input"),
      "nested leaf input missing",
    );
    text(find(".complex-input input") as HTMLInputElement, "invalid");
    const add = [
      ...document.querySelectorAll<HTMLButtonElement>(
        ".ant-drawer-body button",
      ),
    ].find((b) => b.textContent?.trim() === "Element");
    assert(add, "Array Add missing");
    add.click();
    await until(
      () =>
        document.querySelectorAll(".complex-item").length === 4 &&
        !desktop.surface.busy,
      "typing then Add did not compose",
    );
    assert(
      (await bytes()).after ===
        base.after.replace(
          'numbers: [1, "1", -2]',
          'numbers: [invalid, "1", -2, null]',
        ),
      "composed Complex operation changed non-target bytes",
    );
    await until(
      () =>
        find(".complex-item:last-of-type .element-value").contains(
          document.activeElement,
        ),
      "Array Add did not focus added occurrence once",
    );
    record("complex-typing-add", {
      twoOperations: true,
      addedOccurrenceFocused: true,
    });
    await undo();
    await until(
      () => document.querySelectorAll(".complex-item").length === 3,
      "Add Undo missing",
    );
    await until(
      () =>
        document.querySelector('.ant-drawer-body [aria-busy="false"]') !== null,
      "Array descriptor did not become current",
    );
    const secondAdd = [
      ...document.querySelectorAll<HTMLButtonElement>(
        ".ant-drawer-body button",
      ),
    ].find((b) => b.textContent?.trim() === "Element")!;
    secondAdd.click();
    const search = find("#search");
    search.focus();
    key(search, "ArrowLeft");
    await until(
      () =>
        document.querySelectorAll(".complex-item").length === 4 &&
        !desktop.surface.busy &&
        document.querySelector('.ant-drawer-body [aria-busy="false"]') !== null,
      "second Add did not finish",
    );
    await frame();
    await frame();
    assert(
      document.activeElement === search,
      "late Array Add focus stole newer user input",
    );
    await undo();
    record("complex-focus-intent", { newerInputRetained: true });
    find("#problems-bar button").click();
    await until(
      () =>
        !desktop.surface.status.diagnosticsPending &&
        desktop.surface.problems.length > 0,
      "Problems did not publish latest generation",
    );
    const gridTop = desktop.viewport!.getBoundingClientRect().top;
    const problem = desktop.surface.problems.find(
      (d) => d.source === SOURCE && d.fieldPath.join("/") === "numbers/0",
    );
    assert(problem, "nested diagnostic missing");
    const buttons = [
      ...document.querySelectorAll<HTMLButtonElement>(
        "#problems .problem-item",
      ),
    ];
    const problemButton = buttons[desktop.surface.problems.indexOf(problem)];
    assert(problemButton, "nested Problems action missing");
    problemButton.click();
    await until(
      () =>
        !!desktop.interaction.complex?.focusPath &&
        !!document.activeElement?.closest(".element-value"),
      "Problems did not focus nested target",
    );
    assert(
      desktop.viewport!.getBoundingClientRect().top === gridTop,
      "Problems arrival shifted the working grid",
    );
    record("nested-problem-focus", {
      stableGrid: true,
      latestGeneration: problem.generation,
    });
    find('button[aria-label="Close Problems"]').click();
    key(document.activeElement as HTMLElement, "Escape");
    await until(
      () => !desktop.interaction.complex,
      "Complex Escape close missing",
    );
    await undo();
    assert(
      (await bytes()).after === base.after &&
        !desktop.surface.projection!.dirty,
      "Complex operations did not restore exact bytes",
    );
    await focusCell(1, 15);
    key(desktop.viewport!, "Enter");
    await until(
      () => document.querySelectorAll(".complex-item").length === 3,
      "Array reopen missing for navigation",
    );
    find(".complex-item .element-value button").click();
    await until(
      () => !!document.querySelector(".complex-input input"),
      "nested navigation input missing",
    );
    text(find(".complex-input input") as HTMLInputElement, "invalid");
    await desktop.selectTarget(SCHEMA, "complex-navigation", false);
    assert(
      !desktop.surface.error && desktop.surface.projection?.clicked === SCHEMA,
      "outgoing Complex typing blocked navigation",
    );
    assert(
      (await bytes()).after ===
        base.after.replace(
          'numbers: [1, "1", -2]',
          'numbers: [invalid, "1", -2]',
        ),
      "navigation discarded uncommitted nested typing",
    );
    await desktop.selectTarget(SOURCE, "complex-revisit", true);
    await undo();
    assert(
      (await bytes()).after === base.after &&
        !desktop.surface.projection!.dirty,
      "nested typing history did not survive navigation",
    );
    record("complex-navigation", {
      typingPreserved: true,
      sourceLocalUndo: true,
    });
    assert(
      !startup.browserErrors ||
        (startup.browserErrors as string[]).length === 0,
      "browser event errors",
    );
    await invoke("evidence_write", {
      report: {
        format: 1,
        kind: "authoring",
        checks,
        startup,
        visibility: document.visibilityState,
        focused: document.hasFocus(),
        reducedMotion: matchMedia("(prefers-reduced-motion: reduce)").matches,
        mountedRows: document.querySelectorAll(".grid-row").length,
        source: await bytes(),
        schema: await bytes(SCHEMA),
      },
    });
  } catch (error) {
    await invoke("evidence_write", {
      report: {
        format: 1,
        kind: "authoring",
        error: String(error),
        checks,
        startup,
        visibility: document.visibilityState,
        focused: document.hasFocus(),
        projection: desktop.surface.projection,
        interaction: desktop.interaction,
        surfaceError: desktop.surface.error,
        dom: {
          active: document.activeElement?.outerHTML.slice(0, 2000),
          items: [
            ...document.querySelectorAll<HTMLElement>(".complex-item"),
          ].map((item) => ({
            id: item.dataset.item,
            value: item
              .querySelector(".element-value")
              ?.outerHTML.slice(0, 2000),
          })),
          drawerBusy: document
            .querySelector(".ant-drawer-body [aria-busy]")
            ?.getAttribute("aria-busy"),
          intended: desktop.inputIntent,
        },
        source: await bytes().catch((e) => ({ error: String(e) })),
        schema: await bytes(SCHEMA).catch((e) => ({ error: String(e) })),
      },
    });
  }
}
