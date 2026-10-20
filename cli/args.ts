import { parseDocument } from "yaml";
import {
  type Data,
  type Field,
  type Snapshot,
  fieldsOf,
  isObject,
  primitives,
  text,
  typeKind,
  typeSource,
} from "../src/core/types";
import { flagsChange, primitiveOf } from "../src/core/schema";

export interface ParsedArgs {
  positionals: string[];
  flags: Record<string, (string | boolean)[]>;
}

const booleanFlags = new Set([
  "json",
  "help",
  "h",
  "dry-run",
  "strict",
  "array",
  "nullable",
  "no-array",
  "no-nullable",
  "non-unique",
  "all",
  "auto-key",
  "build",
]);

export function parseArgs(argv: string[]): ParsedArgs {
  const positionals: string[] = [];
  const flags: Record<string, (string | boolean)[]> = {};
  const pushFlag = (key: string, value: string | boolean) => {
    flags[key] ??= [];
    flags[key].push(value);
  };
  for (let i = 0; i < argv.length; i++) {
    const token = argv[i];
    if (token === "--") {
      positionals.push(...argv.slice(i + 1));
      break;
    }
    if (token === "-h") {
      pushFlag("help", true);
      continue;
    }
    if (token.startsWith("--")) {
      const eq = token.indexOf("=");
      if (eq >= 0) {
        const key = token.slice(2, eq);
        const val = token.slice(eq + 1);
        pushFlag(key, val);
        continue;
      }
      const key = token.slice(2);
      if (booleanFlags.has(key)) {
        const next = argv[i + 1];
        if (next === "true" || next === "false") {
          pushFlag(key, next === "true");
          i++;
        } else {
          pushFlag(key, true);
        }
        continue;
      }
      const next = argv[i + 1];
      if (next !== undefined && !next.startsWith("--")) {
        pushFlag(key, next);
        i++;
      } else {
        pushFlag(key, true);
      }
      continue;
    }
    positionals.push(token);
  }
  return { positionals, flags };
}

export function flagString(
  flags: ParsedArgs["flags"],
  name: string,
): string | undefined {
  const last = flags[name]?.at(-1);
  return typeof last === "string" ? last : undefined;
}

export function flagStrings(
  flags: ParsedArgs["flags"],
  name: string,
): string[] {
  return (flags[name] ?? []).filter((v): v is string => typeof v === "string");
}

export function flagBool(
  flags: ParsedArgs["flags"],
  name: string,
): boolean | undefined {
  if (`no-${name}` in flags) return false;
  const last = flags[name]?.at(-1);
  if (typeof last === "boolean") return last;
  if (last === "true") return true;
  if (last === "false") return false;
  return undefined;
}

export function splitCsv(input?: string): string[] {
  if (!input) return [];
  return input
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
}

export function parseStructured(input: string): Data {
  const doc = parseDocument(input, {
    intAsBigInt: true,
    keepSourceTokens: false,
    uniqueKeys: true,
  });
  if (doc.errors.length)
    throw new Error(
      `入力データの構文が不正です: ${doc.errors.map((e) => e.message).join("; ")}`,
    );
  return doc.toJS({ maxAliasCount: 0 });
}

export function stringifyJson(value: unknown, pretty = true): string {
  const rawJSON = (JSON as any).rawJSON as
    | ((text: string) => unknown)
    | undefined;
  return JSON.stringify(
    value,
    (_, v) => {
      if (typeof v === "bigint") {
        if (
          v >= BigInt(Number.MIN_SAFE_INTEGER) &&
          v <= BigInt(Number.MAX_SAFE_INTEGER)
        )
          return Number(v);
        return typeof rawJSON === "function" ? rawJSON(String(v)) : String(v);
      }
      return v;
    },
    pretty ? 2 : undefined,
  );
}

export type PathStep = string | number;

