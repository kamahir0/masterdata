import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Inventory,
  Projection,
  Status,
  Reply,
  Diagnostic,
  SelectionSample,
} from "./types";
import "./style.css";
const $ = <T extends HTMLElement = HTMLElement>(id: string) =>
  document.getElementById(id) as T;
$("app").innerHTML =
  `<header id="titlebar"><div id="drag-region" data-tauri-drag-region><strong>MasterData</strong><span id="project-name"></span></div><nav aria-label="Project commands"><button id="open-project">Open Project</button><button id="save">Save</button><button id="validate">Validate</button><button id="project-menu" aria-label="Project menu">•••</button></nav></header><div id="workbench"><aside id="explorer"><div class="pane-heading"><button id="toggle-explorer" aria-expanded="true">▾ Sources</button></div><div id="tree" role="tree" aria-label="Project sources"></div></aside><main><section id="welcome"><h1>YAMLから、データを育てる。</h1><p>Tableと値を直接編集し、元のsourceへ保存します。</p><button id="welcome-open">Open Project</button><p id="welcome-error" role="alert"></p></section><section id="table-surface" hidden><div id="table-context"><h1 id="table-name"></h1><span id="selection-label"></span><select id="record-source" aria-label="Record source"></select><span id="dirty" aria-live="polite"></span><button id="undo" aria-label="Undo">↶</button><button id="redo" aria-label="Redo">↷</button><button id="table-menu" aria-label="Table actions">•••</button></div><div id="grid-area"><div id="pending" role="status" hidden></div><div id="viewport" role="grid" aria-label="Table values" tabindex="0"><div id="grid-header" role="row"></div><div id="grid-body"><div id="rows"></div></div></div><div id="surface-error" role="alert" hidden></div><div id="conflict" hidden><strong>Conflict</strong><span>外部sourceが変更されています。</span><button id="compare">Compare</button><button id="reload-source">Reload source</button></div></div><div id="problems-bar"><button id="problems-toggle" aria-expanded="false">Problems</button><span id="position" aria-live="polite"></span><span id="background-status" role="status"></span></div><section id="problems" hidden aria-label="Problems"><div id="problem-items"></div></section></section></main></div><dialog id="dialog"></dialog>`;
let inventory: Inventory | null = null,
  projection: Projection | null = null;
let selected = "",
  token = 0,
  readToken = 0,
  navigating = false,
  mutation = false,
  selectedRow = 0,
  selectedColumn = 0,
  historySource: string | null = null;
let activeEditor: HTMLInputElement | null = null,
  composing = false;
let editorCommit: Promise<boolean> | null = null;
let status: Status = {
  open: false,
  epoch: 0,
  generation: 0,
  dirty: [],
  recoveryRequired: false,
  diagnosticsPending: false,
  problemCount: 0,
};
let currentSample: SelectionSample | null = null;
export const samples: SelectionSample[] = [];
const localViews = new Map<
  string,
  { top: number; left: number; row: number; column: number }
>();
const viewport = $("viewport"),
  ROW_HEIGHT = 32,
  HEADER_HEIGHT = 72,
  ROW_HEADER = 60,
  COLUMN_WIDTH = 152;
let renderScheduled = false,
  problemsOpen = false;
