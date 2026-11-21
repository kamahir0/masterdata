import {
  fieldsOf,
  isObject,
  list,
  tables,
  type Data,
  type Field,
  type Problem,
  type Snapshot,
  type Source,
} from "./types";

export const singletonId = "MasterValueRowId";
export const isMaster = (s?: Source) => s?.data?.kind === "masterValue";
export function sameKey(a: Data, b: Data) {
  return (
    (typeof a === "string") === (typeof b === "string") &&
    String(a) === String(b)
  );
}
export function sameShape(a: Field, b: Field) {
  return (
    a.type === b.type &&
    !!a.array === !!b.array &&
    !!a.nullable === !!b.nullable
  );
}
export const isMasterRef = (value: Data) =>
  isObject(value) && Object.hasOwn(value, "$masterValue");
export function masterItems(snapshot: Snapshot) {
  return snapshot.sources.filter(isMaster).flatMap((source) =>
    fieldsOf(source).map((field) => ({
      source,
      field,
      key: field.valueKey,
      value: source.data.records?.[0]?.[field.name],
    })),
  );
}
export function rewriteRefs(
  source: Source,
  visit: (ref: Data) => Data,
): Source {
  if (
    !["schema", "data"].includes(source.data?.kind) ||
    !Array.isArray(source.data.records)
  )
    return source;
  let changed = false;
  const records = source.data.records.map((record: Data) => {
    if (!isObject(record)) return record;
    let row = record;
    for (const [name, cell] of Object.entries(record)) {
      if (!isMasterRef(cell)) continue;
      const next = visit(cell.$masterValue);
      if (next === cell.$masterValue) continue;
      row = { ...row, [name]: { $masterValue: next } };
      changed = true;
    }
    return row;
  });
  return changed
    ? { ...source, raw: undefined, data: { ...source.data, records } }
    : source;
}

// Expand only the validation/build snapshot. The editable row retains its reference.
export function resolveMasterValues(input: Snapshot) {
  const problems: Problem[] = [];
  const definitions = input.sources.filter(isMaster);
  const byDefinition = new Map<
    string,
    Map<string, ReturnType<typeof masterItems>>
  >();
  const keyToken = (key: Data) =>
    `${typeof key === "string" ? "s" : "n"}:${String(key)}`;
  for (const item of masterItems(input)) {
    const name = item.source.data.table;
    const byKey = byDefinition.get(name) ?? new Map();
    byDefinition.set(name, byKey);
    const key = keyToken(item.key);
    const entries = byKey.get(key) ?? [];
    entries.push(item);
    byKey.set(key, entries);
  }
  const schemas = new Map(tables(input).map((t) => [t.name, t]));
  const generatedName = (s: Source) =>
    s.data?.kind === "type"
      ? String(s.data.name)
      : String(
          s.data?.csharpName ||
            String(s.data?.table ?? "").replace(
              /(^|[-_ ])([A-Za-z])/g,
              (_, __, c) => c.toUpperCase(),
            ),
        );
  const declarations = input.sources.filter((s) =>
    ["schema", "masterValue", "type"].includes(s.data?.kind),
  );
  const sources = input.sources.map((source) => {
    const data = source.data;
    const issue = (path: (string | number)[], message: string) =>
      problems.push({ source: source.id, path, message, code: "MASTER_VALUE" });
    if (definitions.length && declarations.includes(source)) {
      const name = generatedName(source).toLowerCase();
      if (
        name === "mastervaluedatabase" ||
        (isMaster(source) &&
          ([
            "memorydatabase",
            "databasebuilder",
            "mastermemoryresolver",
          ].includes(name) ||
            declarations.some(
              (s) => s !== source && generatedName(s).toLowerCase() === name,
            )))
      )
        issue(
          [],
          "生成するC#名が他の定義またはデータベースの型名と衝突しています。",
        );
    }
    if (isMaster(source)) {
      if (data.csharpName != null || !Array.isArray(data.fields))
        issue(["fields"], "マスター値には項目一覧と定義名を指定してください。");
      if (!["number", "string"].includes(data.keyMode))
        issue(["keyMode"], "キー方式を指定してください。");
      if (
        !Array.isArray(data.records) ||
        data.records.length !== 1 ||
        !isObject(data.records[0])
      )
        issue(["records"], "マスター値には1件のレコードが必要です。");
      const seen = new Set<string>();
      fieldsOf(source).forEach((field, i) => {
        if (field.name === generatedName(source))
          issue(
            ["fields", i, "name"],
            "型と同名のプロパティは使用できません。",
          );
        const k = field.valueKey;
        const valid =
          data.keyMode === "string"
            ? typeof k === "string" && k.length > 0
            : ["bigint", "number"].includes(typeof k) &&
              Number.isInteger(Number(k)) &&
              Number(k) >= 0 &&
              Number(k) <= 2147483647;
        if (!valid || seen.has(String(k)))
          issue(
            ["fields", i, "valueKey"],
            "項目キーの形式または重複を確認してください。",
          );
        seen.add(String(k));
        if (field.key == null || Number(field.key) === 0)
          issue(
            ["fields", i, "key"],
            "フィールド番号には正の整数を指定してください。",
          );
        if (isMasterRef(data.records?.[0]?.[field.name]))
          issue(["records", 0, field.name], "マスター値同士は参照できません。");
      });
      return {
        ...source,
        data: {
          ...data,
          kind: "schema",
          fields: [
            ...list(data.fields),
            { key: 0, name: singletonId, type: "int" },
          ],
          primaryKey: { fields: [singletonId] },
          records: [{ ...data.records?.[0], [singletonId]: 0n }],
        },
      };
    }
    if (
      !["schema", "data"].includes(data?.kind) ||
      !Array.isArray(data.records)
    )
      return source;
    if (
      data.kind === "data" &&
      definitions.some((s) => s.data.table === data.table)
    )
      issue(["table"], "マスター値のレコードは別ファイルに追加できません。");
    const schema = schemas.get(data.table);
    let changed = false;
    const records = data.records.map((row: Data, index: number) => {
      if (!isObject(row)) return row;
      let result = row;
      for (const [name, cell] of Object.entries(row)) {
        if (!isMasterRef(cell)) continue;
        const ref = cell.$masterValue;
        const matches =
          byDefinition.get(ref?.definition)?.get(keyToken(ref?.key)) ?? [];
        const field = schema?.fields.find((f) => f.name === name);
        if (
          matches.length !== 1 ||
          !field ||
          !sameShape(field, matches[0].field)
        ) {
          issue(
            ["records", index, name],
            `マスター値「${ref?.definition} / ${ref?.key}」の参照先または型を確認してください。`,
          );
        } else {
          result = { ...result, [name]: matches[0].value };
          changed = true;
        }
      }
      return result;
    });
    return changed ? { ...source, data: { ...data, records } } : source;
  });
  return { snapshot: { ...input, sources }, problems };
}