export function parseFieldPath(raw: string): PathStep[] {
  const steps: PathStep[] = [];
  const pattern = /([^.[\]]+)|\[(\d+|\+)\]/g;
  let match: RegExpExecArray | null;
  let consumed = "";
  while ((match = pattern.exec(raw)) !== null) {
    consumed += match[0];
    if (match[1] !== undefined) {
      if (consumed.length > match[0].length && raw[match.index - 1] === ".")
        consumed = raw.slice(0, pattern.lastIndex);
      steps.push(match[1]);
    } else if (match[2] === "+") {
      steps.push("+");
    } else {
      steps.push(Number(match[2]));
    }
  }
  const normalized = raw.replace(/\./g, "");
  const check = steps
    .map((s) => (typeof s === "number" || s === "+" ? `[${s}]` : s))
    .join("");
  if (!steps.length || normalized !== check)
    throw new Error(`フィールドパスが不正です: ${raw}`);
  return steps;
}

export function resolveFieldAtPath(
  snapshot: Snapshot,
  tableFields: Field[],
  steps: PathStep[],
): Field | undefined {
  let current: Field | undefined;
  let fields: Field[] | undefined = tableFields;
  for (let i = 0; i < steps.length; i++) {
    const step = steps[i];
    if (typeof step === "number" || step === "+") {
      if (!current?.array) return undefined;
      current = { ...current, array: false, nullable: false };
      const type = typeSource(snapshot, current.type);
      fields = type?.data?.custom ? fieldsOf(type) : undefined;
      continue;
    }
    if (!fields) return undefined;
    current = fields.find((f) => f.name === step);
    if (!current) return undefined;
    if (!current.array) {
      const type = typeSource(snapshot, current.type);
      fields = type?.data?.custom ? fieldsOf(type) : undefined;
    } else {
      fields = undefined;
    }
  }
  return current;
}

export function coerceScalarOrStructured(
  snapshot: Snapshot,
  field: Field | undefined,
  raw: Data,
): Data {
  if (typeof raw !== "string") {
    if (typeof raw === "number" && Number.isSafeInteger(raw) && field) {
      const prim = primitiveOf(snapshot, field);
      if (["int", "uint", "long", "ulong"].includes(prim)) return BigInt(raw);
    }
    if (Array.isArray(raw) && field && !field.array) {
      const kind = typeKind(typeSource(snapshot, field.type));
      if (kind === "flags") return flagsChange([], raw.map(String));
    }
    if (Array.isArray(raw) && field?.array) {
      return raw.map((item) =>
        coerceScalarOrStructured(
          snapshot,
          { ...field, array: false, nullable: false },
          item,
        ),
      );
    }
    if (isObject(raw) && field && !field.array) {
      const type = typeSource(snapshot, field.type);
      if (type?.data?.custom) {
        const subFields = fieldsOf(type);
        const next: Record<string, Data> = { ...raw };
        for (const sf of subFields) {
          if (sf.name in next)
            next[sf.name] = coerceScalarOrStructured(
              snapshot,
              sf,
              next[sf.name],
            );
        }
        return next;
      }
    }
    return raw;
  }

  const trimmed = raw.trim();
  if (field?.nullable && trimmed === "null") return null;

  if (field) {
    if (field.array) {
      if (trimmed.startsWith("[")) {
        const parsed = parseStructured(trimmed);
        if (!Array.isArray(parsed))
          throw new Error(`配列形式で指定してください: ${raw}`);
        return parsed.map((item) =>
          coerceScalarOrStructured(
            snapshot,
            { ...field, array: false, nullable: false },
            item,
          ),
        );
      }
      if (!trimmed) return [];
      return splitCsv(trimmed).map((item) =>
        coerceScalarOrStructured(
          snapshot,
          { ...field, array: false, nullable: false },
          item,
        ),
      );
    }
    const type = typeSource(snapshot, field.type);
    const kind = typeKind(type);
    const prim = primitiveOf(snapshot, field);
    if (prim === "string" || kind === "enum") {
      if (
        (trimmed.startsWith('"') && trimmed.endsWith('"')) ||
        (trimmed.startsWith("'") && trimmed.endsWith("'"))
      )
        return String(parseStructured(trimmed));
      return raw;
    }
    if (prim === "bool") {
      if (trimmed === "true") return true;
      if (trimmed === "false") return false;
      throw new Error(`真偽値（true / false）を指定してください: ${raw}`);
    }
    if (["int", "uint", "long", "ulong"].includes(prim)) {
      if (!/^-?\d+$/.test(trimmed))
        throw new Error(`整数を指定してください: ${raw}`);
      return BigInt(trimmed);
    }
    if (prim === "float" || prim === "double") {
      const num = Number(trimmed);
      if (!Number.isFinite(num))
        throw new Error(`数値を指定してください: ${raw}`);
      return num;
    }
    if (kind === "flags") {
      if (trimmed.startsWith("[")) {
        const parsed = parseStructured(trimmed);
        if (!Array.isArray(parsed))
          throw new Error(`Flagsは配列またはカンマ区切りで指定してください: ${raw}`);
        return flagsChange([], parsed.map(String));
      }
      return flagsChange([], splitCsv(trimmed));
    }
    if (kind === "custom") {
      const parsed = parseStructured(trimmed);
      if (!isObject(parsed))
        throw new Error(`構造体はJSON/YAMLオブジェクトで指定してください: ${raw}`);
      return coerceScalarOrStructured(snapshot, field, parsed);
    }
  }

  if (trimmed === "null") return null;
  if (trimmed === "true") return true;
  if (trimmed === "false") return false;
  if (/^-?\d+$/.test(trimmed)) return BigInt(trimmed);
  if (/^-?\d+\.\d+$/.test(trimmed)) return Number(trimmed);
  if (
    (trimmed.startsWith("{") && trimmed.endsWith("}")) ||
    (trimmed.startsWith("[") && trimmed.endsWith("]"))
  )
    return parseStructured(trimmed);
  return raw;
}

