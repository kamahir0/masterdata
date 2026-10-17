import { parseDocument, stringify } from "yaml";
import {
  type Data,
  type Field,
  type Snapshot,
  typeKind,
  typeSource,
} from "./types";
import { primitiveOf } from "./schema";
export function writeTsv(rows: string[][]): string {
  return rows
    .map((row) =>
      row
        .map((v) => (/[\t\r\n"]/.test(v) ? `"${v.replace(/"/g, '""')}"` : v))
        .join("\t"),
    )
    .join("\n");
}
export function readTsv(text: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [],
    value = "",
    quoted = false;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quoted) {
      if (c === '"') {
        if (text[i + 1] === '"') {
          value += '"';
          i++;
        } else quoted = false;
      } else value += c;
    } else if (c === '"' && value === "") quoted = true;
    else if (c === "\t") {
      row.push(value);
      value = "";
    } else if (c === "\n" || c === "\r") {
      if (c === "\r" && text[i + 1] === "\n") i++;
      row.push(value);
      rows.push(row);
      row = [];
      value = "";
    } else value += c;
  }
  if (value !== "" || row.length || !rows.length) {
    row.push(value);
    rows.push(row);
  }
  return rows;
}
export function cellText(value: Data): string {
  return value && typeof value === "object"
    ? stringify(value, { lineWidth: 0 }).trimEnd()
    : value === null
      ? "null"
      : value === undefined
        ? ""
        : String(value);
}
export function parseCell(
  snapshot: Snapshot,
  field: Field,
  value: string,
): Data {
  const primitive = primitiveOf(snapshot, field);
  const kind = typeKind(typeSource(snapshot, field.type));
  if (!field.array && primitive === "string") return value;
  if (!field.array && kind === "enum") return value;
  try {
    const doc = parseDocument(value, { intAsBigInt: true });
    if (doc.errors.length) return value;
    return doc.toJS({ maxAliasCount: 0 });
  } catch {
    return value;
  }
}
// The tagged form avoids conflating bigint, string "null", empty string, and null between cells of this editor.
export const encodeCells = (cells: Data[][]) =>
  JSON.stringify(cells, (_, v) =>
    typeof v === "bigint" ? { $masterdataInteger: String(v) } : v,
  );
export const decodeCells = (value: string): Data[][] =>
  JSON.parse(value, (_, v) =>
    v &&
    typeof v === "object" &&
    Object.keys(v).length === 1 &&
    typeof v.$masterdataInteger === "string"
      ? BigInt(v.$masterdataInteger)
      : v,
  );