const basename = (path: string) => path.split("/").at(-1) ?? path;
function text(id: string, value: string) {
  $(id).textContent = value;
}
function errorMessage(e: unknown) {
  return typeof e === "object" && e && "message" in e
    ? String(e.message)
    : String(e);
}
function showError(e: unknown) {
  const panel = inventory ? $("surface-error") : $("welcome-error");
  panel.hidden = false;
  panel.textContent = errorMessage(e);
}
export async function rpc<T>(
  intent: Record<string, unknown>,
): Promise<Reply<T>> {
  return invoke<Reply<T>>("workspace", { intent });
}
function button(label: string, action: () => void) {
  const b = document.createElement("button");
  b.textContent = label;
  b.onclick = action;
  return b;
}
function updateCommands() {
  ($("save") as HTMLButtonElement).disabled =
    !inventory ||
    mutation ||
    navigating ||
    status.recoveryRequired ||
    !projection ||
    (!projection.dirty && !projection.schemaDirty);
  ($("validate") as HTMLButtonElement).disabled = !inventory || mutation;
  ($("undo") as HTMLButtonElement).disabled =
    !projection ||
    navigating ||
    mutation ||
    !(historySource === projection.table.source
      ? projection.schemaCanUndo
      : projection.canUndo);
  ($("redo") as HTMLButtonElement).disabled =
    !projection ||
    navigating ||
    mutation ||
    !(historySource === projection.table.source
      ? projection.schemaCanRedo
      : projection.canRedo);
  text(
    "dirty",
    !navigating && projection && (projection.dirty || projection.schemaDirty)
      ? "● 未保存"
      : "",
  );
  text("background-status", status.diagnosticsPending ? "確認中…" : "");
  text(
    "problems-toggle",
    `Problems${status.problemCount ? ` (${status.problemCount})` : ""}`,
  );
  $("conflict").hidden = navigating || !projection?.conflict;
}
function saveView() {
  if (projection?.source)
    localViews.set(projection.source, {
      top: viewport.scrollTop,
      left: viewport.scrollLeft,
      row: selectedRow,
      column: selectedColumn,
    });
}
function renderTree() {
  const tree = $("tree");
  tree.replaceChildren();
  if (!inventory) return;
  type Folder = { children: Map<string, Folder>; files: Inventory["sources"] };
  const root: Folder = { children: new Map(), files: [] };
  for (const source of inventory.sources) {
    let folder = root;
    const parts = source.path.split("/");
    for (const part of parts.slice(0, -1)) {
      if (!folder.children.has(part))
        folder.children.set(part, { children: new Map(), files: [] });
      folder = folder.children.get(part)!;
    }
    folder.files.push(source);
  }
  const add = (folder: Folder, parent: HTMLElement) => {
    for (const [name, child] of folder.children) {
      const details = document.createElement("details");
      details.open = true;
      const summary = document.createElement("summary");
      summary.textContent = name;
      summary.setAttribute("role", "treeitem");
      summary.setAttribute("aria-expanded", "true");
      details.ontoggle = () =>
        summary.setAttribute("aria-expanded", String(details.open));
      details.append(summary);
      const group = document.createElement("div");
      group.setAttribute("role", "group");
      details.append(group);
      add(child, group);
      parent.append(details);
    }
    for (const s of folder.files) {
      const b = button(basename(s.path), () => void selectTarget(s.path));
      b.setAttribute("role", "treeitem");
      b.dataset.path = s.path;
      b.setAttribute("aria-selected", String(s.path === selected));
      b.title = s.path;
      b.className = "source";
      parent.append(b);
    }
  };
  add(root, tree);
  tree.onkeydown = (e) => {
    const items = Array.from(
        tree.querySelectorAll<HTMLElement>("[role=treeitem]"),
      ).filter((n) => n.getClientRects().length),
      index = items.indexOf(document.activeElement as HTMLElement);
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      items[
        Math.max(
          0,
          Math.min(items.length - 1, index + (e.key === "ArrowDown" ? 1 : -1)),
        )
      ]?.focus();
    } else if (e.key === "ArrowRight" || e.key === "ArrowLeft") {
      const d = (document.activeElement as HTMLElement)?.closest("details");
      if (d) {
        e.preventDefault();
        d.open = e.key === "ArrowRight";
      }
    }
  };
}
async function choice(
  title: string,
  description: string,
  choices: string[],
): Promise<string> {
  const dialog = $<HTMLDialogElement>("dialog");
  if (dialog.open) return "Cancel";
  dialog.replaceChildren();
  const h = document.createElement("h2");
  h.textContent = title;
  const p = document.createElement("p");
  p.textContent = description;
  dialog.append(h, p);
  return new Promise((resolve) => {
    const finish = (value: string) => {
      dialog.close();
      resolve(value);
    };
    const actions = document.createElement("div");
    actions.className = "dialog-actions";
    for (const value of choices)
      actions.append(button(value, () => finish(value)));
    dialog.append(actions);
    dialog.oncancel = (e) => {
      e.preventDefault();
      finish("Cancel");
    };
    dialog.showModal();
    (actions.lastElementChild as HTMLElement)?.focus();
  });
}
async function guard(): Promise<"saved" | "discard" | "cancel"> {
  if (!(await commitEditor())) return "cancel";
  // The actor reply is a barrier for pending authoring operations. A status event
  // may still be in transit when the OS asks to close the window.
  if (!inventory) return "saved";
  const latest = await rpc<Inventory>({ kind: "inventory" });
  if (!latest.data.dirty.length) return "saved";
  const c = await choice(
    "未保存の変更",
    "開いているsourceの変更をどう扱いますか。",
    ["Save All", "Don't Save", "Cancel"],
  );
  if (c === "Cancel") return "cancel";
  if (c === "Don't Save") return "discard";
  try {
    const r = await rpc<{ outcome: string; source: string; message: string }[]>(
      { kind: "saveAll" },
    );
    if (r.data.some((s) => s.outcome !== "Success")) {
      showError(
        r.data
          .filter((s) => s.outcome !== "Success")
          .map((s) => `${s.source}: ${s.outcome} ${s.message}`)
          .join("\n"),
      );
      return "cancel";
    }
    return "saved";
  } catch (e) {
    showError(e);
    return "cancel";
  }
}
export async function openProject(path: string, discard = false) {
  const r = await rpc<Inventory>({ kind: "open", path, discard });
  inventory = r.data;
  projection = null;
  selected = "";
  localViews.clear();
  historySource = null;
  text("project-name", inventory.project.name);
  $("welcome").hidden = true;
  $("table-surface").hidden = true;
  $("welcome-error").hidden = true;
  renderTree();
  updateCommands();
  return r;
}
async function pickProject() {
  try {
    const path = await invoke<string | null>("pick_project");
    if (path) {
      const g = await guard();
      if (g !== "cancel") await openProject(path, g === "discard");
    }
  } catch (e) {
    showError(e);
  }
}
$("open-project").onclick = () => void pickProject();
$("welcome-open").onclick = () => void pickProject();
$("toggle-explorer").onclick = () => {
  $("tree").hidden = !$("tree").hidden;
  $("toggle-explorer").setAttribute("aria-expanded", String(!$("tree").hidden));
};
export async function selectTarget(
  path: string,
  caseName = "ordinary",
  restore = true,
  startOverride?: number,
): Promise<SelectionSample> {
  const sameTarget = caseName === "after-operation" && selected === path;
  saveView();
  selected = path;
  const myToken = ++token;
  const sample: SelectionSample = {
    target: path,
    token: myToken,
    caseName,
    input: performance.now(),
    selectionPublication: 0,
  };
  samples.push(sample);
  currentSample = sample;
  $("table-surface").hidden = false;
  navigating = !sameTarget;
  viewport.hidden = !sameTarget;
  $("record-source").hidden =
    !sameTarget || (projection?.sources.length ?? 0) < 2;
  $("pending").hidden = sameTarget;
  $("pending").textContent = `${basename(path)} を開いています…`;
  text("selection-label", path);
  text(
    "table-name",
    inventory?.sources.find((s) => s.path === path)?.binding ?? basename(path),
  );
  sample.selectionPublication = performance.now();
  if (!sameTarget) $("surface-error").hidden = true;
  $("conflict").hidden = true;
  updateCommands();
  for (const item of document.querySelectorAll<HTMLElement>(".source"))
    item.setAttribute("aria-selected", String(item.dataset.path === path));
  try {
    if (!sameTarget && !(await commitEditor(false)))
      throw new Error("uncommitted edit");
    if (myToken !== token) {
      sample.invalid = "obsolete";
      return sample;
    }
    if (!sameTarget) projection = null;
    const myRead = ++readToken;
    const previous = restore ? localViews.get(path) : undefined,
      start =
        startOverride ??
        Math.max(
          0,
          Math.floor(((previous?.top ?? 0) - HEADER_HEIGHT) / ROW_HEIGHT) - 4,
        ),
      count = Math.min(
        64,
        Math.ceil(Math.max(640, viewport.clientHeight) / ROW_HEIGHT) + 10,
      );
    const r = await rpc<Projection>({
      kind: "select",
      path,
      start,
      count,
      token: myRead,
    });
    sample.ipcReturn = performance.now();
    sample.host = r.host;
    sample.engine = r.data.measurement;
    if (myToken !== token || myRead !== readToken || selected !== path) {
      sample.invalid = "obsolete";
      return sample;
    }
    projection = r.data;
    navigating = false;
    if (caseName !== "after-operation")
      historySource = projection.source ?? projection.table.source;
    sample.statePublication = performance.now();
    const saved = restore
      ? (localViews.get(projection.source ?? "") ?? previous)
      : undefined;
    selectedRow =
      saved?.row ??
      Math.min(selectedRow, Math.max(0, projection.totalRows - 1));
    selectedColumn =
      saved?.column ?? Math.min(selectedColumn, projection.columns.length - 1);
    text("table-name", projection.table.name);
    text("selection-label", projection.source ?? projection.table.source);
    const picker = $<HTMLSelectElement>("record-source");
    picker.replaceChildren();
    picker.hidden = projection.sources.length < 2;
    for (const source of projection.sources) {
      const option = document.createElement("option");
      option.value = source;
      option.textContent = basename(source);
      option.selected = source === projection.source;
      picker.append(option);
    }
    renderHeader();
    renderRows();
    viewport.hidden = false;
    $("pending").hidden = true;
    viewport.scrollTop = saved?.top ?? Math.max(0, start * ROW_HEIGHT);
    viewport.scrollLeft = saved?.left ?? 0;
    sample.domCommit = performance.now();
    updateCommands();
    viewport.getBoundingClientRect();
    sample.layout = performance.now();
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => {
        if (myToken === token && projection?.clicked === path) {
          sample.paintOpportunity = performance.now();
          sample.mountedRows = $("rows").childElementCount;
        } else sample.invalid = "obsolete before paint";
        resolve();
      }),
    );
    return sample;
  } catch (e) {
    if (myToken !== token) {
      sample.invalid = "obsolete";
      return sample;
    }
    sample.invalid = errorMessage(e);
    projection = null;
    navigating = false;
    updateCommands();
    $("pending").hidden = true;
    showError(e);
    throw e;
  }
}
$("record-source").onchange = () =>
  void selectTarget($<HTMLSelectElement>("record-source").value);
