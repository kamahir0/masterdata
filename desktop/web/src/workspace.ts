import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { flushSync } from "react-dom";
import type {
  Inventory,
  Projection,
  Status,
  Reply,
  Diagnostic,
  SelectionSample,
  Shape,
} from "./types";

export const GRID = {
  row: 32,
  header: 64,
  identity: 60,
  column: 160,
  overscan: 4,
};
export type Preference = "system" | "light" | "dark";
export type Choice = {
  title: string;
  description: string;
  actions: string[];
  finish: (answer: string) => void;
};
export type Compare = {
  source: string;
  identity: string;
  before: string;
  after: string;
  conflict: boolean;
};
type WriteResult = { source: string; outcome: string; message: string };
export interface EditorNode {
  path: string[];
  label: string;
  shape: Shape | null;
  kind: string;
  display: string;
  input: string | null;
  valid: boolean;
  editable: boolean;
  reason: string | null;
  problem: { message: string; path: string[] } | null;
  children: EditorNode[];
  totalChildren: number;
  start: number;
  selectedMembers: string[];
}
export interface ComplexView {
  sessionEpoch: number;
  source: string;
  row: string;
  revision: number;
  generation: number;
  node: EditorNode;
}
interface ProblemTarget {
  source: string;
  row: string;
  field: string | null;
  viewIndex: number | null;
  editorPath: string[] | null;
  focusPath: string[];
  editorStart: number;
}
export interface ComplexTarget {
  epoch: number;
  source: string;
  row: string;
  root: string;
  path: string[];
  start: number;
  focusPath?: string[];
  inputIntent?: number;
}
export interface Editor {
  epoch: number;
  source: string;
  revision: number;
  generation: number;
  row: string;
  field: string;
  rowIndex: number;
  column: number;
  initial: string;
  display: string;
}
export interface Selection {
  row: number;
  column: number;
  id: string | null;
  field: string | null;
}
export interface Interaction {
  selection: Selection;
  anchor: Selection | null;
  editor: Editor | null;
  complex: ComplexTarget | null;
  focusIntent: number;
}
export interface Surface {
  inventory: Inventory | null;
  projection: Projection | null;
  target: string;
  pending: boolean;
  busy: boolean;
  error: string | null;
  status: Status;
  query: string;
  queryPending: boolean;
  problemsOpen: boolean;
  problems: Diagnostic[];
  choice: Choice | null;
  comparison: Compare | null;
  appearance: boolean;
  theme: Preference;
}
type SavedView = {
  top: number;
  left: number;
  selection: Selection;
  anchor: Selection | null;
};
const initialStatus: Status = {
  open: false,
  epoch: 0,
  generation: 0,
  dirty: [],
  uncertain: [],
  recoveryRequired: false,
  diagnosticsPending: false,
  problemCount: 0,
};
const errorText = (e: unknown) =>
  typeof e === "object" && e && "message" in e ? String(e.message) : String(e);
export const basename = (path: string) => path.split("/").at(-1) ?? path;

