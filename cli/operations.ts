import {
  type Data,
  type Field,
  type Snapshot,
  type Source,
  type TableInfo,
  fieldsOf,
  isObject,
  list,
  primitives,
  selected,
  setIn,
  tables,
  text,
  typeKind,
  typeSource,
  typeSources,
  uid,
} from "../src/core/types";
import {
  defaultValue,
  inProfile,
  primitiveOf,
  recordAllocator,
} from "../src/core/schema";
import { Session } from "../src/core/session";
import { mapRecords } from "../src/core/structure";
import { fingerprint, validate } from "../src/core/validation";
import { serialize } from "../src/core/yaml";
import {
  type PathStep,
  coerceScalarOrStructured,
  matchesPk,
  matchesWhere,
  parseFieldPath,
  parsePkSelector,
  resolveFieldAtPath,
  setByPath,
  splitCsv,
} from "./args";

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function normalizeSourcePath(input: string): string {
  const trimmed = input.trim().replace(/^sources\//, "");
  return /\.ya?ml$/i.test(trimmed) ? trimmed : `${trimmed}.yaml`;
}

export function ensureSourceEditable(source: Source) {
  if (source.syntaxError || !isObject(source.data))
    throw new Error(
      `対象ファイルにYAML構文エラーがあるため変更できません: ${source.path}`,
    );
}

export function ensureTableEditable(snapshot: Snapshot, tableName: string) {
  const pattern = new RegExp(
    `\\btable:\\s*['"]?${escapeRegExp(tableName)}['"]?\\b`,
  );
  for (const s of snapshot.sources) {
    if (s.syntaxError && s.raw && pattern.test(s.raw))
      throw new Error(
        `テーブル「${tableName}」の関連ファイルにYAML構文エラーがあるため変更できません: ${s.path}`,
      );
    if (
      (s.data?.kind === "schema" || s.data?.kind === "data") &&
      String(s.data?.table) === tableName
    )
      ensureSourceEditable(s);
  }
}

export function ensureTokenEditable(snapshot: Snapshot, token: string) {
  const pattern = new RegExp(`\\b${escapeRegExp(token)}\\b`);
  for (const s of snapshot.sources) {
    if (s.syntaxError && s.raw && pattern.test(s.raw))
      throw new Error(
        `関連する可能性のあるファイルにYAML構文エラーがあるため一括変更を中止しました: ${s.path}`,
      );
  }
}

export function findTable(snapshot: Snapshot, name: string): TableInfo {
  const table = tables(snapshot).find((t) => t.name === name);
  if (!table) throw new Error(`テーブル「${name}」が見つかりません。`);
  ensureSourceEditable(table.source);
  return table;
}

export function findType(snapshot: Snapshot, name: string): Source {
  const source = typeSource(snapshot, name);
  if (!source) throw new Error(`型「${name}」が見つかりません。`);
  ensureSourceEditable(source);
  return source;
}

export function nextFlagBit(source: Source): bigint {
  const def = source.data?.flags;
  const members = Array.isArray(def?.members) ? def.members : [];
  const used = new Set<bigint>();
  for (const m of members) {
    try {
      used.add(BigInt(m.value));
    } catch {}
  }
  const width =
    def?.underlying === "long" || def?.underlying === "ulong" ? 64n : 32n;
  const signed = (def?.underlying ?? "uint") === "int" || def?.underlying === "long";
  const maxBit = signed ? width - 1n : width;
  for (let bit = 0n; bit < maxBit; bit++) {
    const candidate = 1n << bit;
    if (!used.has(candidate)) return candidate;
  }
  throw new Error(`Flags「${source.data?.name}」の空きビットがありません。`);
}

export function nextEnumValue(source: Source): bigint {
  const def = source.data?.enum;
  const members = Array.isArray(def?.members) ? def.members : [];
  if (!members.length) return 0n;
  let max = -1n;
  for (const m of members) {
    try {
      const v = BigInt(m.value);
      if (v > max) max = v;
    } catch {}
  }
  return max + 1n;
}

export function formatProblems(snapshot: Snapshot, profile = "") {
  if (profile && !snapshot.config.profiles[profile])
    throw new Error(`プロファイル「${profile}」が見つかりません。`);
  const rawProblems = validate(snapshot, profile);
  const byId = new Map(snapshot.sources.map((s) => [s.id, s]));
  const allTables = tables(snapshot);
  return rawProblems.map((p) => {
    const src = byId.get(p.source);
    const tableName =
      src?.data?.kind === "schema" || src?.data?.kind === "data"
        ? String(src.data.table ?? "")
        : undefined;
    let recordPk: Data = undefined;
    if (
      tableName &&
      p.path[0] === "records" &&
      typeof p.path[1] === "number" &&
      Array.isArray(src?.data?.records)
    ) {
      const table = allTables.find((t) => t.name === tableName);
      const row = src!.data.records[p.path[1]];
      const pkFields: string[] = table?.source.data.primaryKey?.fields ?? [];
      if (isObject(row) && pkFields.length) {
        recordPk =
          pkFields.length === 1
            ? row[pkFields[0]]
            : Object.fromEntries(pkFields.map((k) => [k, row[k]]));
      }
    }
    return {
      file: src ? `sources/${src.path}` : p.source,
      source: src?.path ?? p.source,
      ...(tableName ? { table: tableName } : {}),
      ...(src?.data?.kind === "type" && src.data?.name
        ? { type: String(src.data.name) }
        : {}),
      ...(recordPk !== undefined ? { recordPk } : {}),
      path: p.path,
      code: p.code,
      message: p.message,
    };
  });
}

const intLimits: Record<string, bigint> = {
  int: 2147483647n,
  uint: 4294967295n,
  long: 9223372036854775807n,
  ulong: 18446744073709551615n,
};

function seedOmittedPrimaryKeys(
  snapshot: Snapshot,
  table: TableInfo,
  rows: Data[],
  record: Record<string, Data>,
  omittedFields: string[],
) {
  for (const name of omittedFields) {
    const field = table.fields.find((f) => f.name === name);
    if (!field || field.array || field.nullable) continue;
    const prim = primitiveOf(snapshot, field);
    if (!(prim in intLimits)) continue;
    let max = 0n;
    for (const r of rows) {
      try {
        const n = BigInt(r?.[name]);
        if (n > max) max = n;
      } catch {}
    }
    const candidate = max + 1n;
    if (candidate <= intLimits[prim]) {
      record[name] = candidate;
    }
  }
}

export function inspectProject(snapshot: Snapshot, target?: string) {
  const allTables = tables(snapshot);
  const allTypes = typeSources(snapshot);
  const problems = formatProblems(snapshot);
  const syntaxErrors = snapshot.sources
    .filter((s) => s.syntaxError)
    .map((s) => ({ path: s.path, message: s.syntaxError! }));

  const summarizeTable = (t: TableInfo) => {
    const pkFields: string[] = list(t.source.data.primaryKey?.fields);
    let nextPrimaryKey: Data = null;
    try {
      const rows = t.sources.flatMap((s) =>
        Array.isArray(s.data?.records) ? s.data.records : [],
      );
      const template = Object.fromEntries(
        t.fields.map((f) => [f.name, defaultValue(snapshot, f)]),
      );
      seedOmittedPrimaryKeys(snapshot, t, rows, template, pkFields);
      const candidate = recordAllocator(snapshot, t)(template);
      if (pkFields.length === 1) nextPrimaryKey = candidate[pkFields[0]];
      else if (pkFields.length > 1)
        nextPrimaryKey = Object.fromEntries(
          pkFields.map((k) => [k, candidate[k]]),
        );
    } catch {
      nextPrimaryKey = null;
    }
    const dataFiles = t.sources.map((s) => ({
      path: s.path,
      kind: String(s.data?.kind ?? "data"),
      recordCount: Array.isArray(s.data?.records) ? s.data.records.length : 0,
    }));
    return {
      name: t.name,
      csharpName: t.source.data.csharpName ?? null,
      path: t.source.path,
      fields: t.fields.map((f, i) => ({
        name: f.name,
        type: f.type,
        key: Number(f.key ?? i),
        array: !!f.array,
        nullable: !!f.nullable,
      })),
      primaryKey: pkFields,
      secondaryKeys: list(t.source.data.secondaryKeys).map((sk: any) => ({
        fields: list<string>(sk?.fields),
        nonUnique: !!sk?.nonUnique,
      })),
      references: list(t.source.data.references).map((r: any) => ({
        name: r?.name,
        csharpName: r?.csharpName ?? null,
        fields: list<string>(r?.fields),
        target: {
          table: r?.target?.table,
          fields: list<string>(r?.target?.fields),
        },
      })),
      recordCount: dataFiles.reduce((acc, f) => acc + f.recordCount, 0),
      dataFiles,
      requiresExplicitSourceOnAdd: t.sources.length > 1,
      nextPrimaryKey,
    };
  };

  const summarizeType = (s: Source) => {
    const kind = typeKind(s);
    const def = kind ? s.data[kind] : {};
    let nextBit: bigint | null = null;
    if (kind === "flags") {
      try {
        nextBit = nextFlagBit(s);
      } catch {
        nextBit = null;
      }
    }
    return {
      name: String(s.data?.name ?? ""),
      kind: kind ?? "unknown",
      path: s.path,
      ...(def?.underlying ? { underlying: String(def.underlying) } : {}),
      ...(kind === "enum" || kind === "flags"
        ? {
            members: list(def?.members).map((m: any) => ({
              name: m?.name,
              value: m?.value,
            })),
          }
        : {}),
      ...(kind === "flags" ? { nextFlagValue: nextBit } : {}),
      ...(kind === "custom"
        ? {
            fields: fieldsOf(s).map((f, i) => ({
              name: f.name,
              type: f.type,
              key: Number(f.key ?? i),
              array: !!f.array,
              nullable: !!f.nullable,
            })),
          }
        : {}),
    };
  };

  if (target) {
    const normalized = target.replace(/^sources\//, "");
    const table = allTables.find(
      (t) => t.name === target || t.source.path === normalized,
    );
    if (table) {
      const pkFields: string[] = list(table.source.data.primaryKey?.fields);
      const rows = table.sources.flatMap((s) =>
        Array.isArray(s.data?.records) ? s.data.records : [],
      );
      const primaryKeyValues = rows
        .slice(0, 200)
        .map((r: any) =>
          pkFields.length === 1
            ? r?.[pkFields[0]]
            : Object.fromEntries(pkFields.map((k) => [k, r?.[k]])),
        );
      const referencedBy = allTables.flatMap((other) =>
        list(other.source.data.references)
          .filter((r: any) => r?.target?.table === table.name)
          .map((r: any) => ({
            table: other.name,
            reference: r.name,
            fields: list<string>(r.fields),
            targetFields: list<string>(r.target?.fields),
          })),
      );
      return {
        kind: "table",
        ...summarizeTable(table),
        referencedBy,
        primaryKeyValues,
        problems: problems.filter((p) => p.table === table.name),
        totalProjectProblems: problems.length,
      };
    }
    const type = allTypes.find(
      (s) => String(s.data?.name) === target || s.path === normalized,
    );
    if (type) {
      const typeName = String(type.data.name);
      const usedByTables = allTables
        .filter((t) => t.fields.some((f) => f.type === typeName))
        .map((t) => ({
          table: t.name,
          fields: t.fields
            .filter((f) => f.type === typeName)
            .map((f) => f.name),
        }));
      const usedByTypes = allTypes
        .filter(
          (s) =>
            s.data?.custom && fieldsOf(s).some((f) => f.type === typeName),
        )
        .map((s) => ({
          type: String(s.data.name),
          fields: fieldsOf(s)
            .filter((f) => f.type === typeName)
            .map((f) => f.name),
        }));
      return {
        ...summarizeType(type),
        usedByTables,
        usedByTypes,
        problems: problems.filter(
          (p) => p.type === typeName || p.source === type.path,
        ),
        totalProjectProblems: problems.length,
      };
    }
    throw new Error(`指定されたテーブルまたは型「${target}」が見つかりません。`);
  }

  return {
    config: snapshot.config,
    tables: allTables.map(summarizeTable),
    types: allTypes.map(summarizeType),
    problems: {
      total: problems.length,
      syntaxErrorCount: syntaxErrors.length,
      syntaxErrors,
    },
  };
}

export function queryRecords(
  snapshot: Snapshot,
  tableName: string,
  options: {
    pk?: Data;
    where?: string[];
    source?: string;
    profile?: string;
    limit?: number;
    offset?: number;
  },
) {
  if (options.profile && !snapshot.config.profiles[options.profile])
    throw new Error(`プロファイル「${options.profile}」が見つかりません。`);
  const activeSnapshot = options.profile
    ? inProfile(snapshot, options.profile)
    : snapshot;
  const table = findTable(activeSnapshot, tableName);
  const pkFields: string[] = list(table.source.data.primaryKey?.fields);
  const pkExpected =
    options.pk !== undefined
      ? parsePkSelector(activeSnapshot, table.fields, pkFields, options.pk)
      : null;
  const sourceFilter = options.source
    ? normalizeSourcePath(options.source)
    : null;
  if (
    sourceFilter &&
    !table.sources.some((s) => s.path === sourceFilter)
  ) {
    throw new Error(
      `テーブル「${tableName}」のデータファイル「${sourceFilter}」が見つかりません。`,
    );
  }
  const matched: { source: string; index: number; record: Data }[] = [];
  for (const src of table.sources) {
    if (sourceFilter && src.path !== sourceFilter) continue;
    ensureSourceEditable(src);
    const records = list(src.data.records);
    records.forEach((record, index) => {
      if (pkExpected && !matchesPk(record, pkFields, pkExpected)) return;
      if (options.where?.length && !matchesWhere(record, options.where)) return;
      matched.push({ source: src.path, index, record });
    });
  }
  const offset = options.offset ?? 0;
  const limit = options.limit ?? matched.length;
  return {
    table: tableName,
    totalMatched: matched.length,
    records: matched.slice(offset, offset + limit),
  };
}

function nextFieldKey(fields: Field[]): number {
  return (
    Math.max(
      -1,
      ...fields.map((f, i) => Number(f.key ?? i)).filter(Number.isFinite),
    ) + 1
  );
}

function ensureUniqueSourcePath(snapshot: Snapshot, path: string) {
  if (
    !path ||
    path
      .split("/")
      .some((p) => !p || p === "." || p === ".." || /[\\\x00-\x1f]/.test(p))
  )
    throw new Error(`ファイルパスが不正です: ${path}`);
  if (snapshot.sources.some((s) => s.path === path))
    throw new Error(`ファイル「sources/${path}」は既に存在します。`);
}

function addFoldersForPath(folders: string[], relPath: string): string[] {
  const next = new Set(folders);
  const parts = relPath.split("/").slice(0, -1);
  for (let i = 0; i < parts.length; i++) {
    next.add(parts.slice(0, i + 1).join("/"));
  }
  return [...next].sort();
}

function backfillDefinedFields(
  snapshot: Snapshot,
  fields: Field[],
  value: Data,
  seen = new Set<string>(),
): Data {
  if (!isObject(value)) return value;
  let changed = false;
  const next: Record<string, Data> = { ...value };
  for (const field of fields) {
    if (next[field.name] === undefined) {
      next[field.name] = defaultValue(snapshot, field);
      changed = true;
      continue;
    }
    const type = typeSource(snapshot, field.type);
    if (type?.data?.custom && !seen.has(field.type)) {
      const nextSeen = new Set(seen).add(field.type);
      const customFields = fieldsOf(type);
      if (field.array && Array.isArray(next[field.name])) {
        const mapped = next[field.name].map((item: Data) =>
          backfillDefinedFields(snapshot, customFields, item, nextSeen),
        );
        if (mapped.some((item: Data, i: number) => item !== next[field.name][i])) {
          next[field.name] = mapped;
          changed = true;
        }
      } else if (!field.array && isObject(next[field.name])) {
        const updated = backfillDefinedFields(
          snapshot,
          customFields,
          next[field.name],
          nextSeen,
        );
        if (updated !== next[field.name]) {
          next[field.name] = updated;
          changed = true;
        }
      }
    }
  }
  return changed ? next : value;
}

export function applyOperation(
  session: Session,
  op: Record<string, any>,
): Record<string, any> {
  const snapshot = session.state.snapshot;
  const kind = String(op.op ?? "");

  switch (kind) {
    case "config.set": {
      const next = structuredClone(snapshot.config);
      if (op.name !== undefined) {
        if (typeof op.name !== "string" || !op.name.trim())
          throw new Error("プロジェクト名は空でない文字列を指定してください。");
        next.name = op.name.trim();
      }
      if (op.publishCsharp !== undefined)
        next.publish.csharp = String(op.publishCsharp);
      if (op.publishBinary !== undefined)
        next.publish.binary = String(op.publishBinary);
      session.change("設定を更新", (s) => ({ ...s, config: next }));
      return { op: kind, config: session.state.snapshot.config };
    }

    case "config.profile.set": {
      const name = String(op.name ?? "").trim();
      if (!name) throw new Error("プロファイル名を指定してください。");
      const current = snapshot.config.profiles[name] ?? {
        include: [],
        exclude: [],
      };
      const include =
        op.include !== undefined
          ? Array.isArray(op.include)
            ? op.include.map(String)
            : splitCsv(String(op.include))
          : current.include;
      const exclude =
        op.exclude !== undefined
          ? Array.isArray(op.exclude)
            ? op.exclude.map(String)
            : splitCsv(String(op.exclude))
          : current.exclude;
      session.change("プロファイルを更新", (s) => ({
        ...s,
        config: {
          ...s.config,
          profiles: {
            ...s.config.profiles,
            [name]: { include, exclude },
          },
        },
      }));
      return {
        op: kind,
        profile: name,
        definition: session.state.snapshot.config.profiles[name],
      };
    }

    case "config.profile.remove": {
      const name = String(op.name ?? "").trim();
      if (!name || !(name in snapshot.config.profiles))
        throw new Error(`プロファイル「${name}」が見つかりません。`);
      session.change("プロファイルを削除", (s) => {
        const profiles = { ...s.config.profiles };
        delete profiles[name];
        return { ...s, config: { ...s.config, profiles } };
      });
      return { op: kind, removed: name };
    }

    case "table.create": {
      const name = String(op.name ?? op.table ?? "").trim();
      if (!name) throw new Error("テーブル名を指定してください。");
      if (tables(snapshot).some((t) => t.name === name))
        throw new Error(`テーブル「${name}」は既に存在します。`);
      const relPath = normalizeSourcePath(
        op.path ? String(op.path) : `${name}.yaml`,
      );
      ensureUniqueSourcePath(snapshot, relPath);
      const rawFields: any[] = Array.isArray(op.fields)
        ? op.fields
        : [{ key: 0, name: "id", type: "int" }];
      const fields: Field[] = rawFields.map((f, i) =>
        typeof f === "string"
          ? {
              key: i,
              name: f.split(":")[0].trim(),
              type: (f.split(":")[1] ?? "int").trim(),
            }
          : {
              key: Number(f.key ?? i),
              name: String(f.name),
              type: String(f.type ?? "int"),
              ...(f.array ? { array: true } : {}),
              ...(f.nullable ? { nullable: true } : {}),
            },
      );
      const pkFields: string[] = Array.isArray(op.pk)
        ? op.pk.map(String)
        : typeof op.pk === "string" && op.pk.trim()
          ? splitCsv(op.pk)
          : [fields[0]?.name ?? "id"];
      const data: Record<string, any> = {
        kind: "schema",
        table: name,
        ...(op.csharpName ? { csharpName: String(op.csharpName) } : {}),
        fields,
        primaryKey: { fields: pkFields },
        records: [],
      };
      session.change("テーブルを作成", (s) => ({
        ...s,
        folders: addFoldersForPath(s.folders, relPath),
        sources: [...s.sources, { id: uid(), path: relPath, data }],
      }));
      return { op: kind, table: name, path: relPath };
    }

    case "table.delete": {
      const name = String(op.name ?? op.table ?? "").trim();
      ensureTableEditable(snapshot, name);
      const table = findTable(snapshot, name);
      const paths = table.sources.map((s) => s.path);
      session.remove(paths);
      return { op: kind, table: name, removedFiles: paths };
    }

    case "table.dataFile.create": {
      const tableName = String(op.table ?? "").trim();
      findTable(snapshot, tableName);
      if (!op.path)
        throw new Error("作成するデータファイルのパスを指定してください。");
      const relPath = normalizeSourcePath(String(op.path));
      ensureUniqueSourcePath(snapshot, relPath);
      const data = { kind: "data", table: tableName, records: [] };
      session.change("テーブルデータファイルを作成", (s) => ({
        ...s,
        folders: addFoldersForPath(s.folders, relPath),
        sources: [...s.sources, { id: uid(), path: relPath, data }],
      }));
      return { op: kind, table: tableName, path: relPath };
    }

    case "table.column.add": {
      const tableName = String(op.table ?? "").trim();
      ensureTableEditable(snapshot, tableName);
      const table = findTable(snapshot, tableName);
      const colName = String(op.name ?? "").trim();
      const colType = String(op.type ?? "").trim();
      if (!colName || !colType)
        throw new Error("追加する列名と型を指定してください。");
      if (table.fields.some((f) => f.name === colName))
        throw new Error(`列「${colName}」は既に存在します。`);
      const key =
        op.key !== undefined ? Number(op.key) : nextFieldKey(table.fields);
      const field: Field = {
        key,
        name: colName,
        type: colType,
        ...(op.array ? { array: true } : {}),
        ...(op.nullable ? { nullable: true } : {}),
      };
      const nextFields = [...table.fields, field];
      session.change("列を追加", (s) => {
        const updatedSources = mapRecords(s, tableName, null, (record) =>
          colName in record
            ? record
            : { ...record, [colName]: defaultValue(s, field) },
        );
        return {
          ...s,
          sources: updatedSources.map((src) =>
            src.id === table.source.id
              ? { ...src, data: setIn(src.data, ["fields"], nextFields) }
              : src,
          ),
        };
      });
      return { op: kind, table: tableName, field };
    }

    case "table.column.set": {
      const tableName = String(op.table ?? "").trim();
      const table = findTable(snapshot, tableName);
      const colName = String(op.name ?? "").trim();
      const index = table.fields.findIndex((f) => f.name === colName);
      if (index < 0)
        throw new Error(`テーブル「${tableName}」に列「${colName}」がありません。`);
      const current = table.fields[index];
      const updated: Field = { ...current };
      if (op.type !== undefined) updated.type = String(op.type);
      if (op.key !== undefined) updated.key = Number(op.key);
      if (op.array !== undefined) {
        if (op.array) updated.array = true;
        else delete updated.array;
      }
      if (op.nullable !== undefined) {
        if (op.nullable) updated.nullable = true;
        else delete updated.nullable;
      }
      session.update(
        table.source.id,
        ["fields", index],
        updated,
        "列定義を更新",
      );
      return { op: kind, table: tableName, field: updated };
    }

    case "table.column.remove": {
      const tableName = String(op.table ?? "").trim();
      ensureTableEditable(snapshot, tableName);
      const table = findTable(snapshot, tableName);
      const colName = String(op.name ?? "").trim();
      const index = table.fields.findIndex((f) => f.name === colName);
      if (index < 0)
        throw new Error(`テーブル「${tableName}」に列「${colName}」がありません。`);
      session.removeField(table.source.id, index);
      return { op: kind, table: tableName, removed: colName };
    }

    case "table.column.rename": {
      const tableName = String(op.table ?? "").trim();
      ensureTableEditable(snapshot, tableName);
      const table = findTable(snapshot, tableName);
      const oldName = String(op.old ?? op.name ?? "").trim();
      const newName = String(op.new ?? op.newName ?? "").trim();
      ensureTokenEditable(snapshot, oldName);
      const index = table.fields.findIndex((f) => f.name === oldName);
      if (index < 0)
        throw new Error(`テーブル「${tableName}」に列「${oldName}」がありません。`);
      session.renameField(table.source.id, index, newName);
      return { op: kind, table: tableName, from: oldName, to: newName };
    }

    case "table.column.move": {
      const tableName = String(op.table ?? "").trim();
      const table = findTable(snapshot, tableName);
      const colName = String(op.name ?? "").trim();
      const from = table.fields.findIndex((f) => f.name === colName);
      if (from < 0)
        throw new Error(`テーブル「${tableName}」に列「${colName}」がありません。`);
      const to = Number(op.to);
      if (!Number.isInteger(to) || to < 0 || to >= table.fields.length)
        throw new Error(`移動先インデックスが範囲外です: ${op.to}`);
      const fieldsWithExplicitKeys = table.fields.map((f, i) => ({
        ...f,
        key: Number(f.key ?? i),
      }));
      const [moved] = fieldsWithExplicitKeys.splice(from, 1);
      fieldsWithExplicitKeys.splice(to, 0, moved);
      session.update(
        table.source.id,
        ["fields"],
        fieldsWithExplicitKeys,
        "列を並替え",
      );
      return { op: kind, table: tableName, column: colName, from, to };
    }

    case "table.key.setPk": {
      const tableName = String(op.table ?? "").trim();
      const table = findTable(snapshot, tableName);
      const fields = Array.isArray(op.fields)
        ? op.fields.map(String)
        : splitCsv(String(op.fields ?? ""));
      if (!fields.length) throw new Error("主キーの項目を指定してください。");
      session.update(
        table.source.id,
        ["primaryKey"],
        { fields },
        "主キーを設定",
      );
      return { op: kind, table: tableName, primaryKey: fields };
    }

    case "table.key.addSk": {
      const tableName = String(op.table ?? "").trim();
      const table = findTable(snapshot, tableName);
      const fields = Array.isArray(op.fields)
        ? op.fields.map(String)
        : splitCsv(String(op.fields ?? ""));
      if (!fields.length) throw new Error("副キーの項目を指定してください。");
      const current = list(table.source.data.secondaryKeys);
      const entry = {
        fields,
        ...(op.nonUnique ? { nonUnique: true } : {}),
      };
      session.update(
        table.source.id,
        ["secondaryKeys"],
        [...current, entry],
        "副キーを追加",
      );
      return { op: kind, table: tableName, secondaryKey: entry };
    }

    case "table.key.removeSk": {
      const tableName = String(op.table ?? "").trim();
      const table = findTable(snapshot, tableName);
      const current = list(table.source.data.secondaryKeys);
      const index = Number(op.index);
      if (!Number.isInteger(index) || index < 0 || index >= current.length)
        throw new Error(`副キーの番号が範囲外です: ${op.index}`);
      session.update(
        table.source.id,
        ["secondaryKeys"],
        current.filter((_, i) => i !== index),
        "副キーを削除",
      );
      return { op: kind, table: tableName, removedIndex: index };
    }

    case "table.ref.add": {
      const tableName = String(op.table ?? "").trim();
      const table = findTable(snapshot, tableName);
      const refName = String(op.name ?? "").trim();
      const ownFields = Array.isArray(op.fields)
        ? op.fields.map(String)
        : splitCsv(String(op.fields ?? ""));
      const targetTable = String(op.targetTable ?? op.target?.table ?? "").trim();
      const targetFields = Array.isArray(op.targetFields ?? op.target?.fields)
        ? (op.targetFields ?? op.target?.fields).map(String)
        : splitCsv(String(op.targetFields ?? ""));
      if (!refName || !ownFields.length || !targetTable || !targetFields.length)
        throw new Error(
          "参照名、自テーブル項目、参照先テーブル、参照先項目をすべて指定してください。",
        );
      const current = list(table.source.data.references);
      if (current.some((r: any) => r?.name === refName))
        throw new Error(`参照「${refName}」は既に存在します。`);
      const entry = {
        name: refName,
        ...(op.csharpName ? { csharpName: String(op.csharpName) } : {}),
        fields: ownFields,
        target: { table: targetTable, fields: targetFields },
      };
      session.update(
        table.source.id,
        ["references"],
        [...current, entry],
        "参照を追加",
      );
      return { op: kind, table: tableName, reference: entry };
    }

    case "table.ref.remove": {
      const tableName = String(op.table ?? "").trim();
      const table = findTable(snapshot, tableName);
      const refName = String(op.name ?? "").trim();
      const current = list(table.source.data.references);
      if (!current.some((r: any) => r?.name === refName))
        throw new Error(`参照「${refName}」が見つかりません。`);
      session.update(
        table.source.id,
        ["references"],
        current.filter((r: any) => r?.name !== refName),
        "参照を削除",
      );
      return { op: kind, table: tableName, removed: refName };
    }

    case "type.create": {
      const rawKind = String(op.kind ?? op.typeKind ?? "").trim();
      const normalizedKind =
        rawKind === "value-object" ? "valueObject" : rawKind;
      if (
        !["enum", "flags", "custom", "valueObject"].includes(normalizedKind)
      )
        throw new Error(
          `型の種類は enum / flags / custom / valueObject を指定してください: ${rawKind}`,
        );
      const name = String(op.name ?? "").trim();
      if (!name) throw new Error("型名を指定してください。");
      if (typeSource(snapshot, name))
        throw new Error(`型「${name}」は既に存在します。`);
      const relPath = normalizeSourcePath(
        op.path ? String(op.path) : `${name}.yaml`,
      );
      ensureUniqueSourcePath(snapshot, relPath);

      let definition: Record<string, any>;
      if (normalizedKind === "valueObject") {
        definition = { underlying: String(op.underlying ?? "int") };
      } else if (normalizedKind === "custom") {
        const rawFields: any[] = Array.isArray(op.fields) ? op.fields : [];
        definition = {
          fields: rawFields.map((f, i) =>
            typeof f === "string"
              ? {
                  key: i,
                  name: f.split(":")[0].trim(),
                  type: (f.split(":")[1] ?? "int").trim(),
                }
              : {
                  key: Number(f.key ?? i),
                  name: String(f.name),
                  type: String(f.type ?? "int"),
                  ...(f.array ? { array: true } : {}),
                  ...(f.nullable ? { nullable: true } : {}),
                },
          ),
        };
      } else if (normalizedKind === "flags") {
        const underlying = String(op.underlying ?? "uint");
        const inputMembers: any[] = Array.isArray(op.members)
          ? op.members
          : typeof op.members === "string"
            ? splitCsv(op.members)
            : [];
        const members: { name: string; value: bigint | number }[] = [
          { name: "None", value: 0 },
        ];
        let bit = 0n;
        for (const item of inputMembers) {
          if (typeof item === "string") {
            const [mName, mVal] = item.split("=");
            if (mName.trim() === "None") continue;
            const value =
              mVal !== undefined ? BigInt(mVal.trim()) : 1n << bit++;
            members.push({ name: mName.trim(), value });
          } else if (isObject(item) && item.name !== "None") {
            const value =
              item.value !== undefined ? BigInt(item.value) : 1n << bit++;
            members.push({ name: String(item.name), value });
          }
        }
        definition = { underlying, members };
      } else {
        const underlying = String(op.underlying ?? "int");
        const inputMembers: any[] = Array.isArray(op.members)
          ? op.members
          : typeof op.members === "string"
            ? splitCsv(op.members)
            : ["NewMember"];
        const members = inputMembers.map((item, i) => {
          if (typeof item === "string") {
            const [mName, mVal] = item.split("=");
            return {
              name: mName.trim(),
              value: mVal !== undefined ? BigInt(mVal.trim()) : i,
            };
          }
          return {
            name: String(item.name),
            value: item.value !== undefined ? BigInt(item.value) : i,
          };
        });
        definition = { underlying, members };
      }

      const data = {
        kind: "type",
        name,
        [normalizedKind]: definition,
      };
      session.change("型を作成", (s) => ({
        ...s,
        folders: addFoldersForPath(s.folders, relPath),
        sources: [...s.sources, { id: uid(), path: relPath, data }],
      }));
      return { op: kind, type: name, kindName: normalizedKind, path: relPath };
    }

    case "type.delete": {
      const name = String(op.name ?? "").trim();
      const source = findType(snapshot, name);
      session.remove([source.path]);
      return { op: kind, removed: name, path: source.path };
    }

    case "type.setUnderlying": {
      const name = String(op.name ?? "").trim();
      const underlying = String(op.underlying ?? "").trim();
      const source = findType(snapshot, name);
      const tKind = typeKind(source);
      if (!tKind || tKind === "custom")
        throw new Error(`型「${name}」には基底型（underlying）を設定できません。`);
      session.update(
        source.id,
        [tKind, "underlying"],
        underlying,
        "基底型を変更",
      );
      return { op: kind, type: name, underlying };
    }

    case "type.member.add": {
      const typeName = String(op.type ?? op.name ?? "").trim();
      const memberName = String(op.member ?? op.memberName ?? "").trim();
      const source = findType(snapshot, typeName);
      const tKind = typeKind(source);
      if (tKind !== "enum" && tKind !== "flags")
        throw new Error(`型「${typeName}」はEnumまたはFlagsではありません。`);
      const members = list(source.data[tKind].members);
      if (members.some((m: any) => m?.name === memberName))
        throw new Error(`候補「${memberName}」は既に存在します。`);
      const value =
        op.value !== undefined
          ? BigInt(op.value)
          : tKind === "flags"
            ? nextFlagBit(source)
            : nextEnumValue(source);
      const entry = { name: memberName, value };
      session.update(
        source.id,
        [tKind, "members"],
        [...members, entry],
        "候補を追加",
      );
      return { op: kind, type: typeName, member: entry };
    }

    case "type.member.set": {
      const typeName = String(op.type ?? op.name ?? "").trim();
      const memberName = String(op.member ?? op.memberName ?? "").trim();
      const source = findType(snapshot, typeName);
      const tKind = typeKind(source);
      if (tKind !== "enum" && tKind !== "flags")
        throw new Error(`型「${typeName}」はEnumまたはFlagsではありません。`);
      const members = list(source.data[tKind].members);
      const index = members.findIndex((m: any) => m?.name === memberName);
      if (index < 0)
        throw new Error(`候補「${memberName}」が見つかりません。`);
      if (op.value === undefined)
        throw new Error("変更する値（--value）を指定してください。");
      const value = BigInt(op.value);
      if (tKind === "flags" && memberName === "None" && value !== 0n)
        throw new Error("Flagsの予約候補「None」の値は0から変更できません。");
      session.update(
        source.id,
        [tKind, "members", index, "value"],
        value,
        "候補の値を変更",
      );
      return { op: kind, type: typeName, member: memberName, value };
    }

    case "type.member.remove": {
      const typeName = String(op.type ?? op.name ?? "").trim();
      const memberName = String(op.member ?? op.memberName ?? "").trim();
      const source = findType(snapshot, typeName);
      const tKind = typeKind(source);
      if (tKind !== "enum" && tKind !== "flags")
        throw new Error(`型「${typeName}」はEnumまたはFlagsではありません。`);
      if (tKind === "flags" && memberName === "None")
        throw new Error("Flagsの予約候補「None」は削除できません。");
      const members = list(source.data[tKind].members);
      const index = members.findIndex((m: any) => m?.name === memberName);
      if (index < 0)
        throw new Error(`候補「${memberName}」が見つかりません。`);
      session.update(
        source.id,
        [tKind, "members"],
        members.filter((_, i) => i !== index),
        "候補を削除",
      );
      return { op: kind, type: typeName, removed: memberName };
    }

    case "type.member.rename":
    case "rename.member": {
      const typeName = String(op.type ?? op.target ?? op.name ?? "").trim();
      const oldName = String(op.old ?? op.member ?? "").trim();
      const newName = String(op.new ?? op.newName ?? "").trim();
      ensureTokenEditable(snapshot, oldName);
      const source = findType(snapshot, typeName);
      const tKind = typeKind(source);
      if (tKind !== "enum" && tKind !== "flags")
        throw new Error(`型「${typeName}」はEnumまたはFlagsではありません。`);
      if (tKind === "flags" && oldName === "None")
        throw new Error("Flagsの予約候補「None」は名前変更できません。");
      const members = list(source.data[tKind].members);
      const index = members.findIndex((m: any) => m?.name === oldName);
      if (index < 0)
        throw new Error(`候補「${oldName}」が見つかりません。`);
      if (members.some((m: any, i: number) => i !== index && m?.name === newName))
        throw new Error(`候補「${newName}」は既に存在します。`);
      session.renameMember(source.id, index, newName);
      return { op: kind, type: typeName, from: oldName, to: newName };
    }

    case "type.member.move": {
      const typeName = String(op.type ?? op.name ?? "").trim();
      const memberName = String(op.member ?? op.memberName ?? "").trim();
      const source = findType(snapshot, typeName);
      const tKind = typeKind(source);
      if (tKind !== "enum" && tKind !== "flags")
        throw new Error(`型「${typeName}」はEnumまたはFlagsではありません。`);
      const members = [...list(source.data[tKind].members)];
      const from = members.findIndex((m: any) => m?.name === memberName);
      if (from < 0)
        throw new Error(`候補「${memberName}」が見つかりません。`);
      const to = Number(op.to);
      if (!Number.isInteger(to) || to < 0 || to >= members.length)
        throw new Error(`移動先インデックスが範囲外です: ${op.to}`);
      if (tKind === "flags" && (memberName === "None" || to === 0))
        throw new Error("Flagsの予約候補「None」は先頭（0番目）に固定されます。");
      const [moved] = members.splice(from, 1);
      members.splice(to, 0, moved);
      session.update(
        source.id,
        [tKind, "members"],
        members,
        "候補を並替え",
      );
      return { op: kind, type: typeName, member: memberName, from, to };
    }

    case "type.field.add": {
      const typeName = String(op.type ?? "").trim();
      const fieldName = String(op.name ?? op.field ?? "").trim();
      const fieldType = String(op.fieldType ?? "").trim();
      ensureTokenEditable(snapshot, typeName);
      const source = findType(snapshot, typeName);
      if (typeKind(source) !== "custom")
        throw new Error(`型「${typeName}」はCustom構造体ではありません。`);
      const fields = fieldsOf(source);
      if (fields.some((f) => f.name === fieldName))
        throw new Error(`フィールド「${fieldName}」は既に存在します。`);
      const key =
        op.key !== undefined ? Number(op.key) : nextFieldKey(fields);
      const field: Field = {
        key,
        name: fieldName,
        type: fieldType,
        ...(op.array ? { array: true } : {}),
        ...(op.nullable ? { nullable: true } : {}),
      };
      const nextFields = [...fields, field];
      session.change("構造体フィールドを追加", (s) => {
        const updatedSources = mapRecords(s, null, typeName, (val) =>
          isObject(val) && !(fieldName in val)
            ? { ...val, [fieldName]: defaultValue(s, field) }
            : val,
        );
        return {
          ...s,
          sources: updatedSources.map((src) =>
            src.id === source.id
              ? {
                  ...src,
                  data: setIn(src.data, ["custom", "fields"], nextFields),
                }
              : src,
          ),
        };
      });
      return { op: kind, type: typeName, field };
    }

    case "type.field.set": {
      const typeName = String(op.type ?? "").trim();
      const fieldName = String(op.name ?? op.field ?? "").trim();
      const source = findType(snapshot, typeName);
      if (typeKind(source) !== "custom")
        throw new Error(`型「${typeName}」はCustom構造体ではありません。`);
      const fields = fieldsOf(source);
      const index = fields.findIndex((f) => f.name === fieldName);
      if (index < 0)
        throw new Error(`フィールド「${fieldName}」が見つかりません。`);
      const updated: Field = { ...fields[index] };
      if (op.fieldType !== undefined) updated.type = String(op.fieldType);
      if (op.key !== undefined) updated.key = Number(op.key);
      if (op.array !== undefined) {
        if (op.array) updated.array = true;
        else delete updated.array;
      }
      if (op.nullable !== undefined) {
        if (op.nullable) updated.nullable = true;
        else delete updated.nullable;
      }
      session.update(
        source.id,
        ["custom", "fields", index],
        updated,
        "構造体フィールド定義を更新",
      );
      return { op: kind, type: typeName, field: updated };
    }

    case "type.field.remove": {
      const typeName = String(op.type ?? "").trim();
      const fieldName = String(op.name ?? op.field ?? "").trim();
      ensureTokenEditable(snapshot, typeName);
      const source = findType(snapshot, typeName);
      if (typeKind(source) !== "custom")
        throw new Error(`型「${typeName}」はCustom構造体ではありません。`);
      const fields = fieldsOf(source);
      const index = fields.findIndex((f) => f.name === fieldName);
      if (index < 0)
        throw new Error(`フィールド「${fieldName}」が見つかりません。`);
      session.removeField(source.id, index);
      return { op: kind, type: typeName, removed: fieldName };
    }

    case "type.field.rename": {
      const typeName = String(op.type ?? "").trim();
      const oldName = String(op.old ?? op.name ?? "").trim();
      const newName = String(op.new ?? op.newName ?? "").trim();
      ensureTokenEditable(snapshot, typeName);
      ensureTokenEditable(snapshot, oldName);
      const source = findType(snapshot, typeName);
      if (typeKind(source) !== "custom")
        throw new Error(`型「${typeName}」はCustom構造体ではありません。`);
      const fields = fieldsOf(source);
      const index = fields.findIndex((f) => f.name === oldName);
      if (index < 0)
        throw new Error(`フィールド「${oldName}」が見つかりません。`);
      session.renameField(source.id, index, newName);
      return { op: kind, type: typeName, from: oldName, to: newName };
    }

    case "type.field.move": {
      const typeName = String(op.type ?? "").trim();
      const fieldName = String(op.name ?? op.field ?? "").trim();
      const source = findType(snapshot, typeName);
      if (typeKind(source) !== "custom")
        throw new Error(`型「${typeName}」はCustom構造体ではありません。`);
      const fields = fieldsOf(source).map((f, i) => ({
        ...f,
        key: Number(f.key ?? i),
      }));
      const from = fields.findIndex((f) => f.name === fieldName);
      if (from < 0)
        throw new Error(`フィールド「${fieldName}」が見つかりません。`);
      const to = Number(op.to);
      if (!Number.isInteger(to) || to < 0 || to >= fields.length)
        throw new Error(`移動先インデックスが範囲外です: ${op.to}`);
      const [moved] = fields.splice(from, 1);
      fields.splice(to, 0, moved);
      session.update(
        source.id,
        ["custom", "fields"],
        fields,
        "構造体フィールドを並替え",
      );
      return { op: kind, type: typeName, field: fieldName, from, to };
    }

    case "rename.table": {
      const oldName = String(op.old ?? "").trim();
      const newName = String(op.new ?? "").trim();
      ensureTableEditable(snapshot, oldName);
      ensureTokenEditable(snapshot, oldName);
      const table = findTable(snapshot, oldName);
      if (tables(snapshot).some((t) => t.name === newName && t !== table))
        throw new Error(`テーブル「${newName}」は既に存在します。`);
      session.renameDeclaration(table.source.id, newName);
      return { op: kind, from: oldName, to: newName };
    }

    case "rename.type": {
      const oldName = String(op.old ?? "").trim();
      const newName = String(op.new ?? "").trim();
      ensureTokenEditable(snapshot, oldName);
      const source = findType(snapshot, oldName);
      if (typeSource(snapshot, newName) && typeSource(snapshot, newName) !== source)
        throw new Error(`型「${newName}」は既に存在します。`);
      session.renameDeclaration(source.id, newName);
      return { op: kind, from: oldName, to: newName };
    }

    case "rename.field": {
      const targetName = String(op.target ?? op.table ?? op.type ?? "").trim();
      const oldName = String(op.old ?? "").trim();
      const newName = String(op.new ?? "").trim();
      ensureTokenEditable(snapshot, oldName);
      const table = tables(snapshot).find((t) => t.name === targetName);
      if (table) {
        ensureTableEditable(snapshot, targetName);
        const index = table.fields.findIndex((f) => f.name === oldName);
        if (index < 0)
          throw new Error(`テーブル「${targetName}」に列「${oldName}」がありません。`);
        session.renameField(table.source.id, index, newName);
        return { op: kind, target: targetName, from: oldName, to: newName };
      }
      const type = findType(snapshot, targetName);
      if (typeKind(type) !== "custom")
        throw new Error(`型「${targetName}」はフィールドを持つ構造体ではありません。`);
      const fields = fieldsOf(type);
      const index = fields.findIndex((f) => f.name === oldName);
      if (index < 0)
        throw new Error(`構造体「${targetName}」にフィールド「${oldName}」がありません。`);
      session.renameField(type.id, index, newName);
      return { op: kind, target: targetName, from: oldName, to: newName };
    }

    case "source.move": {
      const from = normalizeSourcePath(String(op.from ?? op.old ?? ""));
      const to = normalizeSourcePath(String(op.to ?? op.new ?? ""));
      if (!snapshot.sources.some((s) => s.path === from))
        throw new Error(`ファイル「sources/${from}」が見つかりません。`);
      session.change("フォルダを追加", (s) => ({
        ...s,
        folders: addFoldersForPath(s.folders, to),
      }));
      session.move(from, to);
      return { op: kind, from, to };
    }

    case "source.delete": {
      const target = normalizeSourcePath(String(op.path ?? ""));
      if (
        !snapshot.sources.some(
          (s) => s.path === target || s.path.startsWith(`${target}/`),
        )
      )
        throw new Error(`対象「sources/${target}」が見つかりません。`);
      session.remove([target]);
      return { op: kind, removed: target };
    }

    case "record.add": {
      const tableName = String(op.table ?? "").trim();
      ensureTableEditable(snapshot, tableName);
      const table = findTable(snapshot, tableName);
      let targetSource: Source;
      if (op.source) {
        const rel = normalizeSourcePath(String(op.source));
        const found = table.sources.find((s) => s.path === rel);
        if (!found)
          throw new Error(
            `テーブル「${tableName}」のデータファイル「${rel}」が見つかりません。候補: ${table.sources.map((s) => s.path).join(", ")}`,
          );
        targetSource = found;
      } else {
        if (table.sources.length > 1)
          throw new Error(
            `テーブル「${tableName}」は複数のデータファイルに分かれています。追加先を --source で指定してください（候補: ${table.sources.map((s) => s.path).join(", ")}）。`,
          );
        targetSource = table.sources[0] ?? table.source;
      }
      ensureSourceEditable(targetSource);

      const inputRecords: Data[] = Array.isArray(op.records)
        ? op.records
        : [op.values ?? {}];
      const pkFields: string[] = list(table.source.data.primaryKey?.fields);
      const existingRows = table.sources.flatMap((s) =>
        Array.isArray(s.data?.records) ? s.data.records : [],
      );
      const existingPkSet = new Set(
        pkFields.length
          ? existingRows.map((r: any) =>
              fingerprint(pkFields.map((k) => r?.[k])),
            )
          : [],
      );
      const allocator = recordAllocator(snapshot, table);
      const added: Data[] = [];

      for (const rawInput of inputRecords) {
        let baseRecord = Object.fromEntries(
          table.fields.map((f) => [f.name, defaultValue(snapshot, f)]),
        );
        const explicitPkProvided =
          pkFields.length > 0 &&
          isObject(rawInput) &&
          pkFields.every((k) => rawInput[k] !== undefined);

        if (isObject(rawInput)) {
          for (const [rawKey, rawVal] of Object.entries(rawInput)) {
            if (rawKey === "$tags") {
              baseRecord.$tags = Array.isArray(rawVal)
                ? rawVal.map(String)
                : splitCsv(String(rawVal));
              continue;
            }
            const steps = parseFieldPath(rawKey);
            const targetField = resolveFieldAtPath(snapshot, table.fields, steps);
            const coerced = coerceScalarOrStructured(
              snapshot,
              targetField,
              rawVal,
            );
            baseRecord = setByPath(baseRecord, steps, coerced);
          }
        }
        if (Array.isArray(op.assignments)) {
          for (const assign of op.assignments as {
            path: PathStep[];
            value: Data;
          }[]) {
            baseRecord = setByPath(baseRecord, assign.path, assign.value);
          }
        }
        const mentionsPk =
          pkFields.length > 0 &&
          pkFields.every(
            (k) =>
              (isObject(rawInput) && rawInput[k] !== undefined) ||
              (Array.isArray(op.assignments) &&
                op.assignments.some(
                  (a: any) => a.path?.length === 1 && a.path[0] === k,
                )),
          );
        if (op.tags) {
          const tags = Array.isArray(op.tags)
            ? op.tags.map(String)
            : splitCsv(String(op.tags));
          if (tags.length) baseRecord.$tags = tags;
        }

        if ((explicitPkProvided || mentionsPk) && !op.autoKey) {
          const sig = fingerprint(pkFields.map((k) => baseRecord[k]));
          if (existingPkSet.has(sig))
            throw new Error(
              `指定された主キー（${pkFields.map((k) => `${k}=${text(baseRecord[k])}`).join(", ")}）は既に存在します。`,
            );
          existingPkSet.add(sig);
          const allocated = allocator(baseRecord);
          if (baseRecord.$tags) allocated.$tags = baseRecord.$tags;
          added.push(allocated);
        } else {
          const omittedPk = pkFields.filter(
            (k) =>
              !(isObject(rawInput) && rawInput[k] !== undefined) &&
              !(
                Array.isArray(op.assignments) &&
                op.assignments.some(
                  (a: any) => a.path?.length === 1 && a.path[0] === k,
                )
              ),
          );
          seedOmittedPrimaryKeys(
            snapshot,
            table,
            [...existingRows, ...added],
            baseRecord,
            omittedPk,
          );
          const allocated = allocator(baseRecord);
          if (baseRecord.$tags) allocated.$tags = baseRecord.$tags;
          existingPkSet.add(fingerprint(pkFields.map((k) => allocated[k])));
          added.push(allocated);
        }
      }

      const currentRecords = list(targetSource.data.records);
      session.update(
        targetSource.id,
        ["records"],
        [...currentRecords, ...added],
        "レコードを追加",
      );
      return {
        op: kind,
        table: tableName,
        source: targetSource.path,
        addedCount: added.length,
        records: added,
      };
    }

    case "record.set": {
      const tableName = String(op.table ?? "").trim();
      const table = findTable(snapshot, tableName);
      const pkFields: string[] = list(table.source.data.primaryKey?.fields);
      const whereList: string[] = Array.isArray(op.where)
        ? op.where.map(String)
        : op.where
          ? [String(op.where)]
          : [];
      if (op.pk === undefined && !whereList.length && !op.all)
        throw new Error(
          "更新対象を --pk または --where（全件なら --all）で指定してください。",
        );
      const pkExpected =
        op.pk !== undefined
          ? parsePkSelector(snapshot, table.fields, pkFields, op.pk)
          : null;
      const sourceFilter = op.source
        ? normalizeSourcePath(String(op.source))
        : null;

      let updatedCount = 0;
      const nextSources = snapshot.sources.map((src) => {
        if (!table.sources.some((ts) => ts.id === src.id)) return src;
        if (sourceFilter && src.path !== sourceFilter) return src;
        ensureSourceEditable(src);
        const records = list(src.data.records);
        let sourceChanged = false;
        const nextRecords = records.map((row: Data) => {
          if (!isObject(row)) return row;
          if (pkExpected && !matchesPk(row, pkFields, pkExpected)) return row;
          if (whereList.length && !matchesWhere(row, whereList)) return row;

          let nextRow: Record<string, Data> = structuredClone(row);
          if (isObject(op.set)) {
            for (const [rawKey, rawVal] of Object.entries(op.set)) {
              const steps = parseFieldPath(rawKey);
              const targetField = resolveFieldAtPath(
                snapshot,
                table.fields,
                steps,
              );
              const coerced = coerceScalarOrStructured(
                snapshot,
                targetField,
                rawVal,
              );
              nextRow = setByPath(nextRow, steps, coerced);
            }
          }
          if (Array.isArray(op.assignments)) {
            for (const assign of op.assignments as {
              path: PathStep[];
              value: Data;
            }[]) {
              nextRow = setByPath(nextRow, assign.path, assign.value);
            }
          }
          if (op.tags !== undefined) {
            const tags = Array.isArray(op.tags)
              ? op.tags.map(String)
              : splitCsv(String(op.tags));
            if (tags.length) nextRow.$tags = tags;
            else delete nextRow.$tags;
          }
          if (op.addTags) {
            const add = Array.isArray(op.addTags)
              ? op.addTags.map(String)
              : splitCsv(String(op.addTags));
            const existing = Array.isArray(nextRow.$tags)
              ? nextRow.$tags.map(String)
              : [];
            nextRow.$tags = [...new Set([...existing, ...add])];
          }
          if (op.removeTags) {
            const rem = new Set(
              Array.isArray(op.removeTags)
                ? op.removeTags.map(String)
                : splitCsv(String(op.removeTags)),
            );
            const existing = Array.isArray(nextRow.$tags)
              ? nextRow.$tags.map(String)
              : [];
            const filtered = existing.filter((t) => !rem.has(t));
            if (filtered.length) nextRow.$tags = filtered;
            else delete nextRow.$tags;
          }
          updatedCount++;
          sourceChanged = true;
          return nextRow;
        });
        return sourceChanged
          ? { ...src, data: { ...src.data, records: nextRecords } }
          : src;
      });

      if (updatedCount === 0)
        throw new Error("指定された条件に一致するレコードが見つかりません。");

      session.change("レコードを更新", (s) => ({
        ...s,
        sources: nextSources,
      }));
      return { op: kind, table: tableName, updatedCount };
    }

    case "record.delete": {
      const tableName = String(op.table ?? "").trim();
      const table = findTable(snapshot, tableName);
      const pkFields: string[] = list(table.source.data.primaryKey?.fields);
      const whereList: string[] = Array.isArray(op.where)
        ? op.where.map(String)
        : op.where
          ? [String(op.where)]
          : [];
      if (op.pk === undefined && !whereList.length && !op.all)
        throw new Error(
          "削除対象を --pk、--where、または --all で指定してください。",
        );
      const pkExpected =
        op.pk !== undefined
          ? parsePkSelector(snapshot, table.fields, pkFields, op.pk)
          : null;
      const sourceFilter = op.source
        ? normalizeSourcePath(String(op.source))
        : null;

      let deletedCount = 0;
      const nextSources = snapshot.sources.map((src) => {
        if (!table.sources.some((ts) => ts.id === src.id)) return src;
        if (sourceFilter && src.path !== sourceFilter) return src;
        ensureSourceEditable(src);
        const records = list(src.data.records);
        const remaining = records.filter((row: Data) => {
          if (!isObject(row)) return true;
          if (pkExpected && !matchesPk(row, pkFields, pkExpected)) return true;
          if (whereList.length && !matchesWhere(row, whereList)) return true;
          deletedCount++;
          return false;
        });
        return remaining.length !== records.length
          ? { ...src, data: { ...src.data, records: remaining } }
          : src;
      });

      if (deletedCount === 0)
        throw new Error("削除対象のレコードが見つかりません。");

      session.change("レコードを削除", (s) => ({
        ...s,
        sources: nextSources,
      }));
      return { op: kind, table: tableName, deletedCount };
    }

    case "fix": {
      const targetTable = op.table ? String(op.table).trim() : null;
      if (targetTable) {
        ensureTableEditable(snapshot, targetTable);
        findTable(snapshot, targetTable);
      }
      let touchedFiles = 0;
      session.change("構造の補完とYAML正規化", (s) => {
        const tableMap = new Map(tables(s).map((t) => [t.name, t]));
        const nextSources = s.sources.map((src) => {
          if (src.syntaxError || !isObject(src.data)) return src;
          if (
            targetTable &&
            !(
              (src.data.kind === "schema" || src.data.kind === "data") &&
              String(src.data.table) === targetTable
            )
          )
            return src;

          let data = src.data;
          if (data.kind === "schema" && Array.isArray(data.fields)) {
            const fixedFields = data.fields.map((f: any, i: number) =>
              isObject(f) && f.key === undefined ? { key: i, ...f } : f,
            );
            if (fixedFields.some((f: any, i: number) => f !== data.fields[i]))
              data = setIn(data, ["fields"], fixedFields);
          }
          if (
            data.kind === "type" &&
            data.custom &&
            Array.isArray(data.custom.fields)
          ) {
            const fixedFields = data.custom.fields.map((f: any, i: number) =>
              isObject(f) && f.key === undefined ? { key: i, ...f } : f,
            );
            if (
              fixedFields.some(
                (f: any, i: number) => f !== data.custom.fields[i],
              )
            )
              data = setIn(data, ["custom", "fields"], fixedFields);
          }
          if (
            (data.kind === "schema" || data.kind === "data") &&
            Array.isArray(data.records)
          ) {
            const tbl = tableMap.get(String(data.table));
            if (tbl) {
              const nextRecords = data.records.map((row: Data) =>
                backfillDefinedFields(s, tbl.fields, row),
              );
              if (
                nextRecords.some((r: Data, i: number) => r !== data.records[i])
              )
                data = { ...data, records: nextRecords };
            }
          }

          const candidateSource: Source =
            data !== src.data ? { ...src, data } : { ...src, data: { ...data } };
          const formatted = serialize(candidateSource);
          if (data !== src.data || formatted !== src.formatText) {
            touchedFiles++;
            return candidateSource;
          }
          return src;
        });
        return { ...s, sources: nextSources };
      });
      return { op: kind, touchedFiles };
    }

    default:
      throw new Error(`未知の操作です: ${kind}`);
  }
}