function template() {
  return `${ROW_HEADER}px repeat(${projection?.columns.length ?? 0}, ${COLUMN_WIDTH}px)`;
}
function renderHeader() {
  const header = $("grid-header");
  header.replaceChildren();
  if (!projection) return;
  header.style.gridTemplateColumns = template();
  header.style.width = `${ROW_HEADER + projection.columns.length * COLUMN_WIDTH}px`;
  const corner = document.createElement("div");
  corner.className = "corner";
  corner.setAttribute("role", "columnheader");
  corner.textContent = "#";
  header.append(corner);
  for (const col of projection.columns) {
    const h = document.createElement("div");
    h.className = "column";
    h.setAttribute("role", "columnheader");
    h.addEventListener("focusin", () => {
      historySource = projection?.table.source ?? null;
      updateCommands();
    });
    const name = document.createElement("span");
    name.className = "field-name";
    name.textContent = col.field.name;
    const type = document.createElement("select");
    type.setAttribute("aria-label", `${col.field.name} type`);
    for (const t of new Set([
      "int",
      "uint",
      "long",
      "ulong",
      "float",
      "double",
      "bool",
      "string",
      ...(inventory?.types ?? []),
      col.field.typeName,
    ])) {
      const o = document.createElement("option");
      o.value = t;
      o.textContent = t;
      o.selected = t === col.field.typeName;
      type.append(o);
    }
    type.onchange = () =>
      void changeSchema(col.field.name, { typeName: type.value });
    const modifiers = document.createElement("div");
    modifiers.className = "modifiers";
    for (const [caption, key] of [
      ["?", "nullable"],
      ["[ ]", "array"],
    ] as const) {
      const b = button(
        caption,
        () => void changeSchema(col.field.name, { [key]: !col.field[key] }),
      );
      b.setAttribute("aria-label", `${col.field.name} ${key}`);
      b.setAttribute("aria-pressed", String(col.field[key]));
      modifiers.append(b);
    }
    h.append(name, type, modifiers);
    header.append(h);
  }
}
function renderRows() {
  if (!projection) return;
  const rows = $("rows");
  rows.replaceChildren();
  rows.style.top = `${projection.rowStart * ROW_HEIGHT}px`;
  rows.style.width = `${ROW_HEADER + projection.columns.length * COLUMN_WIDTH}px`;
  $("grid-body").style.height = `${projection.totalRows * ROW_HEIGHT}px`;
  $("grid-body").style.width = rows.style.width;
  viewport.setAttribute("aria-rowcount", String(projection.totalRows + 1));
  viewport.setAttribute("aria-colcount", String(projection.columns.length + 1));
  for (const row of projection.rows) {
    const r = document.createElement("div");
    r.className = "grid-row";
    r.dataset.row = row.id;
    r.style.gridTemplateColumns = template();
    r.setAttribute("role", "row");
    r.setAttribute("aria-rowindex", String(row.occurrence + 1));
    const identity = document.createElement("div");
    identity.className = "row-identity";
    identity.setAttribute("role", "rowheader");
    identity.textContent = String(row.occurrence);
    r.append(identity);
    for (let column = 0; column < row.cells.length; column++) {
      const value = row.cells[column],
        cell = document.createElement("div");
      cell.className = "cell";
      cell.setAttribute("role", "gridcell");
      cell.id = `cell-${row.occurrence - 1}-${column}`;
      cell.dataset.column = String(column);
      cell.textContent = value.display;
      cell.setAttribute("aria-colindex", String(column + 2));
      cell.setAttribute("aria-readonly", String(!value.editable));
      if (!value.valid) {
        cell.classList.add("invalid");
        cell.setAttribute("aria-invalid", "true");
        cell.title = `${value.problem?.code ?? ""} ${value.problem?.message ?? ""}`;
      } else if (value.reason) cell.title = value.reason;
      cell.onpointerdown = (e) => {
        if (e.button !== 0 || activeEditor) return;
        selectedRow = row.occurrence - 1;
        selectedColumn = column;
        updateSelection();
        viewport.focus();
      };
      cell.ondblclick = () => beginEditor();
      r.append(cell);
    }
    rows.append(r);
  }
  updateSelection();
}
function updateSelection() {
  for (const c of document.querySelectorAll(".cell.selected")) {
    c.classList.remove("selected");
    c.removeAttribute("aria-selected");
  }
  const cell = $(`cell-${selectedRow}-${selectedColumn}`);
  if (cell) {
    cell.classList.add("selected");
    cell.setAttribute("aria-selected", "true");
    viewport.setAttribute("aria-activedescendant", cell.id);
  }
  text(
    "position",
    projection
      ? `${Math.min(selectedRow + 1, projection.totalRows)} / ${projection.totalRows} · ${projection.columns[selectedColumn]?.field.name ?? ""}`
      : "",
  );
}
async function refreshProjection() {
  if (selected)
    await selectTarget(
      selected,
      "after-operation",
      true,
      projection?.rowStart ?? 0,
    );
}
async function perform(action: () => Promise<unknown>) {
  if (mutation) return;
  const originPath = selected;
  mutation = true;
  $("surface-error").hidden = true;
  updateCommands();
  try {
    await action();
    if (selected === originPath) await refreshProjection();
  } catch (e) {
    showError(e);
  } finally {
    mutation = false;
    updateCommands();
  }
}
async function changeSchema(
  field: string,
  change: { typeName?: string; nullable?: boolean; array?: boolean },
) {
  if (!projection) return;
  const p = projection,
    f = p.columns.find((c) => c.field.name === field)!.field;
  const nullable = change.nullable ?? (change.array ? false : f.nullable),
    array = change.array ?? (change.nullable ? false : f.array);
  historySource = p.table.source;
  await perform(async () => {
    await rpc({
      kind: "schema",
      source: p.table.source,
      revision: p.schemaRevision,
      field,
      nullable,
      array,
      typeName: change.typeName ?? null,
    });
  });
}
function beginEditor() {
  if (!projection?.source || activeEditor || mutation) return;
  const row = projection.rows.find((r) => r.occurrence - 1 === selectedRow),
    cell = row?.cells[selectedColumn],
    shape = projection.columns[selectedColumn]?.shape;
  if (!row || !cell?.editable || !shape) return;
  if (
    shape.array ||
    shape.category === "custom" ||
    shape.category === "flags"
  ) {
    showError("Complex Valueの直接編集を接続中です。");
    return;
  }
  const target = $(`cell-${selectedRow}-${selectedColumn}`);
  if (!target) return;
  const input = document.createElement("input");
  input.className = "active-editor";
  input.value = cell.display;
  input.setAttribute(
    "aria-label",
    `${projection.columns[selectedColumn].field.name} row ${row.occurrence}`,
  );
  target.replaceChildren(input);
  activeEditor = input;
  input.addEventListener("compositionstart", () => {
    composing = true;
  });
  input.addEventListener("compositionend", () => {
    composing = false;
  });
  input.onkeydown = (e) => {
    if (e.isComposing || composing) return;
    if (e.key === "Escape") {
      e.preventDefault();
      activeEditor = null;
      target.textContent = cell.display;
      viewport.focus();
    } else if (e.key === "Enter" || e.key === "Tab") {
      e.preventDefault();
      const delta = e.shiftKey ? -1 : 1;
      const originPath = selected;
      void commitEditor().then((ok) => {
        if (
          ok &&
          selected === originPath &&
          !navigating &&
          selected === projection?.clicked
        ) {
          move(e.key === "Tab" ? 0 : delta, e.key === "Tab" ? delta : 0);
          viewport.focus();
        }
      });
    }
    e.stopPropagation();
  };
  input.onblur = () => {
    if (activeEditor === input && !composing) void commitEditor();
  };
  input.focus();
  input.select();
}
async function commitEditor(refresh = true): Promise<boolean> {
  if (editorCommit) return editorCommit;
  if (!activeEditor) return true;
  if (composing) return false;
  const input = activeEditor,
    p = projection,
    originToken = token,
    originPath = selected;
  if (!p?.source) return false;
  const row = p.rows.find((r) => r.occurrence - 1 === selectedRow);
  if (!row) return false;
  const field = p.columns[selectedColumn].field.name,
    value = input.value;
  activeEditor = null;
  if (value === row.cells[selectedColumn].display) {
    input.parentElement!.textContent = value;
    return true;
  }
  editorCommit = (async () => {
    try {
      await rpc({
        kind: "editText",
        source: p.source,
        revision: p.revision,
        row: row.id,
        field,
        text: value,
      });
      if (originToken === token && originPath === selected) {
        historySource = p.source;
        if (refresh) await refreshProjection();
      }
      return true;
    } catch (e) {
      activeEditor = input;
      if (input.isConnected) input.focus();
      showError(e);
      return false;
    }
  })();
  try {
    return await editorCommit;
  } finally {
    editorCommit = null;
  }
}

