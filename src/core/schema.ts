import {
  type Snapshot,
  type Field,
  type Data,
  type TableInfo,
  typeSource,
  typeKind,
  fieldsOf,
  isObject,
  primitives,
  text,
  selected,
  list,
} from "./types";
export function defaultValue(
  snapshot: Snapshot,
  field: Field,
  seen = new Set<string>(),
): Data {
  if (field.nullable) return null;
  if (field.array) return [];
  if (field.type === "bool") return false;
  if (field.type === "string") return "";
  if (primitives.includes(field.type))
    return field.type.includes("float") || field.type === "double" ? 0 : 0n;
  if (seen.has(field.type)) return null;
  const type = typeSource(snapshot, field.type);
  const kind = typeKind(type);
  if (!type || !kind) return null;
  const definition = type.data[kind];
  if (kind === "valueObject")
    return defaultValue(snapshot, { name: "", type: definition.underlying });
  if (kind === "enum")
    return (
      (definition.members ?? []).find((m: any) => String(m.value) === "0")
        ?.name ??
      definition.members?.[0]?.name ??
      ""
    );
  if (kind === "flags") return ["None"];
  const next = new Set(seen).add(field.type);
  return Object.fromEntries(
    fieldsOf(type).map((f) => [f.name, defaultValue(snapshot, f, next)]),
  );
}
export function newRecord(
  snapshot: Snapshot,
  table: TableInfo,
  template?: Data,
): Data {
  const result = template
    ? structuredClone(template)
    : Object.fromEntries(
        table.fields.map((f) => [f.name, defaultValue(snapshot, f)]),
      );
  const rows = table.sources.flatMap((s) =>
    Array.isArray(s.data?.records) ? s.data.records : [],
  );
  const keys: string[][] = [
    table.source.data.primaryKey?.fields,
    ...list(table.source.data.secondaryKeys)
      .filter((k: any) => !k.nonUnique)
      .map((k: any) => k.fields),
  ].filter((k): k is string[] => Array.isArray(k) && !!k.length);
  for (let attempt = 0; attempt < 10000; attempt++) {
    const conflict = keys.find((key) =>
      rows.some((r) => key.every((k) => text(r?.[k]) === text(result[k]))),
    );
    if (!conflict) return result;
    const name = [...conflict].reverse().find((n) => {
      const f = table.fields.find((f) => f.name === n);
      return f && !f.array && !f.nullable;
    });
    if (!name)
      throw new Error(
        "重複しないキーを作成できません。キーの設定を確認してください。",
      );
    const f = table.fields.find((f) => f.name === name)!;
    const t = typeSource(snapshot, f.type);
    const primitive = t?.data?.valueObject?.underlying ?? f.type;
    if (primitive === "string")
      result[name] = `${template?.[name] || "new"}-${attempt + 1}`;
    else if (["int", "uint", "long", "ulong"].includes(primitive)) {
      const used = new Set(rows.map((r) => String(r?.[name])));
      let maximum = 0n;
      for (const row of rows) {
        try {
          const n = BigInt(row?.[name]);
          if (n > maximum) maximum = n;
        } catch {}
      }
      const limit =
        primitive === "int"
          ? 2147483647n
          : primitive === "uint"
            ? 4294967295n
            : primitive === "long"
              ? 9223372036854775807n
              : 18446744073709551615n;
      let candidate = maximum + 1n;
      if (candidate > limit) {
        candidate = 0n;
        while (used.has(String(candidate)) && candidate <= BigInt(used.size))
          candidate++;
      }
      if (candidate > limit || used.has(String(candidate)))
        throw new Error("キーの空きを割り当てられません。");
      result[name] = candidate;
    } else if (t?.data?.enum) {
      const members = t.data.enum.members ?? [];
      if (attempt >= members.length)
        throw new Error("このEnumの一意な値は使い切られています。");
      result[name] = members[attempt]?.name;
    } else
      throw new Error(
        "このキーは自動割り当てできません。設定を確認してください。",
      );
  }
  throw new Error("一意なキー候補を割り当てられませんでした。");
}
export function flagsChange(current: Data, names: string[]): string[] {
  const previous = Array.isArray(current) ? current : [];
  if (names.includes("None") && !previous.includes("None")) return ["None"];
  const bits = names.filter((n) => n !== "None");
  return bits.length ? bits : ["None"];
}
export function primitiveOf(snapshot: Snapshot, field: Field): string {
  return (
    typeSource(snapshot, field.type)?.data?.valueObject?.underlying ??
    field.type
  );
}
export function inProfile(snapshot: Snapshot, name: string): Snapshot {
  const profile = snapshot.config.profiles[name];
  return {
    ...snapshot,
    sources: snapshot.sources.map((s) =>
      Array.isArray(s.data?.records)
        ? {
            ...s,
            data: {
              ...s.data,
              records: s.data.records.filter((r: any) => selected(r, profile)),
            },
          }
        : s,
    ),
  };
}
