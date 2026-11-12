import { isObject, type Source, type SavedFile } from "./types";
import { moveItem, spliceItems, withIdentity } from "./identity";
import type { Session } from "./session";
import { parseSource } from "./yaml";

export const displayDefinitionKeys = [
  "functionColumns",
  "formatRules",
] as const;
export type DisplayDefinitionKey = (typeof displayDefinitionKeys)[number];
export interface DisplayDefinition {
  name: string;
  script: string;
  [key: string]: unknown;
}
export type DisplayDefinitionList =
  | { kind: "absent"; items: readonly unknown[] }
  | { kind: "array"; items: readonly unknown[] }
  | { kind: "invalid"; items: readonly unknown[] };

export function isDisplayDefinition(
  value: unknown,
): value is DisplayDefinition {
  return (
    isObject(value) &&
    typeof value.name === "string" &&
    typeof value.script === "string"
  );
}

export function displayDefinitions(
  source: Source,
  key: DisplayDefinitionKey,
): DisplayDefinitionList {
  if (displayDocumentProblem(source)) return { kind: "invalid", items: [] };
  if (!Object.hasOwn(source.display?.data ?? {}, key))
    return { kind: "absent", items: [] };
  const value: unknown = source.display!.data[key];
  return Array.isArray(value)
    ? { kind: "array", items: value }
    : { kind: "invalid", items: [] };
}

export function displayDefinitionProblems(
  source: Source,
  key: DisplayDefinitionKey,
): string[][] {
  const { items } = displayDefinitions(source, key);
  const names = new Map<string, number>();
  for (const item of items)
    if (isObject(item) && typeof item.name === "string")
      names.set(item.name, (names.get(item.name) ?? 0) + 1);
  const fieldNames = new Set(
    Array.isArray(source.data?.fields)
      ? source.data.fields.flatMap((field: unknown) =>
          isObject(field) && typeof field.name === "string" ? [field.name] : [],
        )
      : [],
  );
  return items.map((item) => {
    if (!isDisplayDefinition(item))
      return [
        "nameとscriptが文字列のオブジェクトではありません。表示YAMLで修正してください。",
      ];
    const problems: string[] = [];
    if (!item.name.trim()) problems.push("名前が空です。");
    if ((names.get(item.name) ?? 0) > 1)
      problems.push("同じ種別に同名の定義があります。");
    if (key === "functionColumns" && fieldNames.has(item.name))
      problems.push("通常列と同名です。");
    if (!item.script.trim())
      problems.push("コードが空です。未完成のまま適用・保存できます。");
    return problems;
  });
}

export function newDisplayDefinition(
  items: readonly unknown[],
  key: DisplayDefinitionKey,
): DisplayDefinition {
  const names = new Set(
    items.flatMap((item) =>
      isObject(item) && typeof item.name === "string" ? [item.name] : [],
    ),
  );
  const stem = key === "functionColumns" ? "関数列" : "書式ルール";
  let index = 1;
  while (names.has(`${stem}${index}`)) index++;
  return { name: `${stem}${index}`, script: "" };
}

export function editDisplayDefinition(
  items: readonly unknown[],
  index: number,
  name: string,
  script: string,
): unknown[] {
  const item = items[index];
  if (!isDisplayDefinition(item))
    throw new Error("表示YAMLで定義の形式を修正してください。");
  const next = [...items];
  // 未知キーと編集上の同一性を保持し、入力対象の二つのプロパティだけを変更する。
  next[index] = withIdentity(item, { ...item, name, script });
  return withIdentity(items, next);
}

export function moveDisplayDefinition(
  items: readonly unknown[],
  from: number,
  to: number,
): unknown[] {
  if (
    !Number.isInteger(from) ||
    !Number.isInteger(to) ||
    from < 0 ||
    to < 0 ||
    from >= items.length ||
    to >= items.length
  )
    throw new Error("並べ替え対象が見つかりません。");
  return moveItem(items, from, to);
}

