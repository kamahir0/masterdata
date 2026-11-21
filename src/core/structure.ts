import {
  type Snapshot,
  type Source,
  type Field,
  type Data,
  typeSource,
  fieldsOf,
  isObject,
} from "./types";
import { withIdentity } from "./identity";
export function mapTypedValue(
  snapshot: Snapshot,
  field: Field,
  value: Data,
  target: string,
  modify: (value: Data) => Data,
  seen = new Set<string>(),
): Data {
  if (value == null || (isObject(value) && Object.hasOwn(value, "$masterValue"))) return value;
  if (field.array) {
    if (!Array.isArray(value)) return value;
    const next = value.map((v) =>
      mapTypedValue(
        snapshot,
        { ...field, array: false },
        v,
        target,
        modify,
        seen,
      ),
    );
    return next.some((v, i) => v !== value[i])
      ? withIdentity(value, next)
      : value;
  }
  if (field.type === target) return withIdentity(value, modify(value));
  const type = typeSource(snapshot, field.type);
  if (!type?.data?.custom || !isObject(value) || seen.has(field.type))
    return value;
  const nextSeen = new Set(seen).add(field.type);
  let next = value;
  for (const child of fieldsOf(type)) {
    const changed = mapTypedValue(
      snapshot,
      child,
      value[child.name],
      target,
      modify,
      nextSeen,
    );
    if (changed !== value[child.name])
      next = { ...next, [child.name]: changed };
  }
  return withIdentity(value, next);
}
export function mapRecords(
  snapshot: Snapshot,
  owner: Source,
  typeName: string | null,
  modify: (record: Data) => Data,
): Snapshot["sources"] {
  return snapshot.sources.map((source) => {
    if (!Array.isArray(source.data?.records)) return source;
    const schema = source.data?.kind === "masterValue" ? source : snapshot.sources.find(
      (s) => s.data?.kind === "schema" && s.data?.table === source.data.table,
    );
    const records = source.data.records.map((record: Data) => {
      if (!isObject(record)) return record;
      if (!typeName && (source.id === owner.id || (owner.data.kind === "schema" && source.data.kind === "data" && source.data.table === owner.data.table))) return modify(record);
      if (!typeName || !schema) return record;
      let next = record;
      for (const field of fieldsOf(schema)) {
        const changed = mapTypedValue(
          snapshot,
          field,
          record[field.name],
          typeName,
          modify,
        );
        if (changed !== record[field.name])
          next = { ...next, [field.name]: changed };
      }
      return next;
    });
    return records.some((r: Data, i: number) => r !== source.data.records[i])
      ? { ...source, data: { ...source.data, records } }
      : source;
  });
}