// The Rust workspace owns domain lifetime. These two subscriptions publish only
// a bounded projection/chrome and local interaction respectively. Cells never
// subscribe to the selection or to project status.
class Desktop {
  surface: Surface = {
    inventory: null,
    projection: null,
    target: "",
    pending: false,
    busy: false,
    error: null,
    status: initialStatus,
    query: "",
    queryPending: false,
    problemsOpen: false,
    problems: [],
    choice: null,
    comparison: null,
    appearance: false,
    theme: "system",
  };
  interaction: Interaction = {
    selection: { row: 0, column: 0, id: null, field: null },
    anchor: null,
    editor: null,
    complex: null,
    focusIntent: 0,
  };
  private surfaces = new Set<() => void>();
  private interactions = new Set<() => void>();
  subscribe = (callback: () => void) => {
    this.surfaces.add(callback);
    return () => {
      this.surfaces.delete(callback);
    };
  };
  subscribeInteraction = (callback: () => void) => {
    this.interactions.add(callback);
    return () => {
      this.interactions.delete(callback);
    };
  };
  snapshot = () => this.surface;
  interactionSnapshot = () => this.interaction;
  viewport: HTMLElement | null = null;
  private local = new Map<string, SavedView>();
  private token = 0;
  private readToken = 0;
  private problemToken = 0;
  private queryVersion = 0;
  private queryRunning = false;
  private queryTimer: ReturnType<typeof setTimeout> | null = null;
  private queuedQuery: {
    source: string;
    epoch: number;
    path: string;
    text: string;
    version: number;
  } | null = null;
  private paintResolve = new Map<number, () => void>();
  private historySource: string | null = null;
  private commitActive: (() => Promise<boolean>) | null = null;
  private clipboardBusy = false;
  private guardRunning: Promise<"saved" | "discard" | "cancel"> | null = null;
  currentSample: SelectionSample | null = null;
  samples: SelectionSample[] = [];
  evidence = false;
  inputIntent = 0;
  noteInputIntent = () => {
    this.inputIntent++;
  };
  private publish(delta: Partial<Surface>) {
    this.surface = { ...this.surface, ...delta };
    for (const f of this.surfaces) f();
  }
  private interact(delta: Partial<Interaction>) {
    this.interaction = { ...this.interaction, ...delta };
    for (const f of this.interactions) f();
  }
  showError = (e: unknown) => this.publish({ error: errorText(e) });
  dismissError = () => this.publish({ error: null });
  setTheme = async (theme: Preference) => {
    this.publish({ theme });
    try {
      await invoke("application_preferences", { theme });
    } catch (e) {
      this.showError(e);
    }
  };
  appearance = (open: boolean) => this.publish({ appearance: open });
  async rpc<T>(intent: Record<string, unknown>): Promise<Reply<T>> {
    return invoke<Reply<T>>("workspace", {
      intent,
      epoch: intent.epoch ?? this.surface.status.epoch,
    });
  }
  bindEditor(commit: () => Promise<boolean>) {
    this.commitActive = commit;
    return () => {
      if (this.commitActive === commit) this.commitActive = null;
    };
  }
  async commit() {
    return this.commitActive ? this.commitActive() : true;
  }
  choose = (title: string, description: string, actions: string[]) => {
    if (this.surface.choice) return Promise.resolve("Cancel");
    return new Promise<string>((resolve) =>
      this.publish({
        choice: {
          title,
          description,
          actions,
          finish: (answer) => {
            this.publish({ choice: null });
            resolve(answer);
          },
        },
      }),
    );
  };
  private saveView() {
    const p = this.surface.projection;
    if (p?.source && this.viewport)
      this.local.set(p.source, {
        top: this.viewport.scrollTop,
        left: this.viewport.scrollLeft,
        selection: this.interaction.selection,
        anchor: this.interaction.anchor,
      });
    if (p?.source && p.clicked !== p.source && this.viewport)
      this.local.set(p.clicked, this.local.get(p.source)!);
  }
  async openProject(path: string, discard = false) {
    const r = await this.rpc<Inventory>({ kind: "open", path, discard });
    this.token++;
    this.readToken++;
    this.queryVersion++;
    this.queuedQuery = null;
    this.local.clear();
    this.historySource = null;
    this.interact({
      selection: { row: 0, column: 0, id: null, field: null },
      anchor: null,
      editor: null,
      complex: null,
      focusIntent: this.interaction.focusIntent + 1,
    });
    this.publish({
      inventory: r.data,
      projection: null,
      target: "",
      pending: false,
      busy: false,
      query: "",
      queryPending: false,
      error: null,
      comparison: null,
      problems: [],
      status: {
        ...initialStatus,
        open: true,
        epoch: r.host.epoch,
        generation: r.data.generation,
        dirty: r.data.dirty,
        uncertain: r.data.uncertain,
      },
    });
    return r;
  }
  pickProject = async () => {
    try {
      const path = await invoke<string | null>("pick_project");
      if (!path) return;
      const guard = await this.guard();
      if (guard !== "cancel") await this.openProject(path, guard === "discard");
    } catch (e) {
      this.showError(e);
    }
  };
  private scope(p: Projection) {
    return {
      epoch: p.sessionEpoch,
      source: p.source,
      revision: p.revision,
      generation: p.generation,
    };
  }
  private currentFor(p: Projection) {
    const current = this.surface.projection;
    return !this.surface.pending &&
      current?.sessionEpoch === p.sessionEpoch &&
      current.clicked === p.clicked &&
      current.source === p.source &&
      current.table.source === p.table.source
      ? current
      : null;
  }
  async selectTarget(
    path: string,
    caseName = "ordinary",
    restore = true,
    startOverride?: number,
  ): Promise<SelectionSample> {
    const after = caseName === "after-operation",
      searching = caseName === "search-result";
    const same = (after || searching) && this.surface.target === path,
      queryVersion = this.queryVersion;
    this.saveView();
    const mine = ++this.token;
    for (const [old, resolve] of this.paintResolve)
      if (old !== mine) {
        resolve();
        this.paintResolve.delete(old);
      }
    const sample: SelectionSample = {
      target: path,
      token: mine,
      caseName,
      input: performance.now(),
      selectionPublication: 0,
    };
    this.samples.push(sample);
    if (!this.evidence && this.samples.length > 256) this.samples.shift();
    this.currentSample = sample;
    if (!same) this.interact({ focusIntent: this.interaction.focusIntent + 1 });
    flushSync(() =>
      this.publish({
        target: path,
        pending: !same || searching,
        queryPending: searching,
        error: same ? this.surface.error : null,
        comparison: null,
      }),
    );
    sample.selectionPublication = performance.now();
    try {
      if (!same && !(await this.commit()))
        throw new Error(
          "入力を確定できません。元のsourceに戻って確認してください。",
        );
      if (mine !== this.token) {
        sample.invalid = "obsolete";
        return sample;
      }
      if (!same) this.interact({ editor: null, complex: null });
      const read = ++this.readToken,
        previous = restore ? this.local.get(path) : undefined;
      const start =
        startOverride ??
        Math.max(
          0,
          Math.floor(((previous?.top ?? 0) - GRID.header) / GRID.row) -
            GRID.overscan,
        );
      const count = Math.min(
        64,
        Math.ceil(
          Math.max(640, this.viewport?.clientHeight ?? 640) / GRID.row,
        ) + 10,
      );
      const r = await this.rpc<Projection>({
        kind: "select",
        path,
        start,
        count,
        token: read,
      });
      sample.ipcReturn = performance.now();
      sample.host = r.host;
      sample.engine = r.data.measurement;
      if (
        mine !== this.token ||
        read !== this.readToken ||
        this.surface.target !== path ||
        (same && queryVersion !== this.queryVersion) ||
        r.host.epoch !== this.surface.status.epoch
      ) {
        sample.invalid = "obsolete";
        return sample;
      }
      const p = r.data,
        saved = restore
          ? (this.local.get(p.source ?? "") ?? previous)
          : undefined;
      let row =
        saved?.selection.row ??
        Math.min(this.interaction.selection.row, Math.max(0, p.totalRows - 1));
      if (saved?.selection.id && p.source && after) {
        const located = await this.rpc<number | null>({
          kind: "locate",
          epoch: p.sessionEpoch,
          source: p.source,
          row: saved.selection.id,
        });
        if (mine !== this.token || read !== this.readToken) {
          sample.invalid = "obsolete";
          return sample;
        }
        if (located.data !== null) row = located.data;
      }
      const column = saved?.selection.field
        ? Math.max(
            0,
            p.columns.findIndex((c) => c.field.name === saved.selection.field),
          )
        : Math.max(
            0,
            Math.min(
              saved?.selection.column ?? this.interaction.selection.column,
              p.columns.length - 1,
            ),
          );
      const found = p.rows.find((r) => r.viewIndex === row);
      this.interact({
        selection: {
          row,
          column,
          id: found?.id ?? saved?.selection.id ?? null,
          field: p.columns[column]?.field.name ?? null,
        },
        anchor: saved?.anchor ?? null,
      });
      if (!after && !searching) this.historySource = p.source ?? p.table.source;
      sample.statePublication = performance.now();
      const paint = new Promise<void>((resolve) =>
        this.paintResolve.set(mine, resolve),
      );
      flushSync(() =>
        this.publish({
          projection: p,
          pending: false,
          queryPending: false,
          query: p.viewState.search,
        }),
      );
      if (this.viewport) {
        this.viewport.scrollTop = saved?.top ?? start * GRID.row;
        this.viewport.scrollLeft = saved?.left ?? 0;
      }
      // Paint is evidence, never authorization or a mutation completion lock.
      if (!same && this.evidence) {
        let deadline: ReturnType<typeof setTimeout>;
        try {
          await Promise.race([
            paint,
            new Promise<never>((_, reject) => {
              deadline = setTimeout(
                () =>
                  reject(
                    new Error(
                      "target paint opportunity was not observed within 1s",
                    ),
                  ),
                1000,
              );
            }),
          ]);
        } finally {
          clearTimeout(deadline!);
          this.paintResolve.delete(mine);
        }
      }
      return sample;
    } catch (e) {
      if (mine !== this.token) {
        sample.invalid = "obsolete";
        return sample;
      }
      sample.invalid = errorText(e);
      this.publish({
        projection: null,
        pending: false,
        queryPending: false,
        error: errorText(e),
      });
      throw e;
    }
  }
  committed(projection: Projection) {
    const sample = this.currentSample;
    if (
      !sample ||
      sample.token !== this.token ||
      projection !== this.surface.projection ||
      this.surface.pending
    )
      return;
    if (sample.paintOpportunity) return;
    sample.domCommit ??= performance.now();
    sample.reactCommit ??= sample.domCommit;
    this.viewport?.getBoundingClientRect();
    sample.layout = performance.now();
    requestAnimationFrame(() => {
      const current = this.currentFor(projection);
      if (
        sample.token === this.token &&
        current?.generation === projection.generation &&
        current.revision === projection.revision
      ) {
        sample.paintOpportunity ??= performance.now();
        sample.mountedRows =
          this.viewport?.querySelectorAll(".grid-row").length ?? 0;
      } else sample.invalid = "obsolete before paint";
      this.paintResolve.get(sample.token)?.();
      this.paintResolve.delete(sample.token);
    });
  }
  async refresh() {
    if (!this.surface.target || this.surface.queryPending) return;
    await this.selectTarget(
      this.surface.target,
      "after-operation",
      true,
      this.surface.projection?.rowStart ?? 0,
    );
  }
  async operation(
    intent: Record<string, unknown>,
    refresh = true,
  ): Promise<boolean> {
    if (this.surface.busy) return false;
    const target = this.surface.target,
      epoch = this.surface.status.epoch;
    this.publish({ busy: true, error: null });
    try {
      await this.rpc(intent);
      if (
        refresh &&
        target === this.surface.target &&
        epoch === this.surface.status.epoch &&
        !this.surface.pending
      )
        await this.refresh();
      return true;
    } catch (e) {
      if (epoch === this.surface.status.epoch) this.showError(e);
      return false;
    } finally {
      if (epoch === this.surface.status.epoch) this.publish({ busy: false });
    }
  }
  setSelection(row: number, column: number, extend = false) {
    const p = this.surface.projection;
    if (!p || this.surface.pending || this.surface.queryPending) return;
    const r = p.rows.find((r) => r.viewIndex === row),
      old = this.interaction.selection;
    this.interact({
      selection: {
        row,
        column,
        id: r?.id ?? null,
        field: p.columns[column]?.field.name ?? null,
      },
      anchor: extend ? (this.interaction.anchor ?? old) : null,
      focusIntent: this.interaction.focusIntent + 1,
    });
    this.historySource = p.source ?? p.table.source;
    const sample = this.currentSample;
    if (
      sample?.paintOpportunity &&
      !sample.firstAccepted &&
      sample.target === p.clicked &&
      sample.token === this.token
    )
      sample.firstAccepted = performance.now();
  }
  move(rows: number, columns: number, extend = false) {
    const p = this.surface.projection;
    if (!p) return;
    const s = this.interaction.selection;
    const row = Math.max(0, Math.min(p.totalRows - 1, s.row + rows)),
      column = Math.max(0, Math.min(p.columns.length - 1, s.column + columns));
    this.setSelection(row, column, extend);
    const v = this.viewport;
    if (!v) return;
    const top = row * GRID.row + GRID.header,
      left = column * GRID.column + GRID.identity;
    if (top < v.scrollTop + GRID.header) v.scrollTop = row * GRID.row;
    else if (top + GRID.row > v.scrollTop + v.clientHeight)
      v.scrollTop = top + GRID.row - v.clientHeight;
    if (left < v.scrollLeft + GRID.identity)
      v.scrollLeft = left - GRID.identity;
    else if (left + GRID.column > v.scrollLeft + v.clientWidth)
      v.scrollLeft = left + GRID.column - v.clientWidth;
    void this.ensureWindow();
  }
  async ensureWindow() {
    const p = this.surface.projection,
      v = this.viewport;
    if (
      !p ||
      !v ||
      this.surface.pending ||
      this.surface.queryPending ||
      this.interaction.editor
    )
      return;
    const start = Math.max(
        0,
        Math.floor((v.scrollTop - GRID.header) / GRID.row) - GRID.overscan,
      ),
      count = Math.min(64, Math.ceil(v.clientHeight / GRID.row) + 10);
    if (
      start >= p.rowStart &&
      Math.min(p.totalRows, start + count - GRID.overscan) <=
        p.rowStart + p.rows.length
    )
      return;
    const read = ++this.readToken,
      token = this.token;
    try {
      const r = await this.rpc<Projection>({
        kind: "select",
        epoch: p.sessionEpoch,
        path: p.clicked,
        start,
        count,
        token: read,
      });
      if (
        read !== this.readToken ||
        token !== this.token ||
        this.surface.pending ||
        this.surface.projection?.clicked !== p.clicked ||
        r.host.epoch !== this.surface.status.epoch
      )
        return;
      const row = r.data.rows.find(
        (r) => r.viewIndex === this.interaction.selection.row,
      );
      if (row)
        this.interact({
          selection: { ...this.interaction.selection, id: row.id },
        });
      this.publish({ projection: r.data });
    } catch (e) {
      if (read === this.readToken && token === this.token) this.showError(e);
    }
  }
  beginEditor = () => {
    const p = this.surface.projection,
      s = this.interaction.selection;
    if (
      !p?.source ||
      this.surface.busy ||
      this.surface.pending ||
      this.surface.queryPending ||
      this.interaction.editor
    )
      return;
    const r = p.rows.find((r) => r.viewIndex === s.row),
      cell = r?.cells[s.column],
      column = p.columns[s.column];
    if (!r || !cell?.editable || !column?.shape) return;
    if (
      column.shape.array ||
      column.shape.nullable ||
      ["custom", "flags", "enum"].includes(column.shape.category)
    ) {
      this.interact({
        complex: {
          epoch: p.sessionEpoch,
          source: p.source,
          row: r.id,
          root: column.field.name,
          path: [column.field.name],
          start: 0,
          inputIntent: this.inputIntent,
        },
      });
    } else
      this.interact({
        editor: {
          epoch: p.sessionEpoch,
          source: p.source,
          revision: p.revision,
          generation: p.generation,
          row: r.id,
          field: column.field.name,
          rowIndex: s.row,
          column: s.column,
          initial: cell.value?.kind === "null" ? "" : cell.display,
          display: cell.display,
        },
        complex: null,
      });
  };
  closeEditor = () => this.interact({ editor: null });
  collapseRange = () => this.interact({ anchor: null });
  async edit(editor: Editor, text: string) {
    if (text === editor.initial) {
      this.closeEditor();
      return true;
    }
    const ok = await this.operation({
      kind: "editText",
      epoch: editor.epoch,
      source: editor.source,
      revision: editor.revision,
      generation: editor.generation,
      row: editor.row,
      field: editor.field,
      text,
    });
    if (ok && this.interaction.editor === editor) {
      this.historySource = editor.source;
      this.closeEditor();
    }
    return ok;
  }
  closeComplex = () => this.interact({ complex: null });
  complexPath = (path: string[], start = 0, focusPath?: string[]) => {
    const c = this.interaction.complex;
    if (c)
      this.interact({
        complex: {
          ...c,
          path,
          start,
          focusPath,
          inputIntent: this.inputIntent,
        },
      });
  };
  async complexView(target: ComplexTarget): Promise<ComplexView | null> {
    const p = this.surface.projection;
    if (
      !p ||
      p.source !== target.source ||
      p.sessionEpoch !== target.epoch ||
      this.surface.pending
    )
      return null;
    const r = await this.rpc<ComplexView>({
      kind: "complex",
      ...this.scope(p),
      row: target.row,
      path: target.path,
      start: target.start,
      count: 64,
    });
    return r.data;
  }
  async complexEdit(
    view: ComplexView,
    path: string[],
    operation: Record<string, unknown>,
  ) {
    const current = this.surface.projection;
    // Navigation publishes the incoming identity before committing outgoing
    // typing. Its captured descriptor still targets the original source; Rust
    // checks that complete generation/revision context before applying it.
    if (
      !current ||
      current.source !== view.source ||
      current.sessionEpoch !== view.sessionEpoch ||
      current.revision !== view.revision ||
      current.generation !== view.generation
    )
      return false;
    this.historySource = view.source;
    return this.operation({
      kind: "complexEdit",
      epoch: view.sessionEpoch,
      source: view.source,
      revision: view.revision,
      generation: view.generation,
      row: view.row,
      path,
      operation,
    });
  }
  async schema(
    field: string,
    change: { typeName?: string; nullable?: boolean; array?: boolean },
  ) {
    const previous = this.surface.projection;
    if (!previous || !(await this.commit())) return;
    const p = this.currentFor(previous);
    if (!p) return;
    const f = p.columns.find((c) => c.field.name === field)?.field;
    if (!f) return;
    this.historySource = p.table.source;
    await this.operation({
      kind: "schema",
      epoch: p.sessionEpoch,
      source: p.table.source,
      revision: p.schemaRevision,
      generation: p.generation,
      field,
      typeName: change.typeName ?? null,
      nullable: change.nullable ?? (change.array ? false : f.nullable),
      array: change.array ?? (change.nullable ? false : f.array),
    });
  }
  focusSchema = () => {
    const source = this.surface.projection?.table.source ?? null;
    if (source === this.historySource) return;
    this.historySource = source;
    this.publish({});
  };
  history() {
    const p = this.surface.projection;
    return this.historySource === p?.table.source
      ? { undo: p?.schemaCanUndo, redo: p?.schemaCanRedo }
      : { undo: p?.canUndo, redo: p?.canRedo };
  }
  undo = async (redo = false) => {
    const source = this.historySource,
      previous = this.surface.projection;
    if (
      source &&
      previous &&
      (await this.commit()) &&
      this.currentFor(previous)
    )
      await this.operation({
        kind: "undo",
        epoch: previous.sessionEpoch,
        source,
        redo,
      });
  };
  save = async () => {
    const p = this.surface.projection;
    if (!p || !(await this.commit())) return;
    await this.saveIntent({
      kind: "save",
      epoch: p.sessionEpoch,
      table: p.table.name,
      source: p.source,
    });
  };
  saveAll = async () => {
    const epoch = this.surface.status.epoch;
    if (await this.commit()) await this.saveIntent({ kind: "saveAll", epoch });
  };
  private async saveIntent(intent: Record<string, unknown>) {
    if (this.surface.busy) return;
    const epoch = this.surface.status.epoch;
    this.publish({ busy: true, error: null });
    try {
      const r =
        await this.rpc<{ source: string; outcome: string; message: string }[]>(
          intent,
        );
      const failures = r.data.filter((r) => r.outcome !== "Success");
      if (this.surface.status.epoch === r.host.epoch) {
        await this.refresh();
        if (failures.length)
          this.showError(
            failures
              .map((f) => `${f.source}: ${f.outcome} ${f.message}`)
              .join("\n"),
          );
      }
    } catch (e) {
      if (epoch === this.surface.status.epoch) this.showError(e);
    } finally {
      if (epoch === this.surface.status.epoch) this.publish({ busy: false });
    }
  }
  validate = async () => {
    try {
      await this.rpc({ kind: "validate" });
    } catch (e) {
      this.showError(e);
    }
  };
  guard(): Promise<"saved" | "discard" | "cancel"> {
    if (this.guardRunning) return this.guardRunning;
    this.guardRunning = (async () => {
      if (!(await this.commit())) return "cancel" as const;
      if (!this.surface.inventory) return "saved" as const;
      const latest = await this.rpc<Inventory>({ kind: "inventory" });
      if (
        !latest.data.dirty.length &&
        !latest.data.uncertain.length &&
        !this.surface.status.recoveryRequired
      )
        return "saved" as const;
      const choice = await this.choose(
        "未保存の変更",
        "開いているsourceの変更をどう扱いますか。",
        ["Save All", "Don't Save", "Cancel"],
      );
      if (choice === "Cancel") return "cancel" as const;
      if (choice === "Don't Save") return "discard" as const;
      const saved = await this.rpc<
        { source: string; outcome: string; message: string }[]
      >({ kind: "saveAll", epoch: latest.host.epoch });
      const failures = saved.data.filter((s) => s.outcome !== "Success");
      if (failures.length) {
        this.showError(
          failures
            .map((s) => `${s.source}: ${s.outcome} ${s.message}`)
            .join("\n"),
        );
        return "cancel" as const;
      }
      return "saved" as const;
    })()
      .catch((e) => {
        this.showError(e);
        return "cancel" as const;
      })
      .finally(() => {
        this.guardRunning = null;
      });
    return this.guardRunning;
  }
  reloadProject = async () => {
    const inventory = this.surface.inventory;
    if (!inventory) return;
    const result = await this.guard();
    if (result !== "cancel")
      await this.openProject(inventory.root, result === "discard");
  };
  async compare(source?: string) {
    const previous = this.surface.projection;
    if (!previous || !(await this.commit())) return;
    const p = this.currentFor(previous);
    if (!p) return;
    const path = source ?? (p.dirty && p.source ? p.source : p.table.source);
    try {
      const r = await this.rpc<{
        identity: string;
        before: string;
        after: string;
      }>({ kind: "compare", epoch: p.sessionEpoch, source: path });
      if (this.currentFor(p))
        this.publish({
          comparison: { source: path, ...r.data, conflict: p.conflict },
        });
    } catch (e) {
      if (this.currentFor(p)) this.showError(e);
    }
  }
  closeCompare = () => this.publish({ comparison: null });
  overwrite = async () => {
    const compared = this.surface.comparison,
      epoch = this.surface.status.epoch;
    if (!compared) return;
    const choice = await this.choose(
      "Overwrite",
      "確認した外部sourceをこの保存候補で置き換えます。",
      ["Overwrite", "Cancel"],
    );
    if (choice !== "Overwrite" || this.surface.comparison !== compared) return;
    this.closeCompare();
    if (epoch !== this.surface.status.epoch || this.surface.busy) return;
    this.publish({ busy: true, error: null });
    try {
      const r = await this.rpc<WriteResult>({
        kind: "overwrite",
        epoch,
        source: compared.source,
        identity: compared.identity,
      });
      if (epoch !== this.surface.status.epoch) return;
      await this.refresh();
      if (r.data.outcome !== "Success")
        this.showError(`${r.data.source}: ${r.data.outcome} ${r.data.message}`);
    } catch (e) {
      if (epoch === this.surface.status.epoch) this.showError(e);
    } finally {
      if (epoch === this.surface.status.epoch) this.publish({ busy: false });
    }
  };
  reloadSource = async (source?: string) => {
    const p = this.surface.projection;
    if (!p) return;
    const path = source ?? p.source ?? p.table.source;
    const choice = await this.choose(
      "Reload source",
      "このsourceの未保存変更を破棄してdiskを読み直します。",
      ["Reload source", "Cancel"],
    );
    if (choice === "Reload source")
      await this.operation({
        kind: "reloadSource",
        epoch: p.sessionEpoch,
        source: path,
      });
  };
  toggleProblems = () => {
    this.publish({ problemsOpen: !this.surface.problemsOpen });
    if (this.surface.problemsOpen) void this.readProblems();
  };
  async readProblems() {
    const token = ++this.problemToken,
      epoch = this.surface.status.epoch;
    try {
      const r = await this.rpc<{
        problems: Diagnostic[];
        pending: boolean;
        generation: number;
      }>({ kind: "problems", epoch, start: 0, count: 256 });
      if (
        token !== this.problemToken ||
        r.host.epoch !== this.surface.status.epoch
      )
        return;
      this.publish({
        problems:
          !r.data.pending &&
          r.data.generation === this.surface.status.generation
            ? r.data.problems
            : [],
      });
    } catch (e) {
      if (token === this.problemToken) this.showError(e);
    }
  }
  async focusProblem(problem: Diagnostic) {
    if (
      problem.generation !== this.surface.status.generation ||
      this.surface.status.diagnosticsPending
    )
      return;
    try {
      await this.selectTarget(problem.source, "problem", true);
      const p = this.surface.projection;
      if (
        !p ||
        problem.generation !== p.generation ||
        p.clicked !== problem.source
      )
        return;
      const intent = this.interaction.focusIntent;
      const resolved = await this.rpc<ProblemTarget | null>({
        kind: "problemTarget",
        epoch: p.sessionEpoch,
        source: problem.source,
        generation: problem.generation,
        occurrence: problem.occurrence,
        path: problem.fieldPath,
      });
      const target = resolved.data;
      const current = () =>
        this.currentFor(p) &&
        this.surface.status.generation === problem.generation &&
        this.interaction.focusIntent === intent;
      if (!current() || !target) return;
      let at = target.viewIndex;
      if (at === null) {
        const answer = await this.choose(
          "Searchで非表示のrecord",
          "Searchを解除して、このProblemのrecordを表示します。",
          ["Clear Search", "Cancel"],
        );
        if (answer !== "Clear Search" || !current()) return;
        await this.rpc({
          kind: "search",
          epoch: p.sessionEpoch,
          source: target.source,
          text: "",
        });
        if (!current()) return;
        this.local.delete(target.source);
        await this.refresh();
        if (!current()) return;
        at = (
          await this.rpc<number | null>({
            kind: "locate",
            epoch: p.sessionEpoch,
            source: target.source,
            row: target.row,
          })
        ).data;
      }
      if (!current() || at === null) return;
      const column = Math.max(
        0,
        p.columns.findIndex((c) => c.field.name === target.field),
      );
      this.setSelection(at, column);
      const applied = this.interaction.focusIntent;
      const v = this.viewport;
      if (v) {
        v.scrollTop = Math.max(0, at * GRID.row - GRID.header);
        v.scrollLeft = Math.max(0, column * GRID.column - GRID.identity);
      }
      await this.ensureWindow();
      if (
        !this.currentFor(p) ||
        applied !== this.interaction.focusIntent ||
        this.surface.status.generation !== problem.generation
      )
        return;
      this.viewport?.focus();
      if (target.editorPath)
        this.interact({
          complex: {
            epoch: p.sessionEpoch,
            source: target.source,
            row: target.row,
            root: target.field!,
            path: target.editorPath,
            start: target.editorStart,
            focusPath: target.focusPath,
            inputIntent: this.inputIntent,
          },
        });
      else if (target.field) this.beginEditor();
    } catch (e) {
      if (
        this.surface.target === problem.source &&
        this.surface.status.generation === problem.generation
      )
        this.showError(e);
    }
  }
  search = (text: string) => {
    const p = this.surface.projection;
    if (!p?.source) return;
    if (this.queryTimer) clearTimeout(this.queryTimer);
    this.interact({ focusIntent: this.interaction.focusIntent + 1 });
    this.publish({ query: text, queryPending: true });
    this.queuedQuery = {
      source: p.source,
      epoch: p.sessionEpoch,
      path: p.clicked,
      text,
      version: ++this.queryVersion,
    };
    this.queryTimer = setTimeout(() => {
      this.queryTimer = null;
      void this.drainSearch();
    }, 150);
  };
  private async drainSearch() {
    if (this.queryRunning) return;
    this.queryRunning = true;
    try {
      while (this.queuedQuery) {
        const request = this.queuedQuery;
        this.queuedQuery = null;
        if (request.epoch !== this.surface.status.epoch) continue;
        try {
          if (!(await this.commit())) {
            if (
              request.version === this.queryVersion &&
              request.path === this.surface.target
            ) {
              this.publish({
                queryPending: false,
                query: this.surface.projection?.viewState.search ?? "",
              });
              this.showError("入力を確定してからSearchをやり直してください。");
            }
            continue;
          }
          await this.rpc({
            kind: "search",
            epoch: request.epoch,
            source: request.source,
            text: request.text,
          });
          if (
            request.version !== this.queryVersion ||
            request.path !== this.surface.target ||
            request.epoch !== this.surface.status.epoch
          )
            continue;
          this.local.delete(request.source);
          this.interact({
            selection: { ...this.interaction.selection, row: 0 },
            anchor: null,
          });
          await this.selectTarget(request.path, "search-result", false, 0);
        } catch (e) {
          if (
            request.version === this.queryVersion &&
            request.path === this.surface.target
          ) {
            this.publish({ queryPending: false, projection: null });
            this.showError(e);
          }
        }
      }
    } finally {
      this.queryRunning = false;
    }
  }
  async copyGrid() {
    const p = this.surface.projection,
      s = this.interaction.selection,
      a = this.interaction.anchor ?? s;
    if (
      !p?.source ||
      !s.id ||
      !a.id ||
      !s.field ||
      !a.field ||
      this.clipboardBusy ||
      this.surface.pending ||
      this.surface.queryPending
    )
      return;
    this.clipboardBusy = true;
    try {
      const r = await this.rpc<string>({
        kind: "copy",
        ...this.scope(p),
        anchor: a.id,
        focus: s.id,
        first: a.field,
        last: s.field,
      });
      await invoke("clipboard_text", { text: r.data });
    } catch (e) {
      this.showError(e);
    } finally {
      this.clipboardBusy = false;
    }
  }
  async pasteGrid(text?: string) {
    const p = this.surface.projection,
      s = this.interaction.selection;
    if (
      !p?.source ||
      !s.id ||
      !s.field ||
      this.clipboardBusy ||
      this.surface.pending ||
      this.surface.queryPending
    )
      return;
    this.clipboardBusy = true;
    try {
      const input =
        text ?? (await invoke<string>("clipboard_text", { text: null }));
      this.historySource = p.source;
      await this.operation({
        kind: "paste",
        ...this.scope(p),
        row: s.id,
        field: s.field,
        text: input,
      });
    } catch (e) {
      this.showError(e);
    } finally {
      this.clipboardBusy = false;
    }
  }
  async focusRow(
    id: string,
    intent: number,
    source: string,
    epoch: number,
    column = this.interaction.selection.column,
  ) {
    if (intent !== this.interaction.focusIntent) return;
    const r = await this.rpc<number | null>({
      kind: "locate",
      epoch,
      source,
      row: id,
    });
    if (
      r.data === null ||
      intent !== this.interaction.focusIntent ||
      source !== this.surface.projection?.source ||
      epoch !== this.surface.status.epoch
    )
      return;
    this.setSelection(r.data, column);
    const applied = this.interaction.focusIntent;
    if (this.viewport)
      this.viewport.scrollTop = Math.max(0, r.data * GRID.row - GRID.header);
    await this.ensureWindow();
    if (
      applied === this.interaction.focusIntent &&
      epoch === this.surface.status.epoch &&
      source === this.surface.projection?.source
    )
      this.viewport?.focus();
  }
  addRow = async (
    before: string | null = null,
    expected = this.surface.projection,
  ) => {
    if (!expected) return;
    if (!(await this.commit())) return;
    const p = this.currentFor(expected);
    if (!p?.source || this.surface.busy) return;
    const focus = this.interaction.focusIntent,
      source = p.source;
    this.historySource = source;
    this.publish({ busy: true, error: null });
    try {
      if (p.viewState.search)
        await this.rpc({
          kind: "search",
          epoch: p.sessionEpoch,
          source,
          text: "",
        });
      const r = await this.rpc<string>({
        kind: "addRow",
        ...this.scope(p),
        before,
      });
      this.local.delete(source);
      await this.refresh();
      if (focus === this.interaction.focusIntent)
        await this.focusRow(r.data, focus, source, p.sessionEpoch, 0);
    } catch (e) {
      if (p.sessionEpoch === this.surface.status.epoch) this.showError(e);
    } finally {
      if (p.sessionEpoch === this.surface.status.epoch)
        this.publish({ busy: false });
    }
  };
  async rowAction(action: string, p: Projection, row: string) {
    if (!(await this.commit()) || !p.source) return;
    const current = this.currentFor(p);
    if (!current) return;
    p = current;
    this.historySource = p.source;
    if (action === "above") return this.addRow(row, p);
    if (action === "below") {
      const next = await this.rpc<string | null>({
        kind: "nextRow",
        epoch: p.sessionEpoch,
        source: p.source,
        row,
      });
      if (this.currentFor(p)) return this.addRow(next.data, p);
      return;
    }
    const scope = { ...this.scope(p), row },
      focus = this.interaction.focusIntent;
    const ok = await this.operation(
      action === "up" || action === "down"
        ? { kind: "nudgeRow", ...scope, delta: action === "up" ? -1 : 1 }
        : { kind: "deleteRow", ...scope, restore: action === "restore" },
    );
    if (ok && p.source && (action === "up" || action === "down"))
      await this.focusRow(row, focus, p.source, p.sessionEpoch);
  }
  async columns(order: string[], p: Projection) {
    if (!(await this.commit())) return false;
    const current = this.currentFor(p);
    if (!current) return false;
    p = current;
    this.historySource = p.table.source;
    return this.operation({
      kind: "columns",
      epoch: p.sessionEpoch,
      source: p.table.source,
      revision: p.schemaRevision,
      generation: p.generation,
      order,
    });
  }
  async moveRow(row: string, targetIndex: number, p: Projection) {
    const current = this.currentFor(p);
    if (
      !current?.source ||
      current.revision !== p.revision ||
      current.generation !== p.generation ||
      current.viewState.search
    )
      return false;
    const target = current.rows.find((r) => r.viewIndex === targetIndex),
      from = p.rows.find((r) => r.id === row)?.viewIndex;
    if (
      !target ||
      target.pendingDelete ||
      from === undefined ||
      from === targetIndex
    )
      return false;
    let before: string | null = target.id;
    if (targetIndex > from)
      before = (
        await this.rpc<string | null>({
          kind: "nextRow",
          epoch: p.sessionEpoch,
          source: p.source,
          row: target.id,
        })
      ).data;
    if (
      !this.currentFor(p) ||
      this.surface.projection?.revision !== p.revision ||
      this.surface.projection.generation !== p.generation
    )
      return false;
    const focus = this.interaction.focusIntent;
    this.historySource = p.source;
    const ok = await this.operation({
      kind: "moveRow",
      ...this.scope(p),
      row,
      before,
    });
    if (ok) await this.focusRow(row, focus, p.source!, p.sessionEpoch);
    return ok;
  }
  async start() {
    await listen<Status>("workspace-status", (event) => {
      const next = event.payload,
        current = this.surface.status;
      if (
        next.epoch < current.epoch ||
        (next.epoch === current.epoch && next.generation < current.generation)
      )
        return;
      const samePaths = (a: string[], b: string[]) =>
        a.length === b.length && a.every((path, i) => path === b[i]);
      // Read replies can carry the same status. Republishing it would rerender
      // ordinary chrome, and an open Problems panel would request itself again.
      if (
        next.open === current.open &&
        next.epoch === current.epoch &&
        next.generation === current.generation &&
        next.recoveryRequired === current.recoveryRequired &&
        next.diagnosticsPending === current.diagnosticsPending &&
        next.problemCount === current.problemCount &&
        samePaths(next.dirty, current.dirty) &&
        samePaths(next.uncertain, current.uncertain)
      )
        return;
      this.publish({
        status: next,
        problems:
          next.diagnosticsPending || next.generation !== current.generation
            ? []
            : this.surface.problems,
      });
      if (this.surface.problemsOpen && !next.diagnosticsPending)
        void this.readProblems();
    });
    await listen(
      "exit-requested",
      () =>
        void this.guard()
          .then(async (result) => {
            if (result !== "cancel")
              await invoke("finish_exit", { discard: result === "discard" });
          })
          .catch(this.showError),
    );
  }
}
export const desktop = new Desktop();
