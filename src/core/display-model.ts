import {
  displayDefinitionProblems,
  displayDefinitions,
  isDisplayDefinition,
} from "./display-definitions";
import { primitiveOf } from "./schema";
import type { Snapshot, TableInfo } from "./types";

export interface DisplayStyle {
  bold?: boolean;
  italic?: boolean;
  color?: string;
  backgroundColor?: string;
}
export interface DisplayProgram {
  name: string;
  kind: "function" | "rule";
  script: string;
  error?: string;
}
export interface DisplayColumn {
  name: string;
  kind: "data" | "function";
  type?: string;
}
export interface DisplayInput {
  programs: DisplayProgram[];
  columns: DisplayColumn[];
  records: unknown[];
  error?: string;
}
export interface CellDisplay {
  text?: string;
  style?: DisplayStyle;
  error?: string;
}
export type RowDisplay = CellDisplay[];
export type DisplayJob =
  | { type: "init"; input: DisplayInput; disabled: number[] }
  | { type: "rows"; indices: number[] };
export type DisplayReply =
  | { type: "started"; program: number }
  | { type: "ready" }
  | { type: "rows"; indices: number[]; values: RowDisplay[] }
  | { type: "error"; message: string };

export function displayModel(table: TableInfo | undefined, snapshot: Snapshot) {
  if (!table?.source.display) return;
  const total = (["functionColumns", "formatRules"] as const).reduce(
    (n, key) => n + displayDefinitions(table.source, key).items.length,
    0,
  );
  const limitError =
    total > 32
      ? "表示定義は32件までです。表示設定で定義数を減らしてください。"
      : undefined;
  const programs: DisplayProgram[] = [];
  for (const key of ["functionColumns", "formatRules"] as const) {
    const items = displayDefinitions(table.source, key).items;
    const problems = limitError
      ? []
      : displayDefinitionProblems(table.source, key);
    items.slice(0, 32 - programs.length).forEach((item, index) => {
      const valid = isDisplayDefinition(item);
      programs.push({
        name: valid && item.name ? item.name : `${key}[${index}]`,
        kind: key === "functionColumns" ? "function" : "rule",
        script: valid ? item.script : "",
        error: problems[index]?.join(" ") || undefined,
      });
    });
  }
  if (!programs.length) return;
  const functions = programs.filter((p) => p.kind === "function");
  const columns: DisplayColumn[] = [
    ...table.fields.map((f) => ({
      name: f.name,
      kind: "data" as const,
      type: f.array ? "array" : primitiveOf(snapshot, f),
    })),
    ...functions.map((p) => ({ name: p.name, kind: "function" as const })),
  ];
  const offsets = new Map<string, number>();
  const records: unknown[] = [];
  let error = limitError;
  for (const source of table.sources) {
    offsets.set(source.id, records.length);
    const rows = source.data?.records;
    if (Array.isArray(rows)) {
      for (const record of rows) records.push(record);
    } else if (rows !== undefined) {
      error ??= `${source.path} の records を配列に修正すると、表全体の表示計算を再開します。`;
    }
  }
  return {
    functions,
    offsets,
    input: { programs, columns, records, error },
  };
}

// Check the transfer before structuredClone or wire encoding duplicates a large
// draft. Depth and work are bounded even for incomplete nested field values.
export function checkDisplayInput(input: DisplayInput) {
  if (input.error) throw new Error(input.error);
  if (
    input.records.length > 100_000 ||
    input.programs.length > 32 ||
    input.columns.length > 256
  )
    throw new Error("表示計算の上限（10万行・32定義・256列）を超えています。");
  let size = 0;
  function visit(value: unknown, depth: number) {
    size += 8;
    if (depth > 64 || size > 8 * 1024 * 1024)
      throw new Error("表示計算の入力サイズまたはネスト上限を超えています。");
    if (typeof value === "string") size += value.length;
    else if (typeof value === "bigint") size += String(value).length;
    else if (Array.isArray(value)) {
      for (const item of value) visit(item, depth + 1);
    } else if (value && typeof value === "object") {
      for (const key in value) {
        if (!Object.hasOwn(value, key)) continue;
        size += key.length;
        visit((value as Record<string, unknown>)[key], depth + 1);
      }
    }
  }
  visit(input, 0);
  if (size > 8 * 1024 * 1024)
    throw new Error("表示計算の入力サイズ上限を超えています。");
}
