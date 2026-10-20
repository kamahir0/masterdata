import { inheritIdentity } from "./identity";
export type Data = any;
export interface Field {
  name: string;
  type: string;
  key?: number | bigint;
  array?: boolean;
  nullable?: boolean;
}
export interface Source {
  id: string;
  path: string;
  data: Data;
  raw?: string;
  formatText?: string;
  syntaxError?: string;
}
export interface Profile {
  include: string[];
  exclude: string[];
}
export interface Config {
  name: string;
  profiles: Record<string, Profile>;
  publish: { csharp: string; binary: string };
}
export interface Snapshot {
  sources: Source[];
  folders: string[];
  config: Config;
}
export interface SavedFile {
  id: string;
  path: string;
  text: string;
}
export interface LoadedProject {
  root: string;
  token: string;
  snapshot: Snapshot;
  saved: SavedFile[];
  configText: string;
}
export interface SaveRequest {
  token: string;
  files: SavedFile[];
  folders: string[];
  config: Config;
}
export interface SaveResult {
  files: SavedFile[];
  configText: string;
}
export interface Problem {
  source: string;
  path: (string | number)[];
  message: string;
  code: string;
}
export interface TableInfo {
  source: Source;
  name: string;
  fields: Field[];
  sources: Source[];
}
export interface BuildResult {
  directory: string;
  revision: number;
  profile: string;
  tables: number;
  records: number;
  log: string;
  files: string[];
  hashes: Record<string, string>;
}
export interface DesktopAPI {
  initial(): Promise<LoadedProject | null>;
  close(): Promise<void>;
  open(): Promise<LoadedProject | null>;
  create(name: string): Promise<LoadedProject | null>;
  sample(): Promise<LoadedProject>;
  save(request: SaveRequest): Promise<SaveResult>;
  build(
    token: string,
    snapshot: Snapshot,
    profile: string,
    revision: number,
  ): Promise<BuildResult>;
  publish(token: string, targets: Config["publish"]): Promise<string>;
  reveal(token: string, output?: boolean): Promise<void>;
  dirty(dirty: boolean): void;
  onCommand(callback: (command: string) => void): () => void;
}
declare global {
  interface Window {
    desktop: DesktopAPI;
  }
}
export const primitives = [
  "bool",
  "int",
  "uint",
  "long",
  "ulong",
  "float",
  "double",
  "string",
];
export const defaultConfig = (name = "新しいプロジェクト"): Config => ({
  name,
  profiles: { production: { include: [], exclude: ["debug"] } },
  publish: { csharp: "", binary: "" },
});
export const uid = (): string => crypto.randomUUID();
export const isObject = (v: Data): v is Record<string, Data> =>
  !!v && typeof v === "object" && !Array.isArray(v);
export const text = (v: Data): string =>
  v === null
    ? "null"
    : v === undefined
      ? ""
      : typeof v === "object"
        ? Array.isArray(v)
          ? `[${v.length}]`
          : "{ … }"
        : String(v);
export const list = <T = any>(value: any): T[] =>
  Array.isArray(value) ? value : [];
export const fieldsOf = (s?: Source): Field[] =>
  list(s?.data?.custom ? s.data.custom.fields : s?.data?.fields).map((f) =>
    isObject(f) && typeof f.name === "string" && typeof f.type === "string"
      ? (f as Field)
      : ({
          ...(isObject(f) ? f : {}),
          name: typeof f?.name === "string" ? f.name : "",
          type: typeof f?.type === "string" ? f.type : "",
        } as Field),
  );
export const typeKind = (
  s?: Source,
): "enum" | "flags" | "custom" | "valueObject" | undefined =>
  s &&
  (["enum", "flags", "custom", "valueObject"].find((k) => s.data?.[k]) as any);
interface ProjectIndex {
  tables: TableInfo[];
  types: Source[];
  typeByName: Map<string, Source>;
}
const indexes = new WeakMap<Snapshot, ProjectIndex>();
function index(snapshot: Snapshot): ProjectIndex {
  const cached = indexes.get(snapshot);
  if (cached) return cached;
  const types = snapshot.sources.filter((s) => s.data?.kind === "type");
  const dataByTable = new Map<string, Source[]>();
  for (const source of snapshot.sources)
    if (source.data?.kind === "schema" || source.data?.kind === "data") {
      const name = String(source.data.table);
      const group = dataByTable.get(name) ?? [];
      group.push(source);
      dataByTable.set(name, group);
    }
  const value = {
    types,
    typeByName: new Map(types.map((s) => [String(s.data.name), s])),
    tables: snapshot.sources
      .filter((s) => s.data?.kind === "schema")
      .map((source) => ({
        source,
        name: String(source.data.table ?? ""),
        fields: fieldsOf(source),
        sources: dataByTable.get(String(source.data.table)) ?? [],
      })),
  };
  indexes.set(snapshot, value);
  return value;
}
export const tables = (snapshot: Snapshot): TableInfo[] =>
  index(snapshot).tables;
export const typeSources = (snapshot: Snapshot): Source[] =>
  index(snapshot).types;
export const typeSource = (
  snapshot: Snapshot,
  name: string,
): Source | undefined => index(snapshot).typeByName.get(name);
export function getIn(value: Data, path: (string | number)[]): Data {
  for (const key of path) value = value?.[key];
  return value;
}
export function setIn(
  value: Data,
  path: (string | number)[],
  next: Data,
): Data {
  if (!path.length) {
    inheritIdentity(value, next);
    return next;
  }
  const [key, ...rest] = path;
  const copy = Array.isArray(value)
    ? [...value]
    : { ...(isObject(value) ? value : {}) };
  inheritIdentity(value, copy);
  (copy as any)[key] = setIn(value?.[key], rest, next);
  return copy;
}
export function selected(record: Data, profile?: Profile): boolean {
  if (!profile) return true;
  const tags = Array.isArray(record?.$tags) ? record.$tags : [];
  return (
    (!profile.include.length ||
      profile.include.some((t) => tags.includes(t))) &&
    !profile.exclude.some((t) => tags.includes(t))
  );
}

export const searchText = (value: Data): string =>
  value && typeof value === "object"
    ? Object.values(value).map(searchText).join(" ")
    : text(value);
