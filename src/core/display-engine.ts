import variant from "@jitl/quickjs-asmjs-mjs-release-sync";
import {
  newQuickJSWASMModuleFromVariant,
  type QuickJSContext,
  type QuickJSHandle,
  type QuickJSRuntime,
} from "quickjs-emscripten-core";
import type {
  CellDisplay,
  DisplayInput,
  DisplayProgram,
  RowDisplay,
} from "./display-model";
import { encode } from "./wire";

interface Compiled {
  cold: boolean;
  context: QuickJSContext;
  invoke: QuickJSHandle;
  runtime: QuickJSRuntime;
}
const mib = 1024 * 1024;
const moduleReady = () => newQuickJSWASMModuleFromVariant(variant);
export class DisplayEngine {
  private compiled = new Map<number, Compiled>();
  private failures = new Map<number, string>();
  private deadline = 0;
  private constructor(
    private module: Awaited<ReturnType<typeof moduleReady>>,
    private input: DisplayInput,
    private wire: string,
    private guest: string,
  ) {}
  static async create(
    input: DisplayInput,
    guest: string,
    disabled: number[] = [],
  ) {
    if (
      input.records.length > 100_000 ||
      input.programs.length > 32 ||
      input.columns.length > 256
    )
      throw new Error(
        "表示計算の上限（10万行・32定義・256列）を超えています。",
      );
    const wire = encode(input.records);
    if (wire.length > 8 * mib)
      throw new Error("表示計算のテーブル入力が8 Mi文字を超えています。");
    const engine = new DisplayEngine(await moduleReady(), input, wire, guest);
    for (const index of disabled)
      engine.failures.set(index, "時間上限のため停止しました。");
    return engine;
  }
  private compile(index: number, program: DisplayProgram) {
    const runtime = this.module.newRuntime();
    // All per-definition runtimes share the module's allocator. Bound their
    // combined guest heaps as well as the allocation available to a single script.
    runtime.setMemoryLimit(
      Math.min(24 * mib, Math.floor((48 * mib) / this.input.programs.length)),
    );
    runtime.setMaxStackSize(256 * 1024);
    runtime.setInterruptHandler(() => performance.now() >= this.deadline);
    this.deadline = performance.now() + 1500;
    const context = runtime.newContext();
    let keep = false;
    try {
      const setup = context.evalCode(this.guest, "masterdata-display.js", {
        type: "global",
      });
      if (setup.error) {
        setup.error.dispose();
        throw new Error("表示VMを初期化できません。");
      }
      const args = [
        this.wire,
        JSON.stringify(this.input.columns),
        program.script,
        program.kind,
      ].map((s) => context.newString(s));
      try {
        const result = context.callFunction(
          setup.value,
          context.undefined,
          ...args,
        );
        if (result.error) {
          result.error.dispose();
          throw new Error("表示VMの初期化上限（メモリ・時間）を超えました。");
        }
        if (context.typeof(result.value) !== "function") {
          const message = context.getString(result.value);
          result.value.dispose();
          throw new Error(message);
        }
        this.compiled.set(index, {
          context,
          runtime,
          invoke: result.value,
          cold: true,
        });
        keep = true;
      } finally {
        for (const arg of args) arg.dispose();
        setup.value.dispose();
      }
    } finally {
      if (!keep) {
        context.dispose();
        runtime.dispose();
      }
    }
  }
  private release(index: number) {
    const compiled = this.compiled.get(index);
    if (!compiled) return;
    compiled.invoke.dispose();
    compiled.context.dispose();
    compiled.runtime.dispose();
    this.compiled.delete(index);
  }
  evaluate(
    indices: number[],
    started: (program: number) => void,
  ): RowDisplay[] {
    const { programs, columns } = this.input;
    const firstFunction = columns.findIndex((c) => c.kind === "function");
    const values = indices.map(() => columns.map((): CellDisplay => ({})));
    let functionIndex = firstFunction;
    for (const [index, program] of programs.entries()) {
      const first = program.kind === "rule" ? 0 : functionIndex++;
      const end = program.kind === "rule" ? columns.length : first + 1;
      started(index);
      let failure = this.failures.get(index) ?? program.error;
      if (!failure && !this.compiled.has(index)) {
        try {
          if (program.script.length > 32_768)
            throw new Error("コードは32768文字以内です。");
          this.compile(index, program);
        } catch (e) {
          failure = e instanceof Error ? e.message : String(e);
          this.failures.set(index, failure);
        }
      }
      const budget = performance.now() + 1200;
      for (const [r, row] of indices.entries()) {
        for (let col = first; col < end; col++) {
          let result: CellDisplay = {};
          if (!failure) {
            try {
              const compiled = this.compiled.get(index)!;
              this.deadline = Math.min(
                performance.now() + (compiled.cold ? 300 : 50),
                budget,
              );
              result = this.cell(compiled, row, col);
              compiled.cold = false;
            } catch (error) {
              failure = error instanceof Error ? error.message : String(error);
              this.failures.set(index, failure);
              this.release(index);
            }
          }
          const dest = values[r][col];
          if (result.text !== undefined && program.kind === "function")
            dest.text = result.text;
          if (result.style) dest.style = { ...dest.style, ...result.style };
          const problem = failure || result.error;
          if (problem) {
            dest.error = [dest.error, `${program.name}: ${problem}`]
              .filter(Boolean)
              .join("\n")
              .slice(0, 1024);
            if (program.kind === "function") dest.text = "計算エラー";
          }
        }
      }
    }
    return values;
  }
  private cell(
    { context, invoke }: Compiled,
    row: number,
    col: number,
  ): CellDisplay {
    const rowArg = context.newNumber(row),
      colArg = context.newNumber(col);
    try {
      const result = context.callFunction(
        invoke,
        context.undefined,
        rowArg,
        colArg,
      );
      if (result.error) {
        result.error.dispose();
        throw new Error(
          performance.now() >= this.deadline
            ? "時間上限を超えました。"
            : "評価を停止しました（メモリ・スタック上限）。",
        );
      }
      try {
        if (context.typeof(result.value) !== "string")
          throw new Error("表示結果が不正です。");
        const length = context
          .getProp(result.value, "length")
          .consume((h) => context.getNumber(h));
        if (!(length <= 26_000))
          throw new Error("表示結果の上限を超えました。");
        // Enforce the property allowlist again where guest data enters the host.
        const data = JSON.parse(context.getString(result.value));
        if (data.resourceLimit === true)
          throw new Error("メモリ・スタック上限を超えました。");
        const output: CellDisplay = {};
        if (typeof data.text === "string")
          output.text = data.text.slice(0, 4096);
        if (typeof data.error === "string")
          output.error = data.error.slice(0, 240);
        if (data.style) {
          output.style = {};
          for (const key of ["bold", "italic"] as const)
            if (typeof data.style[key] === "boolean")
              output.style[key] = data.style[key];
          for (const key of ["color", "backgroundColor"] as const)
            if (
              typeof data.style[key] === "string" &&
              /^#[0-9a-f]{6}$/i.test(data.style[key])
            )
              output.style[key] = data.style[key];
        }
        return output;
      } finally {
        result.value.dispose();
      }
    } finally {
      rowArg.dispose();
      colArg.dispose();
    }
  }
  dispose() {
    for (const index of this.compiled.keys()) this.release(index);
  }
}