function move(rows: number, columns: number) {
  if (!projection) return;
  selectedRow = Math.max(
    0,
    Math.min(projection.totalRows - 1, selectedRow + rows),
  );
  selectedColumn = Math.max(
    0,
    Math.min(projection.columns.length - 1, selectedColumn + columns),
  );
  const cell = $(`cell-${selectedRow}-${selectedColumn}`);
  if (cell) {
    cell.scrollIntoView({ block: "nearest", inline: "nearest" });
    updateSelection();
  } else {
    viewport.scrollTop = selectedRow * ROW_HEIGHT;
    void updateWindow();
  }
}
viewport.onfocus = () => {
  if (projection) historySource = projection.source ?? projection.table.source;
  updateCommands();
};
viewport.onkeydown = (e) => {
  if (activeEditor || e.isComposing || mutation || !projection) return;
  const mod = e.metaKey || e.ctrlKey;
  if (mod && e.key.toLowerCase() === "z") {
    e.preventDefault();
    void undo(e.shiftKey);
    return;
  }
  if (mod && e.key.toLowerCase() === "y") {
    e.preventDefault();
    void undo(true);
    return;
  }
  if (e.key === "Enter" || e.key === "F2") {
    e.preventDefault();
    beginEditor();
    return;
  }
  const direction: { [key: string]: [number, number] } = {
    ArrowDown: [1, 0],
    ArrowUp: [-1, 0],
    ArrowLeft: [0, -1],
    ArrowRight: [0, 1],
  };
  if (direction[e.key]) {
    e.preventDefault();
    const before = [selectedRow, selectedColumn];
    move(...direction[e.key]);
    if (
      currentSample &&
      currentSample.token === token &&
      !currentSample.firstAccepted &&
      (before[0] !== selectedRow || before[1] !== selectedColumn)
    )
      currentSample.firstAccepted = performance.now();
  }
};
async function updateWindow() {
  if (!projection || activeEditor || mutation || navigating) return;
  const start = Math.max(
      0,
      Math.floor((viewport.scrollTop - HEADER_HEIGHT) / ROW_HEIGHT) - 4,
    ),
    count = Math.min(64, Math.ceil(viewport.clientHeight / ROW_HEIGHT) + 12);
  if (
    start >= projection.rowStart &&
    Math.min(projection.totalRows, start + count) <=
      projection.rowStart + projection.rows.length
  )
    return;
  const path = selected,
    myToken = token,
    myRead = ++readToken;
  try {
    const r = await rpc<Projection>({
      kind: "select",
      path,
      start,
      count,
      token: myRead,
    });
    if (
      myRead !== readToken ||
      myToken !== token ||
      !projection ||
      projection.clicked !== path
    )
      return;
    projection = r.data;
    renderRows();
  } catch (e) {
    if (myRead === readToken && myToken === token) showError(e);
  }
}

