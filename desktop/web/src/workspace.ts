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
  FieldOperation,
  MigrationReview,
  SetResult,
  TypeProjection,
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
  migration?: { token: string; sources: string[] };
};
type WriteResult = { source: string; outcome: string; message: string };
type FieldScope = {token: string; sources: string[]; dirty: string[]; affectedRecords: number; destructive: boolean};
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
  tags: boolean;
}
export interface TagTarget { epoch: number; source: string; row: string; start: number; inputIntent: number; }
export interface TagView {
  sessionEpoch: number; source: string; row: string; revision: number; generation: number;
  entries: {index: number; text: string; valid: boolean; reason: string | null}[];
  start: number; total: number; editable: boolean; reason: string | null;
  known: string[]; partial: boolean;
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
export interface TemporaryInput {
  source: string;
  revision: number;
  generation: number;
  label: string;
  text: string;
  dirty: boolean;
  cancel: () => void;
}
interface HeldInput { source: string; label: string; text: string; reason: string; }
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
  tags: TagTarget | null;
  focusIntent: number;
}
export interface Surface {
  inventory: Inventory | null;
  projection: Projection | null;
  typeProjection: TypeProjection | null;
  target: string;
  pending: boolean;
  externalPending: boolean;
  busy: boolean;
  deliveryMutating: boolean;
  deliveryCapturing: boolean;
  error: string | null;
  status: Status;
  query: string;
  queryPending: boolean;
  problemsOpen: boolean;
  problems: Diagnostic[];
  choice: Choice | null;
  comparison: Compare | null;
  appearance: boolean;
  settingsOpen: boolean;
  settingsInputDirty: boolean;
  theme: Preference;
  heldInputs: HeldInput[];
  uncertainField: { token: string; epoch: number; target: string } | null;
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
  configDirty: false,
  configUncertain: false,
  configIdentity: "",
  uncertain: [],
  recoveryRequired: false,
  diagnosticsPending: false,
  problemCount: 0,
  externalVersion: 0,
  environmentError: null,
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
    typeProjection: null,
    target: "",
    pending: false,
    externalPending: false,
    busy: false,
    deliveryMutating: false,
    deliveryCapturing: false,
    error: null,
    status: initialStatus,
    query: "",
    queryPending: false,
    problemsOpen: false,
    problems: [],
    choice: null,
    comparison: null,
    appearance: false,
    settingsOpen: false,
    settingsInputDirty: false,
    theme: "system",
    heldInputs: [],
    uncertainField: null,
  };
  interaction: Interaction = {
    selection: { row: 0, column: 0, id: null, field: null },
    anchor: null,
    editor: null,
    complex: null,
    tags: null,
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
  private settingsHooks: {commit:()=>Promise<boolean>;dirty:()=>boolean;save:()=>Promise<void>} | null = null;
  private inputPreview: (() => TemporaryInput | null) | null = null;
  private clipboardBusy = false;
  private externalRunning = false;
  private externalPending = false;
  private guardRunning: Promise<"saved" | "discard" | "cancel"> | null = null;
  private unconfirmedWriteViews = new Set<string>();
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
  deliveryGate(mutating: boolean, capturing: boolean) {
    if(mutating !== this.surface.deliveryMutating || capturing !== this.surface.deliveryCapturing)
      this.publish({deliveryMutating:mutating,deliveryCapturing:capturing});
  }
  setTheme = async (theme: Preference) => {
    this.publish({ theme });
    try {
      await invoke("application_preferences", { theme });
    } catch (e) {
      this.showError(e);
    }
  };
  appearance = (open: boolean) => this.publish({ appearance: open });
  projectSettings = (open: boolean) => this.publish({settingsOpen:open});
  settingsTyping = (dirty: boolean) => {if(dirty!==this.surface.settingsInputDirty)this.publish({settingsInputDirty:dirty});};
  bindSettings(hooks: NonNullable<typeof this.settingsHooks>) {
    this.settingsHooks=hooks;
    return ()=>{if(this.settingsHooks===hooks)this.settingsHooks=null;};
  }
  async rpc<T>(intent: Record<string, unknown>): Promise<Reply<T>> {
    return invoke<Reply<T>>("workspace", {
      intent,
      epoch: intent.epoch ?? this.surface.status.epoch,
    });
  }
  async refreshInventory() {
    const epoch = this.surface.status.epoch;
    const reply = await this.rpc<Inventory>({kind:"inventory",epoch});
    if (epoch !== this.surface.status.epoch || reply.host.epoch !== epoch) return null;
    this.publish({inventory:reply.data});
    return reply.data;
  }
  bindEditor(commit: () => Promise<boolean>, inputPreview?: () => TemporaryInput | null) {
    this.commitActive = commit;
    this.inputPreview = inputPreview ?? null;
    return () => {
      if (this.commitActive === commit) { this.commitActive = null; this.inputPreview = null; }
    };
  }
  async commit() {
    return this.commitActive ? this.commitActive() : true;
  }
  protectWriteView(key: string, pending: boolean) {
    if (pending) this.unconfirmedWriteViews.add(key);
    else this.unconfirmedWriteViews.delete(key);
  }
  async guardSource(source: string, epoch: number, title = "Rename / Move前の未保存変更", ignoreTemporary = false) {
    const current = () => epoch === this.surface.status.epoch;
    const inventory = await this.rpc<Inventory>({kind: "inventory", epoch});
    if (!current()) return false;
    const input = ignoreTemporary ? null : this.inputPreview?.();
    const held = this.surface.heldInputs.some(input => input.source === source);
    if (inventory.data.recoveryRequired || inventory.data.uncertain.includes(source)) {
      this.showError("書き込み結果のRecheckが必要です。");
      return false;
    }
    const temporary = input?.source === source && input.dirty;
    if (temporary || held || inventory.data.dirty.includes(source)) {
      const choice = await this.choose(title, `${source} の変更をどう扱いますか。`, held ? ["Don't Save", "Cancel"] : ["Save", "Don't Save", "Cancel"]);
      if (!current() || choice === "Cancel") return false;
      if (choice === "Save") {
        if (!ignoreTemporary && this.inputPreview?.()?.source === source && !(await this.commit())) return false;
        if (!current() || this.surface.heldInputs.some(input => input.source === source)) return false;
        const saved = await this.rpc<WriteResult[]>({kind: "saveSource", epoch, source});
        const failures = saved.data.filter(result => result.outcome !== "Success");
        if (!current()) return false;
        if (failures.length) { this.showError(failures.map(result => `${result.outcome}: ${result.message}`).join("\n")); return false; }
      } else {
        await this.rpc({kind: "discardSource", epoch, source});
        if (!current()) return false;
        // The explicit guard authorizes only this physical source's input and
        // history. Other open inputs/drafts are independent authoring lifetimes.
        if (!ignoreTemporary && this.inputPreview?.()?.source === source) this.inputPreview()?.cancel();
        this.publish({heldInputs: this.surface.heldInputs.filter(input => input.source !== source)});
      }
      await this.refreshInventory();
      if (current() && (this.surface.projection?.source === source || this.surface.projection?.table.source === source)) await this.refresh();
    } else if (input?.source === source) input.cancel();
    return current();
  }
  async sourceMoved(source: string, destination: string, epoch: number) {
    if (epoch !== this.surface.status.epoch) return;
    const p = this.surface.projection;
    if (p?.source === source || p?.clicked === source) this.saveView();
    const view = this.local.get(source);
    if (view) { this.local.delete(source); this.local.set(destination, view); }
    if (this.historySource === source) this.historySource = destination;
    const clicked = this.surface.target;
    if (clicked === source) await this.selectTarget(destination, "source-path", true);
    else if (p?.source === source || p?.table.source === source) await this.selectTarget(clicked, "source-path", true);
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
    if (this.surface.heldInputs.length && !discard) throw new Error("保持中の入力の確認が必要です。");
    const r = await this.rpc<Inventory>({ kind: "open", path, discard });
    this.token++;
    this.readToken++;
    this.queryVersion++;
    this.queuedQuery = null;
    this.local.clear();
    this.unconfirmedWriteViews.clear();
    this.historySource = null;
    this.interact({
      selection: { row: 0, column: 0, id: null, field: null },
      anchor: null,
      editor: null,
      complex: null,
      tags: null,
      focusIntent: this.interaction.focusIntent + 1,
    });
    this.publish({
      inventory: r.data,
      projection: null,
      typeProjection: null,
      target: "",
      pending: false,
      externalPending: false,
      busy: false,
      deliveryMutating: false,
      deliveryCapturing: false,
      query: "",
      queryPending: false,
      error: null,
      comparison: null,
      uncertainField: null,
      problems: [],
      heldInputs: [],
      settingsOpen: false,
      settingsInputDirty: false,
      status: {
        ...initialStatus,
        open: true,
        epoch: r.host.epoch,
        generation: r.data.generation,
        dirty: r.data.dirty,
        configDirty: r.data.configDirty,
        configUncertain: r.data.configUncertain,
        configIdentity: r.data.configIdentity,
        uncertain: r.data.uncertain,
        recoveryRequired: r.data.recoveryRequired,
        externalVersion: r.data.externalVersion,
        environmentError: r.data.environmentError,
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
    const external = caseName === "external-change",
      after = caseName === "after-operation" || external,
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
    this.sampleForeground(sample, "input");
    this.samples.push(sample);
    if (!this.evidence && this.samples.length > 256) this.samples.shift();
    this.currentSample = sample;
    if (!same) this.interact({ focusIntent: this.interaction.focusIntent + 1 });
    flushSync(() =>
      this.publish({
        target: path,
        pending: external || !same || searching,
        externalPending: external,
        queryPending: searching,
        error: same ? this.surface.error : null,
        comparison: null,
        typeProjection: same ? this.surface.typeProjection : null,
      }),
    );
    sample.selectionPublication = performance.now();
    try {
      if (!same && !(await this.commit())) {
        const input=this.inputPreview?.();
        if(input?.dirty) {
          this.publish({heldInputs:[...this.surface.heldInputs,{source:input.source,label:input.label,text:input.text,reason:"入力を確定できませんでした。元のsourceで確認してください。"}]});
          input.cancel();
        }
        throw new Error(
          "入力を確定できません。元のsourceに戻って確認してください。",
        );
      }
      if (mine !== this.token) {
        sample.invalid = "obsolete";
        return sample;
      }
      if (!same) this.interact({ editor: null, complex: null, tags: null });
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
      const r = await this.rpc<Projection | TypeProjection>({
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
      if ("kind" in r.data) {
        const p = r.data;
        this.historySource = null;
        this.interact({editor:null,complex:null,anchor:null});
        sample.statePublication = performance.now();
        const paint = new Promise<void>(resolve => this.paintResolve.set(mine,resolve));
        flushSync(()=>this.publish({projection:null,typeProjection:p,pending:external,externalPending:external,query:"",queryPending:false}));
        if (!same && this.evidence) await this.waitSelectionPaint(mine,paint);
        return sample;
      }
      const p = r.data,
        saved = restore
          ? (this.local.get(p.source ?? "") ?? previous)
          : undefined;
      let row = Math.max(0, Math.min(
        saved?.selection.row ?? this.interaction.selection.row,
        Math.max(0, p.totalRows - 1),
      ));
      const visibleIdentity = p.rows.find((row) => row.id === saved?.selection.id);
      if (visibleIdentity) row = visibleIdentity.viewIndex;
      else if (saved?.selection.id && p.source && (after || searching)) {
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
        else if (searching) row = 0;
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
          id: found?.id ?? (row === saved?.selection.row ? saved?.selection.id : null) ?? null,
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
          typeProjection: null,
          pending: external,
          externalPending: external,
          queryPending: false,
          query: p.viewState.search,
        }),
      );
      if (this.viewport) {
        this.viewport.scrollTop = saved?.top ?? start * GRID.row;
        this.viewport.scrollLeft = saved?.left ?? 0;
      }
      // Paint is evidence, never authorization or a mutation completion lock.
      if (!same && this.evidence) await this.waitSelectionPaint(mine,paint);
      return sample;
    } catch (e) {
      if (mine !== this.token) {
        sample.invalid = "obsolete";
        return sample;
      }
      sample.invalid = errorText(e);
      this.publish({
        projection: null,
        typeProjection: null,
        pending: false,
        externalPending: false,
        queryPending: false,
        error: errorText(e),
      });
      throw e;
    }
  }
  private async waitSelectionPaint(token: number, paint: Promise<void>) {
    let deadline: ReturnType<typeof setTimeout>;
    try {
      await Promise.race([paint,new Promise<never>((_,reject)=>{
        deadline=setTimeout(()=>reject(new Error("target paint opportunity was not observed within 1s")),1000);
      })]);
    } finally {clearTimeout(deadline!);this.paintResolve.delete(token);}
  }
  private sampleForeground(sample: SelectionSample, boundary: "input" | "paint" | "interaction") {
    if (!this.evidence) return;
    const observed = { visibility: document.visibilityState, focused: document.hasFocus() };
    (sample.observations ??= {})[boundary] = observed;
    if (observed.visibility !== "visible" || !observed.focused)
      sample.invalid ??= `measurement unavailable: foreground at ${boundary}`;
  }
  currentType(p: TypeProjection) {
    const current=this.surface.typeProjection;
    return !this.surface.pending && this.surface.target===p.clicked && current===p && p.sessionEpoch===this.surface.status.epoch;
  }
  committedType(p: TypeProjection, element: HTMLElement) {
    const sample=this.currentSample;
    if(!sample || sample.token!==this.token || !this.currentType(p) || sample.paintOpportunity)return;
    sample.domCommit??=performance.now();sample.reactCommit??=sample.domCommit;
    element.getBoundingClientRect();sample.layout=performance.now();
    requestAnimationFrame(()=>{
      if(sample.token===this.token && this.currentType(p)) {sample.paintOpportunity??=performance.now();sample.mountedRows=element.querySelectorAll('.ant-table-row').length;this.sampleForeground(sample,"paint");}
      else sample.invalid="obsolete before paint";
      this.paintResolve.get(sample.token)?.();this.paintResolve.delete(sample.token);
    });
  }
  acceptedType(p: TypeProjection) {
    if(this.currentType(p) && this.currentSample?.target===p.clicked && !this.currentSample.firstAccepted) {
      this.currentSample.firstAccepted=performance.now();
      this.sampleForeground(this.currentSample,"interaction");
    }
  }
  migrationUncertain(token:string,epoch:number,target:string) {
    if(epoch===this.surface.status.epoch)this.publish({uncertainField:{token,epoch,target}});
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
        this.sampleForeground(sample, "paint");
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
    ) {
      sample.firstAccepted = performance.now();
      this.sampleForeground(sample, "interaction");
    }
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
    if (this.interaction.tags) {
      const target = this.interaction.tags, intent = this.inputIntent;
      void this.commit().then(ok => {
        if(ok && this.interaction.tags === target && this.currentFor(p) && this.inputIntent === intent) {
          this.closeTags(); this.beginEditor();
        }
      });
      return;
    }
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
      current.revision !== view.revision
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
  closeTags = () => this.interact({tags: null});
  async openTags(p: Projection, row: string) {
    if (!(await this.commit())) return;
    const current = this.currentFor(p);
    if (!current?.source || this.surface.pending) return;
    this.interact({editor: null, complex: null, tags: {epoch: current.sessionEpoch, source: current.source, row, start: 0, inputIntent: this.inputIntent}});
  }
  tagsPage(start: number) {
    const target = this.interaction.tags;
    if(target) this.interact({tags: {...target, start, inputIntent: this.inputIntent}});
  }
  async tagView(target: TagTarget): Promise<TagView | null> {
    const p = this.surface.projection;
    if (!p || p.source !== target.source || p.sessionEpoch !== target.epoch || this.surface.pending) return null;
    return (await this.rpc<TagView>({kind: "tags", ...this.scope(p), row: target.row, start: target.start})).data;
  }
  async tagEdit(view: TagView, operation: Record<string, unknown>) {
    const current = this.surface.projection;
    if(!current || current.source !== view.source || current.sessionEpoch !== view.sessionEpoch || current.revision !== view.revision) return false;
    this.historySource = view.source;
    return this.operation({kind: "tagEdit", epoch: view.sessionEpoch, source: view.source, revision: view.revision, generation: view.generation, row: view.row, operation});
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
  async fieldOperation(operation: FieldOperation, expected: Projection, ignoreTemporary = false) {
    const epoch = expected.sessionEpoch;
    if (this.surface.busy || this.surface.status.recoveryRequired || this.surface.uncertainField || epoch !== this.surface.status.epoch) throw new Error("現在この操作を開始できません。未確定の結果はRecheckで確認してください。");
    if (!ignoreTemporary && !(await this.commit())) throw new Error("入力を確定できません。");
    const p = this.surface.projection;
    if (!p || p.clicked !== expected.clicked || p.sessionEpoch !== epoch) throw new Error("対象が変更されています。もう一度確定してください。");
    const context = { epoch, source: p.table.source, revision: p.schemaRevision, generation: p.generation };
    this.publish({busy: true, error: null});
    let scope: FieldScope | null = null;
    let attempted = false;
    try {
      scope = (await this.rpc<FieldScope>({kind:"fieldScope", ...context, operation})).data;
      if (epoch !== this.surface.status.epoch) throw new Error("Projectが変更されています。");
      for (const source of scope.sources) {
        if (!(await this.guardSource(source, epoch, "構造変更前の未保存変更", ignoreTemporary))) return null;
      }
      if (scope.dirty.length) throw new Error("未保存変更を処理しました。変更後のTableで、もう一度確定してください。");
      if (scope.destructive) {
        let answer: string;
        do {
          answer = await this.choose("Drop Field", `${operation.kind === "drop" ? operation.field : "Field"}を${scope.affectedRecords}件のrecord、${scope.sources.length}個のsourceから削除します。`, ["Delete", "Compare", "Cancel"]);
          if (answer === "Compare") {
            await this.migrationCompare(scope.token,scope.sources[0],scope.sources);
            const shown = this.surface.comparison;
            if (shown) await new Promise<void>(resolve => { const unsubscribe=this.subscribe(() => {if(!this.surface.comparison){unsubscribe();resolve();}}); });
          }
        } while(answer === "Compare" && epoch === this.surface.status.epoch);
        if(answer !== "Delete") return null;
      }
      if (epoch !== this.surface.status.epoch) return null;
      attempted = true;
      this.protectWriteView(`field:${scope.token}`,true);
      const [review,result] = (await this.rpc<[MigrationReview,SetResult]>({kind:"fieldOperation", ...context, request:{token:scope.token,operation,authorizeDestructive:scope.destructive}})).data;
      this.protectWriteView(`field:${scope.token}`,false);
      await this.refreshInventory();
      if(this.surface.target === expected.clicked && !this.surface.pending) await this.refresh();
      if(result.outcome !== "Success") throw new Error(`${result.outcome}: ${result.message}`);
      return review;
    } catch(e) {
      if (scope && attempted && typeof e === "object" && e && "code" in e) this.protectWriteView(`field:${scope.token}`,false);
      if (scope && attempted && this.unconfirmedWriteViews.has(`field:${scope.token}`)) {
        this.publish({uncertainField:{token:scope.token,epoch,target:expected.clicked}});
        this.showError(`結果を確認できません。自動再試行は止めています。${errorText(e)}`);
      }
      throw e;
    } finally {
      if(epoch === this.surface.status.epoch) this.publish({busy:false});
    }
  }
  async fieldAction(operation: FieldOperation, expected: Projection) {
    const intent=this.inputIntent;
    try {
      const review=await this.fieldOperation(operation,expected);
      const name=review?.command.declaration?.name;
      if(name && this.surface.target===expected.clicked && this.inputIntent===intent) {
        const column=this.surface.projection?.columns.findIndex(c=>c.field.name===name) ?? -1, viewport=this.viewport;
        if(viewport && column>=0) {
          const left=GRID.identity+column*GRID.column;
          if(left<viewport.scrollLeft)viewport.scrollLeft=left;
          else if(left+GRID.column>viewport.scrollLeft+viewport.clientWidth)viewport.scrollLeft=left+GRID.column-viewport.clientWidth;
        }
        requestAnimationFrame(()=>requestAnimationFrame(() => {
          if(this.surface.target===expected.clicked && this.inputIntent===intent) this.viewport?.querySelector<HTMLButtonElement>(`.column[data-field="${CSS.escape(name)}"] .field-name-button`)?.focus();
        }));
      }
    } catch(e) { this.showError(e); }
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
    if(this.surface.settingsOpen) {await this.settingsHooks?.save();return;}
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
    if (await (this.settingsHooks?.commit()??Promise.resolve(true)) && await this.commit()) await this.saveIntent({ kind: "saveAll", epoch });
  };
  private async saveIntent(intent: Record<string, unknown>) {
    if (this.surface.busy) return;
    if (this.surface.deliveryCapturing) {this.showError("保存済みinputの取得中です。完了後にSaveを再操作してください。");return;}
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
    if(this.surface.deliveryMutating) {this.showError("実行中のBuild / Publishが確定してからProjectを切り替えるか終了してください。");return Promise.resolve("cancel");}
    if (this.guardRunning) return this.guardRunning;
    this.guardRunning = (async () => {
      if (this.unconfirmedWriteViews.size) {
        this.showError("書き込みの完了またはRecheckで結果を確認してから続けてください。");
        return "cancel" as const;
      }
      if (this.surface.heldInputs.length) {
        const choice = await this.choose("保持中の入力と未保存の変更", "保持中の入力は変更後のsourceへ安全に適用できません。保持中の入力とsourceの未保存変更を破棄して続けますか。", ["Don't Save", "Cancel"]);
        return choice === "Don't Save" ? "discard" as const : "cancel" as const;
      }
      if (!(await this.commit())) return "cancel" as const;
      if (!this.surface.inventory) return "saved" as const;
      const latest = await this.rpc<Inventory>({ kind: "inventory" });
      if (
        !latest.data.dirty.length &&
        !latest.data.configDirty && !latest.data.configUncertain && !this.settingsHooks?.dirty() &&
        !latest.data.uncertain.length &&
        !latest.data.recoveryRequired
      )
        return "saved" as const;
      const choice = await this.choose(
        "未保存の変更",
        "sourceとProject Settingsの変更をどう扱いますか。",
        ["Save All", "Don't Save", "Cancel"],
      );
      if (choice === "Cancel") return "cancel" as const;
      if (choice === "Don't Save") return "discard" as const;
      if (!(await (this.settingsHooks?.commit()??Promise.resolve(true)))) return "cancel" as const;
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
      // Save All advances drafts, not uncertain creation or structural outcomes.
      // Read current protection again before allowing a Project session to end.
      const remaining = await this.rpc<Inventory>({ kind: "inventory", epoch: latest.host.epoch });
      if (remaining.data.dirty.length || remaining.data.configDirty || remaining.data.configUncertain || remaining.data.uncertain.length || remaining.data.recoveryRequired) {
        this.showError("未確定の書き込みが残っています。Recheckで結果を確認してください。");
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
  async migrationCompare(token: string, source: string, sources: string[]) {
    const epoch=this.surface.status.epoch;
    const r=await this.rpc<[string,string]>({kind:"migrationCompare",epoch,token,source});
    if(epoch===this.surface.status.epoch) this.publish({comparison:{source,identity:"",before:r.data[0],after:r.data[1],conflict:false,migration:{token,sources}}});
  }
  async recoverMigration(id: string, restoreOld: boolean) {
    const epoch=this.surface.status.epoch;
    if(restoreOld && await this.choose("Restore OLD", "このMigrationが書いたとfreshに確認できるsourceだけを、保存したOLD bytesへ戻します。",["Restore OLD","Cancel"])!=="Restore OLD") return;
    this.publish({busy:true,error:null});
    try {
      await this.rpc({kind:"migrationRecovery",epoch,id,restoreOld,authorized:restoreOld});
      if(epoch!==this.surface.status.epoch) return;
      await this.refreshInventory(); await this.refresh();
    } catch(e) {if(epoch===this.surface.status.epoch)this.showError(e);}
    finally {if(epoch===this.surface.status.epoch)this.publish({busy:false});}
  }
  async recheckFieldOperation() {
    const attempt=this.surface.uncertainField;
    if(!attempt || attempt.epoch!==this.surface.status.epoch) return;
    this.publish({busy:true});
    try {
      const reply=await this.rpc<SetResult>({kind:"migrationResult",epoch:attempt.epoch,token:attempt.token});
      this.protectWriteView(`field:${attempt.token}`,false);
      this.publish({uncertainField:null,error:reply.data.outcome==="Success" ? null : `${reply.data.outcome}: ${reply.data.message}`});
      await this.refreshInventory(); if(!this.surface.pending) await this.refresh();
    } catch(e) {this.showError(e);}
    finally {if(attempt.epoch===this.surface.status.epoch)this.publish({busy:false});}
  }
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
        !!this.currentFor(p) &&
        this.surface.status.generation === problem.generation &&
        this.interaction.focusIntent === intent;
      if (!current() || !target) return;
      await this.focusResolved(p,target,current,()=>this.surface.status.generation===problem.generation);
    } catch (e) {
      if (
        this.surface.target === problem.source &&
        this.surface.status.generation === problem.generation
      )
        this.showError(e);
    }
  }
  async focusBuildProblem(id:number,index:number,source:string) {
    const epoch=this.surface.status.epoch;
    try {
      await this.selectTarget(source,"build-problem",true);
      const intent=this.interaction.focusIntent;
      const resolved=await this.rpc<{source:string;target:ProblemTarget|null;generation:number}>({kind:"deliveryProblemTarget",epoch,id,index});
      const p=this.surface.projection;
      const valid=()=>epoch===this.surface.status.epoch&&this.surface.status.generation===resolved.data.generation;
      const current=()=>!!p&&!!this.currentFor(p)&&this.interaction.focusIntent===intent&&valid();
      if(!p||!resolved.data.target||!current())return;
      await this.focusResolved(p,resolved.data.target,current,valid);
    } catch(error) {if(epoch===this.surface.status.epoch)this.showError(error);}
  }
  private async focusResolved(p:Projection,target:ProblemTarget,current:()=>boolean,valid:()=>boolean) {
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
        !valid()
      )
        return;
      this.viewport?.focus();
      if (target.tags) await this.openTags(this.surface.projection!, target.row);
      else if (target.editorPath)
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
    if (action === "tags") return this.openTags(p, row);
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
        next.configDirty === current.configDirty && next.configUncertain === current.configUncertain && next.configIdentity === current.configIdentity &&
        next.externalVersion === current.externalVersion &&
        next.environmentError === current.environmentError &&
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
      if (next.epoch === current.epoch && next.externalVersion !== current.externalVersion)
        this.externalRefresh();
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
  private externalRefresh() {
    this.externalPending = true;
    // Once an external observation is known, the old view loses interaction
    // authority immediately. Its mounted input keeps temporary typing until a
    // fresh projection/error resolves; background refresh never commits it.
    const element = document.activeElement as HTMLElement | null,
      input = this.inputIntent,
      inputPreview = this.inputPreview;
    if (this.surface.target) this.publish({ pending: true, externalPending: true });
    if (this.externalRunning) return;
    this.externalRunning = true;
    void (async () => {
      let operationEpoch = this.surface.status.epoch;
      try {
        while (this.externalPending) {
          this.externalPending = false;
          const epoch = this.surface.status.epoch;
          operationEpoch = epoch;
          const inventory = await this.rpc<Inventory>({ kind: "inventory", epoch });
          if (epoch !== this.surface.status.epoch) continue;
          this.publish({ inventory: inventory.data });
          if (this.surface.status.environmentError) {
            await this.recheckInput(inputPreview, epoch);
            this.publish({ pending: false, externalPending: false, projection: null, typeProjection:null, error: this.surface.status.environmentError });
          } else if (this.surface.target) {
            const selected = await this.selectTarget(this.surface.target, "external-change", true);
            await this.recheckInput(inputPreview, epoch);
            if (this.currentSample === selected && !selected.invalid && epoch === this.surface.status.epoch && !this.externalPending)
              this.publish({ pending: false, externalPending: false });
          }
        }
        if (operationEpoch === this.surface.status.epoch && input === this.inputIntent && element?.isConnected && (this.surface.projection||this.surface.typeProjection))
          element.focus({ preventScroll: true });
      } catch (error) {
        if (operationEpoch === this.surface.status.epoch) {
          this.publish({ projection: null, typeProjection:null, pending: false, externalPending: false });
          this.showError(error);
          await this.recheckInput(inputPreview, operationEpoch);
        }
      }
      finally { this.externalRunning = false; if (this.externalPending) this.externalRefresh(); }
    })();
  }
  private async recheckInput(preview: typeof this.inputPreview, epoch: number) {
    if (!preview || preview !== this.inputPreview || epoch !== this.surface.status.epoch) return;
    const input = preview();
    if (!input) return;
    let observed: Reply<{current: boolean; reason: string | null}>;
    try { observed = await this.rpc({kind: "authoringState", epoch, source: input.source, revision: input.revision, generation: input.generation}); }
    catch (error) { if (epoch === this.surface.status.epoch) this.showError(error); return; }
    if (observed.data.current || preview !== this.inputPreview || epoch !== this.surface.status.epoch) return;
    const latest = preview();
    if (latest?.dirty) this.publish({heldInputs: [...this.surface.heldInputs, {source: latest.source, label: latest.label, text: latest.text, reason: observed.data.reason ?? "authoring context changed"}]});
    latest?.cancel();
  }
  discardHeldInput = (index: number) => this.publish({heldInputs: this.surface.heldInputs.filter((_input, at) => at !== index)});
  copyHeldInput = async (index: number) => {
    const input = this.surface.heldInputs[index];
    if (input) await invoke("clipboard_text", {text: input.text});
  };
}
export const desktop = new Desktop();
