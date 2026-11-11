import React, { useEffect, useMemo, useSyncExternalStore } from "react";
import type {
  CellDisplay,
  DisplayInput,
  DisplayJob,
  DisplayReply,
  RowDisplay,
} from "../core/display-model";

import { checkDisplayInput } from "../core/display-model";

const cacheLimit = 2048;
const cacheUnits = 4 * 1024 * 1024;
function rowSize(cells: RowDisplay) {
  return cells.reduce(
    (size, cell) =>
      size + 48 + (cell.text?.length ?? 0) + (cell.error?.length ?? 0),
    0,
  );
}
export class GridDisplay {
  private worker?: Worker;
  private queue = new Set<number>();
  private pending: number[] = [];
  private cache = new Map<number, RowDisplay>();
  private units = 0;
  private copyRows = new Set<number>();
  private copyTooLarge = false;
  private ready = false;
  private program = -1;
  private disabled = new Set<number>();
  private timer?: ReturnType<typeof setTimeout>;
  private scheduled = false;
  private listeners = new Map<number, Set<() => void>>();
  private error?: RowDisplay;
  private active = false;

  constructor(private input: DisplayInput) {}
  open() {
    this.active = true;
    try {
      checkDisplayInput(this.input);
      this.start();
    } catch (error) {
      this.fail(String(error));
    }
  }
  private start() {
    try {
      this.worker = new Worker(
        new URL("../core/display-worker.ts", import.meta.url),
        { type: "module" },
      );
      const current = this.worker;
      current.onmessage = ({ data }: MessageEvent<DisplayReply>) => {
        if (current !== this.worker) return;
        if (data.type === "error") {
          this.fail(data.message);
          return;
        }
        if (data.type === "started") {
          this.program = data.program;
          this.watch(4000);
          return;
        }
        clearTimeout(this.timer);
        if (data.type === "rows") {
          data.indices.forEach((row, i) => {
            const previous = this.cache.get(row);
            if (previous) this.units -= rowSize(previous);
            this.cache.set(row, data.values[i]);
            this.units += rowSize(data.values[i]);
            this.listeners.get(row)?.forEach((notify) => notify());
          });
          this.pending = [];
          this.evict();
        }
        this.program = -1;
        this.ready = true;
        this.flush();
      };
      current.onerror = () => {
        if (this.worker === current)
          this.fail("表示計算のWorkerを読み込めません。");
      };
      this.watch(10_000);
      this.send({
        type: "init",
        input: this.input,
        disabled: [...this.disabled],
      });
    } catch (e) {
      this.fail(String(e));
    }
  }
  private evict() {
    for (const [row, cells] of this.cache) {
      if (this.cache.size <= cacheLimit && this.units <= cacheUnits) return;
      if (this.listeners.has(row) || this.copyRows.has(row)) continue;
      this.cache.delete(row);
      this.units -= rowSize(cells);
    }
    if (this.units > cacheUnits && this.copyRows.size && !this.copyTooLarge) {
      this.copyTooLarge = true;
      for (const row of this.copyRows) {
        if (this.listeners.has(row)) continue;
        this.queue.delete(row);
        const cells = this.cache.get(row);
        if (cells) {
          this.cache.delete(row);
          this.units -= rowSize(cells);
        }
      }
    }
  }
  prepareCopy(rows: number[]) {
    if (rows.length > cacheLimit) return "too-large";
    if (
      this.copyRows.size !== rows.length ||
      rows.some((r) => !this.copyRows.has(r))
    ) {
      for (const row of this.copyRows)
        if (!this.listeners.has(row)) this.queue.delete(row);
      this.copyRows = new Set(rows);
      this.copyTooLarge = false;
    }
    if (this.copyTooLarge) return "too-large";
    const missing = rows.filter((row) => !this.read(row));
    missing.forEach((row) => this.request(row));
    if (missing.length) return "pending";
    this.copyRows.clear();
    return "ready";
  }
  private watch(ms: number) {
    clearTimeout(this.timer);
    this.timer = setTimeout(() => {
      if (this.program < 0 || this.disabled.has(this.program)) {
        this.fail("表示計算の応答時間を超えました。");
        return;
      }
      this.disabled.add(this.program);
      this.worker?.terminate();
      this.ready = false;
      this.program = -1;
      this.pending.forEach((row) => this.queue.add(row));
      this.pending = [];
      this.start();
    }, ms);
  }
  private send(job: DisplayJob) {
    this.worker?.postMessage(job);
  }
  private fail(message: string) {
    clearTimeout(this.timer);
    this.worker?.terminate();
    this.worker = undefined;
    this.queue.clear();
    this.pending = [];
    this.error = this.input.columns.map((col) => ({
      error: message,
      text: col.kind === "function" ? "計算エラー" : undefined,
    }));
    this.listeners.forEach((set) => set.forEach((notify) => notify()));
  }
  close() {
    this.active = false;
    this.worker?.terminate();
    this.worker = undefined;
    clearTimeout(this.timer);
    this.pending = [];
    this.ready = false;
    this.cache.clear();
    this.units = 0;
    this.copyRows.clear();
    this.copyTooLarge = false;
    this.queue.clear();
    this.error = undefined;
  }
  read(row: number) {
    return this.error ?? this.cache.get(row);
  }
  request(row: number) {
    if (!this.read(row) && !this.pending.includes(row)) this.queue.add(row);
    if (!this.scheduled) {
      this.scheduled = true;
      queueMicrotask(() => {
        this.scheduled = false;
        this.flush();
      });
    }
  }
  private flush() {
    if (!this.active || !this.ready || !this.queue.size) return;
    this.ready = false;
    this.pending = [];
    for (const row of this.queue) {
      this.queue.delete(row);
      this.pending.push(row);
      if (this.pending.length === 12) break;
    }
    this.watch(4000);
    this.send({ type: "rows", indices: this.pending });
  }
  subscribe(row: number, notify: () => void) {
    const listeners = this.listeners.get(row) ?? new Set();
    listeners.add(notify);
    this.listeners.set(row, listeners);
    this.request(row);
    return () => {
      listeners.delete(notify);
      if (!listeners.size) {
        this.listeners.delete(row);
        if (!this.copyRows.has(row)) this.queue.delete(row);
      }
    };
  }
}
export function useGridDisplay(input?: DisplayInput) {
  const display = useMemo(() => input && new GridDisplay(input), [input]);
  useEffect(() => {
    display?.open();
    return () => display?.close();
  }, [display]);
  return display;
}
export function DisplayCell({
  display,
  row,
  col,
  derived,
  children,
  edit,
}: {
  display: GridDisplay;
  row: number;
  col: number;
  derived?: boolean;
  children?: React.ReactNode;
  edit: () => void;
}) {
  const subscribe = useMemo(
    () => (fn: () => void) => display.subscribe(row, fn),
    [display, row],
  );
  const result: CellDisplay | undefined = useSyncExternalStore(
    subscribe,
    () => display.read(row)?.[col],
  );
  const style = result?.style;
  const css: React.CSSProperties = {
    fontWeight:
      style?.bold === undefined ? undefined : style.bold ? "bold" : "normal",
    fontStyle:
      style?.italic === undefined
        ? undefined
        : style.italic
          ? "italic"
          : "normal",
    color: style?.color,
    backgroundColor: style?.backgroundColor,
  };
  return (
    <div
      className="display-cell"
      style={
        {
          ...css,
          "--display-color": style?.color,
          "--display-bg": style?.backgroundColor,
          "--display-weight": css.fontWeight,
          "--display-font": css.fontStyle,
        } as React.CSSProperties
      }
      data-ink={style?.color}
      data-fill={style?.backgroundColor}
      data-weight={style?.bold}
      data-italic={style?.italic}
    >
      {derived ? (
        <span className="display-text">{result?.text ?? "…"}</span>
      ) : (
        children
      )}
      {result?.error && (
        <button
          type="button"
          className="display-error"
          title={result.error}
          aria-label={`表示定義を修正: ${result.error}`}
          onClick={edit}
        >
          !
        </button>
      )}
    </div>
  );
}