export function setByPath(
  root: Data,
  steps: PathStep[],
  value: Data,
): Data {
  if (!steps.length) return value;
  const [head, ...tail] = steps;
  if (head === "+") {
    const arr = Array.isArray(root) ? [...root] : [];
    arr.push(setByPath(undefined, tail, value));
    return arr;
  }
  if (typeof head === "number") {
    const arr = Array.isArray(root) ? [...root] : [];
    if (head < 0 || head > arr.length)
      throw new Error(`配列インデックス [${head}] が範囲外です（長さ ${arr.length}）。`);
    arr[head] = setByPath(arr[head], tail, value);
    return arr;
  }
  const obj = isObject(root) ? { ...root } : {};
  obj[head] = setByPath(obj[head], tail, value);
  return obj;
}

export function getByPath(root: Data, steps: PathStep[]): Data {
  let cur = root;
  for (const step of steps) {
    if (cur === null || cur === undefined) return undefined;
    if (step === "+") return undefined;
    cur = cur[step as any];
  }
  return cur;
}

export function parseAssignments(
  snapshot: Snapshot,
  tableFields: Field[],
  tokens: string[],
): { path: PathStep[]; value: Data }[] {
  return tokens.map((token) => {
    const eq = token.indexOf("=");
    if (eq <= 0)
      throw new Error(`"field=value" の形式で指定してください: ${token}`);
    const rawPath = token.slice(0, eq).trim();
    const rawValue = token.slice(eq + 1);
    const steps = parseFieldPath(rawPath);
    if (steps[0] === "$tags") {
      const tags = rawValue.trim().startsWith("[")
        ? (parseStructured(rawValue) as string[]).map(String)
        : splitCsv(rawValue);
      return { path: steps, value: tags };
    }
    const targetField = resolveFieldAtPath(snapshot, tableFields, steps);
    return {
      path: steps,
      value: coerceScalarOrStructured(snapshot, targetField, rawValue),
    };
  });
}

export function parsePkSelector(
  snapshot: Snapshot,
  tableFields: Field[],
  pkFields: string[],
  rawPk: Data,
): Record<string, string> {
  if (!pkFields.length)
    throw new Error("テーブルに主キーが設定されていません。");
  if (isObject(rawPk)) {
    return Object.fromEntries(
      pkFields.map((name) => {
        if (!(name in rawPk))
          throw new Error(`複合主キーの項目「${name}」を指定してください。`);
        return [name, text(rawPk[name])];
      }),
    );
  }
  const str = String(rawPk).trim();
  if (str.startsWith("{")) {
    const obj = parseStructured(str);
    return parsePkSelector(snapshot, tableFields, pkFields, obj);
  }
  if (pkFields.length === 1 && !str.includes("=")) {
    const field = tableFields.find((f) => f.name === pkFields[0]);
    const coerced = coerceScalarOrStructured(snapshot, field, str);
    return { [pkFields[0]]: text(coerced) };
  }
  const parts = splitCsv(str);
  const map: Record<string, string> = {};
  for (const part of parts) {
    const eq = part.indexOf("=");
    if (eq <= 0)
      throw new Error(
        `複合主キーは "key1=val1,key2=val2" またはJSONで指定してください: ${str}`,
      );
    const k = part.slice(0, eq).trim();
    const v = part.slice(eq + 1).trim();
    const field = tableFields.find((f) => f.name === k);
    map[k] = text(coerceScalarOrStructured(snapshot, field, v));
  }
  for (const name of pkFields) {
    if (!(name in map))
      throw new Error(`複合主キーの項目「${name}」が不足しています。`);
  }
  return map;
}