export function removeDisplayDefinition(
  items: readonly unknown[],
  index: number,
): unknown[] {
  if (!Number.isInteger(index) || index < 0 || index >= items.length)
    throw new Error("削除対象が見つかりません。");
  return spliceItems(items, index, 1);
}

export function appendDisplayDefinition(
  items: readonly unknown[],
  item: DisplayDefinition,
): unknown[] {
  return spliceItems(items, items.length, 0, item);
}

export function updateDisplayDefinitions(
  session: Session,
  source: Source,
  key: DisplayDefinitionKey,
  next: readonly unknown[],
  label: string,
): void {
  const current = session
    .getState()
    .snapshot.sources.find((item) => item.id === source.id);
  // 未適用入力は開始時の表示文書にだけ属する。再読込・Undo・置換後に
  // 別項目へ適用されないよう、値の一致ではなく文書の同一性で確認する。
  if (
    !current ||
    current.syntaxError ||
    current.data?.kind !== "schema" ||
    current.display !== source.display
  )
    throw new Error(
      "対象の定義が変更されました。入力を取り消し、現在の定義を開き直してください。",
    );
  if (displayDefinitions(current, key).kind === "invalid")
    throw new Error(
      `${key}は編集できない形式です。表示YAMLで修正してください。`,
    );
  const display =
    current.display ??
    parseSource(`${current.path}.view`, "{}\n", `display:${current.id}`);
  session.display(
    source.id,
    {
      ...display,
      data: { ...display.data, [key]: next },
      raw: undefined,
      syntaxError: undefined,
    },
    label,
  );
}

export function displayDocumentProblem(source: Source): string | undefined {
  const doc = source.display;
  if (!doc) return;
  if (doc.syntaxError) return doc.syntaxError;
  if (!isObject(doc.data))
    return "表示YAMLのルートはオブジェクトにしてください。元の内容は保持しています。";
  // No version field is emitted by this format. Future versioned documents are
  // opaque until explicitly repaired; never reinterpret or reset them.
  if (Object.hasOwn(doc.data, "version"))
    return "この版の表示YAMLには対応していません。元の内容は保持しています。";
}

export function updateRawDisplay(
  session: Session,
  source: Source,
  raw: string,
): void {
  const current = session
    .getState()
    .snapshot.sources.find((s) => s.id === source.id);
  if (!current || current.display !== source.display)
    throw new Error(
      "対象の表示YAMLが変更されました。入力を取り消して開き直してください。",
    );
  session.display(
    source.id,
    {
      ...parseSource(`${current.path}.view`, raw, `display:${current.id}`),
      raw,
    },
    "表示YAMLを編集",
  );
}

// The schema and its attachment form a save unit, including absent destinations.
// Unchanged companions must also be checked: editing only the view does not
// authorize saving against an externally replaced schema (or vice versa).
export function displaySaveGuards(
  before: readonly SavedFile[],
  after: readonly SavedFile[],
) {
  const old = new Map(before.map((f) => [f.path, f.text]));
  const next = new Map(after.map((f) => [f.path, f.text]));
  const paths = new Set<string>();
  for (const path of new Set([...old.keys(), ...next.keys()])) {
    if (old.get(path) === next.get(path)) continue;
    const owner = path.endsWith(".view") ? path.slice(0, -5) : path;
    if (!/\.ya?ml$/i.test(owner)) continue;
    const view = `${owner}.view`;
    // A data file (or failed schema parse) can have an unrelated .view next
    // to it. Editing its YAML in place does not claim that attachment.
    if (old.has(owner) && next.has(owner) && !old.has(view) && !next.has(view))
      continue;
    paths.add(owner);
    paths.add(view);
  }
  return [...paths].map((path) => ({
    path,
    before: old.get(path) ?? null,
    after: next.get(path) ?? null,
  }));
}
