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
export function recordAllocator(
  snapshot: Snapshot,
  table: TableInfo,
): (template?: Data) => Data {
  const rows = table.sources.flatMap((s) =>
    Array.isArray(s.data?.records) ? s.data.records : [],
  );
  const keys: string[][] = [
    table.source.data.primaryKey?.fields,
    ...list(table.source.data.secondaryKeys)
      .filter((k) => !k.nonUnique)
      .map((k) => k.fields),
  ].filter((k): k is string[] => Array.isArray(k) && !!k.length);
  const signature = (key: string[], row: Data) =>
    JSON.stringify(key.map((n) => text(row?.[n])));
  const indexes = keys.map(
    (key) => new Set(rows.map((row) => signature(key, row))),
  );
  const keyNames = [...new Set(keys.flat())];
  const used = new Map(
    keyNames.map((name) => [name, new Set(rows.map((r) => text(r?.[name])))]),
  );
  const maxima = new Map<string, bigint>();
  const counters = new Map<string, number>();
  for (const name of keyNames) {
    let max = 0n;
    for (const row of rows) {
      try {
        const n = BigInt(row?.[name]);
        if (n > max) max = n;
      } catch {}
    }
    maxima.set(name, max);
  }
  return (template?: Data) => {
    const result = template
      ? structuredClone(template)
      : Object.fromEntries(
          table.fields.map((f) => [f.name, defaultValue(snapshot, f)]),
        );
    for (let attempt = 0; attempt < 10000; attempt++) {
      const conflict = keys.findIndex((key, i) =>
        indexes[i].has(signature(key, result)),
      );
      if (conflict < 0) {
        keys.forEach((key, i) => indexes[i].add(signature(key, result)));
        keyNames.forEach((name) => {
          used.get(name)!.add(text(result[name]));
          try {
            const n = BigInt(result[name]);
            if (n > (maxima.get(name) ?? 0n)) maxima.set(name, n);
          } catch {}
        });
        return result;
      }
      const components = [...keys[conflict]]
        .reverse()
        .map((name) => table.fields.find((f) => f.name === name))
        .filter((f): f is Field => !!f && !f.array && !f.nullable);
      // Prefer a numeric/string component of a composite key; cycling only its Enum component can falsely exhaust a valid key space.
      const field =
        components.find((f) =>
          ["int", "uint", "long", "ulong", "string"].includes(
            primitiveOf(snapshot, f),
          ),
        ) ?? components[0];
      if (!field)
        throw new Error(
          "一意なキーを作成できません。キーの設定を確認してください。",
        );
      const primitive = primitiveOf(snapshot, field);
      const values = used.get(field.name)!;
      if (primitive === "string") {
        let i = counters.get(field.name) ?? 0;
        let candidate: string;
        do {
          candidate = `${template?.[field.name] || "new"}-${++i}`;
        } while (values.has(candidate));
        counters.set(field.name, i);
        result[field.name] = candidate;
      } else if (["int", "uint", "long", "ulong"].includes(primitive)) {
        const limit =
          primitive === "int"
            ? 2147483647n
            : primitive === "uint"
              ? 4294967295n
              : primitive === "long"
                ? 9223372036854775807n
                : 18446744073709551615n;
        let candidate = (maxima.get(field.name) ?? 0n) + 1n;
        if (candidate > limit) {
          candidate = 0n;
          while (
            values.has(String(candidate)) &&
            candidate <= BigInt(values.size)
          )
            candidate++;
        }
        if (candidate > limit || values.has(String(candidate)))
          throw new Error("キーの空きを割り当てられません。");
        result[field.name] = candidate;
      } else {
        const members =
          typeSource(snapshot, field.type)?.data?.enum?.members ?? [];
        if (attempt >= members.length)
          throw new Error("このEnumの一意な値は使い切られています。");
        result[field.name] = members[attempt]?.name;
      }
    }
    throw new Error("一意なキー候補を割り当てられませんでした。");
  };
}
export function newRecord(
  snapshot: Snapshot,
  table: TableInfo,
  template?: Data,
): Data {
  return recordAllocator(snapshot, table)(template);
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