export function matchesPk(
  row: Data,
  pkFields: string[],
  expected: Record<string, string>,
): boolean {
  if (!isObject(row)) return false;
  return pkFields.every((name) => text(row[name]) === expected[name]);
}

function compareScalar(actual: Data, op: string, rhsRaw: string): boolean {
  if (op === "in") {
    const candidates = splitCsv(rhsRaw);
    if (Array.isArray(actual))
      return actual.some((item) => candidates.includes(text(item)));
    return candidates.includes(text(actual));
  }
  if (op === "~=") {
    const regex = new RegExp(rhsRaw);
    if (Array.isArray(actual))
      return actual.some((item) => regex.test(text(item)));
    return regex.test(text(actual));
  }
  if (Array.isArray(actual) && (op === "=" || op === "==")) {
    return actual.map(text).includes(rhsRaw) || JSON.stringify(actual) === rhsRaw;
  }
  if (Array.isArray(actual) && op === "!=") {
    return !actual.map(text).includes(rhsRaw);
  }
  if (actual === null || actual === undefined) {
    if (op === "=" || op === "==") return rhsRaw === "null";
    if (op === "!=") return rhsRaw !== "null";
    return false;
  }
  const leftNum =
    typeof actual === "bigint"
      ? actual
      : typeof actual === "number" && Number.isSafeInteger(actual)
        ? BigInt(actual)
        : null;
  if (leftNum !== null && /^-?\d+$/.test(rhsRaw.trim())) {
    const rightNum = BigInt(rhsRaw.trim());
    if (op === "=" || op === "==") return leftNum === rightNum;
    if (op === "!=") return leftNum !== rightNum;
    if (op === ">") return leftNum > rightNum;
    if (op === ">=") return leftNum >= rightNum;
    if (op === "<") return leftNum < rightNum;
    if (op === "<=") return leftNum <= rightNum;
  }
  if (typeof actual === "number" && /^-?\d+(\.\d+)?$/.test(rhsRaw.trim())) {
    const rightFloat = Number(rhsRaw.trim());
    if (op === "=" || op === "==") return actual === rightFloat;
    if (op === "!=") return actual !== rightFloat;
    if (op === ">") return actual > rightFloat;
    if (op === ">=") return actual >= rightFloat;
    if (op === "<") return actual < rightFloat;
    if (op === "<=") return actual <= rightFloat;
  }
  const leftStr = text(actual);
  const rightStr = rhsRaw.replace(/^['"]|['"]$/g, "");
  if (op === "=" || op === "==") return leftStr === rightStr;
  if (op === "!=") return leftStr !== rightStr;
  if (op === ">") return leftStr > rightStr;
  if (op === ">=") return leftStr >= rightStr;
  if (op === "<") return leftStr < rightStr;
  if (op === "<=") return leftStr <= rightStr;
  return false;
}

export function matchesWhere(row: Data, expressions: string[]): boolean {
  if (!isObject(row)) return false;
  for (const expr of expressions) {
    const match = expr.match(
      /^([A-Za-z0-9_$.[\]]+)\s*(!=|>=|<=|~=|==|=|>|<|\s+in\s+)\s*(.*)$/,
    );
    if (!match)
      throw new Error(
        `--where 条件の形式が不正です（例: "rarity=SSR", "price>=100"）: ${expr}`,
      );
    const [, rawPath, rawOp, rhs] = match;
    const op = rawOp.trim();
    const steps = parseFieldPath(rawPath.trim());
    const actual = getByPath(row, steps);
    if (!compareScalar(actual, op, rhs.trim())) return false;
  }
  return true;
}