viewport.onscroll = () => {
  if (renderScheduled) return;
  renderScheduled = true;
  requestAnimationFrame(() => {
    renderScheduled = false;
    void updateWindow();
  });
};
async function undo(redo: boolean) {
  if (!historySource) return;
  const source = historySource;
  await perform(async () => {
    await rpc({ kind: "undo", source, redo });
  });
}
$("undo").onclick = () => void undo(false);
$("redo").onclick = () => void undo(true);
async function save() {
  if (!(await commitEditor()) || !projection) return;
  const p = projection;
  await perform(async () => {
    const r = await rpc<{ source: string; outcome: string; message: string }[]>(
      { kind: "save", table: p.table.name, source: p.source },
    );
    const failures = r.data.filter((s) => s.outcome !== "Success");
    if (failures.length)
      showError(
        failures
          .map((s) => `${s.source}: ${s.outcome} ${s.message}`)
          .join("\n"),
      );
  });
}
$("save").onclick = () => void save();
$("validate").onclick = () => void rpc({ kind: "validate" }).catch(showError);
async function showProblems() {
  const r = await rpc<{
    problems: Diagnostic[];
    pending: boolean;
    generation: number;
  }>({ kind: "problems", start: 0, count: 256 });
  const items = $("problem-items");
  items.replaceChildren();
  for (const p of r.data.problems)
    items.append(
      button(
        `${p.source}:${p.line} · ${p.code} ${p.message}`,
        () => void focusProblem(p),
      ),
    );
  if (!r.data.problems.length) {
    const p = document.createElement("p");
    p.textContent = r.data.pending ? "確認中…" : "Problemsはありません。";
    items.append(p);
  }
}
async function focusProblem(problem: Diagnostic) {
  const ordinal = (problem.occurrence ?? 1) - 1;
  await selectTarget(
    problem.source,
    "problem",
    false,
    Math.max(0, ordinal - 3),
  );
  selectedRow = ordinal;
  selectedColumn = Math.max(
    0,
    projection?.columns.findIndex(
      (c) => c.field.name === problem.fieldPath[0],
    ) ?? 0,
  );
  updateSelection();
  viewport.focus();
}
$("problems-toggle").onclick = () => {
  problemsOpen = !problemsOpen;
  $("problems").hidden = !problemsOpen;
  $("problems-toggle").setAttribute("aria-expanded", String(problemsOpen));
  if (problemsOpen) void showProblems();
};
async function compare() {
  if (!projection?.source) return;
  const source = projection.source,
    r = await rpc<{ identity: string; before: string; after: string }>({
      kind: "compare",
      source,
    });
  const dialog = $<HTMLDialogElement>("dialog");
  dialog.replaceChildren();
  const title = document.createElement("h2");
  title.textContent = "Source / 保存候補";
  dialog.append(title);
  const views = document.createElement("div");
  views.className = "compare";
  for (const [label, value] of [
    ["現在のdisk", r.data.before],
    ["未保存候補", r.data.after],
  ]) {
    const pane = document.createElement("label");
    pane.textContent = label;
    const area = document.createElement("textarea");
    area.readOnly = true;
    area.value = value;
    pane.append(area);
    views.append(pane);
  }
  dialog.append(views);
  const actions = document.createElement("div");
  actions.className = "dialog-actions";
  actions.append(button("Close", () => dialog.close()));
  if (projection.conflict)
    actions.append(
      button("Overwrite…", () => {
        dialog.close();
        void choice(
          "Overwrite",
          "確認した外部sourceをこの候補で置き換えます。",
          ["Overwrite", "Cancel"],
        ).then((c) => {
          if (c === "Overwrite")
            void perform(async () => {
              const result = await rpc<{ outcome: string; message: string }>({
                kind: "overwrite",
                source,
                identity: r.data.identity,
              });
              if (result.data.outcome !== "Success")
                showError(`${result.data.outcome}: ${result.data.message}`);
            });
        });
      }),
    );
  dialog.append(actions);
  dialog.showModal();
}
$("compare").onclick = () => void compare();
$("reload-source").onclick = () => {
  if (!projection?.source) return;
  const source = projection.source;
  void choice(
    "Reload source",
    "このsourceの未保存変更を破棄し、diskを読み直します。",
    ["Reload source", "Cancel"],
  ).then((c) => {
    if (c === "Reload source")
      void perform(async () => {
        await rpc({ kind: "reloadSource", source });
      });
  });
};
$("table-menu").onclick = () => {
  void choice("Table actions", "保存候補をsourceと比較できます。", [
    "Compare",
    "Cancel",
  ]).then((c) => {
    if (c === "Compare") void compare();
  });
};
$("project-menu").onclick = () => {
  void choice("Project", "Project全体の保存と読み直し。", [
    "Save All",
    "Reload Project",
    "Cancel",
  ]).then(async (c) => {
    if (c === "Save All")
      await perform(async () => {
        const r = await rpc<{ source: string; outcome: string }[]>({
          kind: "saveAll",
        });
        if (r.data.some((s) => s.outcome !== "Success")) showError(r.data);
      });
    else if (c === "Reload Project" && inventory) {
      const g = await guard();
      if (g !== "cancel") await openProject(inventory.root, g === "discard");
    }
  });
};
document.onkeydown = (e) => {
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s") {
    e.preventDefault();
    void save();
  }
};
await listen<Status>("workspace-status", (event) => {
  const next = event.payload;
  if (
    next.epoch < status.epoch ||
    (next.epoch === status.epoch && next.generation < status.generation)
  )
    return;
  status = next;
  updateCommands();
  for (const item of document.querySelectorAll<HTMLElement>(".source"))
    item.classList.toggle(
      "dirty",
      status.dirty.includes(item.dataset.path ?? ""),
    );
  if (problemsOpen && !next.diagnosticsPending) void showProblems();
});
await listen<string>("exit-requested", () => {
  void guard()
    .then(async (g) => {
      if (g !== "cancel")
        await invoke("finish_exit", { discard: g === "discard" });
    })
    .catch(showError);
});
const boot = await invoke<{
  platform: string;
  initialProject: string | null;
  evidence: boolean;
}>("boot");
document.body.classList.toggle("mac", boot.platform === "macos");
updateCommands();
if (boot.initialProject) {
  try {
    await openProject(boot.initialProject);
  } catch (e) {
    showError(e);
  }
}
export function acceptedInteraction(sample: SelectionSample) {
  viewport.focus();
  viewport.dispatchEvent(
    new KeyboardEvent("keydown", {
      key:
        selectedColumn === (projection?.columns.length ?? 1) - 1
          ? "ArrowLeft"
          : "ArrowRight",
      bubbles: true,
    }),
  );
  if (!sample.firstAccepted)
    sample.invalid = sample.invalid ?? "interaction was not accepted";
}
export function view() {
  return projection;
}
if (boot.evidence) {
  const evidence = await import("./evidence");
  await evidence.run({ rpc, selectTarget, samples, acceptedInteraction, view });
}
