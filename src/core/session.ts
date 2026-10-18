import {
  type LoadedProject,
  type Snapshot,
  type Source,
  type SavedFile,
  type Field,
  type Data,
  type SaveResult,
  uid,
  setIn,
  fieldsOf,
  tables,
  typeKind,
  defaultConfig,
} from "./types";
import { serialize, parseSource } from "./yaml";
import { defaultValue, newRecord } from "./schema";
import { inheritIdentity } from "./identity";
import { mapRecords, mapTypedValue } from "./structure";
export interface SessionState {
  project: LoadedProject | null;
  snapshot: Snapshot;
  revision: number;
  past: { snapshot: Snapshot; label: string }[];
  future: { snapshot: Snapshot; label: string }[];
  saved: Map<string, SavedFile>;
  savedConfig: string;
}
const empty: Snapshot = { sources: [], folders: [], config: defaultConfig() };
export class Session {
  state: SessionState = {
    project: null,
    snapshot: empty,
    revision: 0,
    past: [],
    future: [],
    saved: new Map(),
    savedConfig: "",
  };
  private listeners = new Set<() => void>();
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  getState = () => this.state;
  private emit() {
    this.listeners.forEach((l) => l());
  }
  load(project: LoadedProject) {
    this.state = {
      project,
      snapshot: project.snapshot,
      revision: 0,
      past: [],
      future: [],
      saved: new Map(project.saved.map((f) => [f.id, f])),
      savedConfig: JSON.stringify(project.snapshot.config),
    };
    this.savedSources = new Map(project.snapshot.sources.map((s) => [s.id, s]));
    this.savedFolders = JSON.stringify(project.snapshot.folders);
    this.emit();
  }
  private savedSources = new Map<string, Source>();
  private savedFolders = "[]";
  private serialized = new WeakMap<
    Source,
    { base?: SavedFile; text: string }
  >();
  private fileText(source: Source) {
    const base = this.state.saved.get(source.id);
    let cached = this.serialized.get(source);
    if (!cached || cached.base !== base) {
      cached = { base, text: serialize(source, base) };
      this.serialized.set(source, cached);
    }
    return cached.text;
  }
  dirtySource(source: Source) {
    const base = this.savedSources.get(source.id);
    return (
      !base ||
      base.path !== source.path ||
      base.data !== source.data ||
      base.raw !== source.raw
    );
  }
  get dirty() {
    if (!this.state.project) return false;
    return (
      this.state.snapshot.sources.some((s) => this.dirtySource(s)) ||
      this.savedSources.size !== this.state.snapshot.sources.length ||
      JSON.stringify(this.state.snapshot.config) !== this.state.savedConfig ||
      JSON.stringify(this.state.snapshot.folders) !== this.savedFolders
    );
  }
  change(label: string, modify: (s: Snapshot) => Snapshot) {
    const snapshot = modify(this.state.snapshot);
    if (snapshot === this.state.snapshot) return;
    const previousSources = new Map(
      this.state.snapshot.sources.map((s) => [s.id, s]),
    );
    for (const source of snapshot.sources) {
      const previous = previousSources.get(source.id);
      if (
        previous &&
        source !== previous &&
        Array.isArray(source.data?.records) &&
        Array.isArray(previous.data?.records) &&
        source.data.records.length === previous.data.records.length
      )
        source.data.records.forEach((row: Data, i: number) =>
          inheritIdentity(previous.data.records[i], row),
        );
    }
    this.state = {
      ...this.state,
      snapshot,
      revision: this.state.revision + 1,
      past: [
        ...this.state.past.slice(-99),
        { snapshot: this.state.snapshot, label },
      ],
      future: [],
    };
    this.emit();
  }
  undo() {
    const entry = this.state.past.at(-1);
    if (!entry) return;
    this.state = {
      ...this.state,
      snapshot: entry.snapshot,
      revision: this.state.revision + 1,
      past: this.state.past.slice(0, -1),
      future: [
        ...this.state.future,
        { snapshot: this.state.snapshot, label: entry.label },
      ],
    };
    this.emit();
  }
  redo() {
    const entry = this.state.future.at(-1);
    if (!entry) return;
    this.state = {
      ...this.state,
      snapshot: entry.snapshot,
      revision: this.state.revision + 1,
      future: this.state.future.slice(0, -1),
      past: [
        ...this.state.past,
        { snapshot: this.state.snapshot, label: entry.label },
      ],
    };
    this.emit();
  }
  update(
    id: string,
    path: (string | number)[],
    value: Data,
    label = "値を編集",
  ) {
    this.change(label, (s) => ({
      ...s,
      sources: s.sources.map((f) =>
        f.id === id
          ? {
              ...f,
              data: setIn(f.data, path, value),
              raw: undefined,
              syntaxError: undefined,
            }
          : f,
      ),
    }));
  }
  raw(id: string, raw: string) {
    this.change("ソースを編集", (s) => ({
      ...s,
      sources: s.sources.map((f) =>
        f.id === id ? parseSource(f.path, raw, id) : f,
      ),
    }));
  }
  add(
    kind: "table" | "enum" | "flags" | "custom" | "valueObject" | "data",
    folder = "",
    table = "",
  ): string {
    const id = uid();
    let index = 1;
    const stem =
      kind === "table" ? "new-table" : kind === "data" ? "new-data" : "NewType";
    const names = new Set(
      this.state.snapshot.sources.flatMap((f) => [
        f.data?.table,
        f.data?.name,
        f.path
          .split("/")
          .at(-1)
          ?.replace(/\.ya?ml$/, ""),
      ]),
    );
    while (names.has(`${stem}${index}`)) index++;
    const name = `${stem}${index}`;
    const data: any =
      kind === "table"
        ? {
            kind: "schema",
            table: name,
            fields: [{ key: 0, name: "id", type: "int" }],
            primaryKey: { fields: ["id"] },
            records: [],
          }
        : kind === "data"
          ? { kind: "data", table, records: [] }
          : {
              kind: "type",
              name,
              [kind]:
                kind === "custom"
                  ? { fields: [] }
                  : kind === "valueObject"
                    ? { underlying: "int" }
                    : {
                        underlying: kind === "flags" ? "uint" : "int",
                        members: [
                          {
                            name: kind === "flags" ? "None" : "NewMember",
                            value: 0,
                          },
                        ],
                      },
            };
    const path = [folder, `${name}.yaml`].filter(Boolean).join("/");
    this.change("新規作成", (s) => ({
      ...s,
      sources: [...s.sources, { id, path, data }],
    }));
    return id;
  }
  addFolder(parent = ""): string {
    let n = 1;
    const paths = new Set([
      ...this.state.snapshot.folders,
      ...this.state.snapshot.sources.map((s) => s.path),
    ]);
    let path = "";
    do {
      path = [parent, `新しいフォルダ${n++}`].filter(Boolean).join("/");
    } while (paths.has(path));
    this.change("フォルダを作成", (s) => ({
      ...s,
      folders: [...s.folders, path],
    }));
    return path;
  }
  move(oldPath: string, newPath: string) {
    if (!newPath || newPath === oldPath || newPath.startsWith(`${oldPath}/`))
      return;
    if (
      newPath
        .split("/")
        .some((p) => !p || p === "." || p === ".." || /[\\\x00-\x1f]/.test(p))
    )
      throw new Error("名前に使用できない文字があります。");
    const s = this.state.snapshot;
    if (
      s.sources.some((f) => f.path === newPath) ||
      s.folders.includes(newPath)
    )
      throw new Error("同名の項目が存在します。");
    const rename = (p: string) =>
      p === oldPath
        ? newPath
        : p.startsWith(`${oldPath}/`)
          ? newPath + p.slice(oldPath.length)
          : p;
    this.change("移動・名前変更", (s) => ({
      ...s,
      sources: s.sources.map((f) =>
        rename(f.path) === f.path ? f : { ...f, path: rename(f.path) },
      ),
      folders: s.folders.map(rename),
    }));
  }
  remove(paths: string[]) {
    this.change("削除", (s) => ({
      ...s,
      sources: s.sources.filter(
        (f) => !paths.some((p) => f.path === p || f.path.startsWith(`${p}/`)),
      ),
      folders: s.folders.filter(
        (f) => !paths.some((p) => f === p || f.startsWith(`${p}/`)),
      ),
    }));
  }
  addField(id: string, copyIndex?: number): string {
    const source = this.state.snapshot.sources.find((f) => f.id === id)!;
    const fields = fieldsOf(source);
    const template = copyIndex === undefined ? undefined : fields[copyIndex];
    const prefix = template ? `${template.name}Copy` : "field";
    let n = 1;
    while (fields.some((f) => f.name === `${prefix}${n}`)) n++;
    const name = `${prefix}${n}`;
    const field: Field = {
      ...template,
      key:
        Math.max(
          -1,
          ...fields.map((f) => Number(f.key ?? -1)).filter(Number.isFinite),
        ) + 1,
      name,
      type: template?.type ?? "string",
    };
    const next = [...fields];
    next.splice(template ? copyIndex! + 1 : fields.length, 0, field);
    this.change(template ? "項目を複製" : "項目を追加", (snapshot) => {
      const sources = mapRecords(
        snapshot,
        source.data.custom ? null : source.data.table,
        source.data.custom ? source.data.name : null,
        (record) =>
          name in record
            ? record
            : {
                ...record,
                [name]:
                  template && Object.hasOwn(record, template.name)
                    ? structuredClone(record[template.name])
                    : defaultValue(snapshot, field),
              },
      );
      return {
        ...snapshot,
        sources: sources.map((s) =>
          s.id === id
            ? {
                ...s,
                data: setIn(
                  s.data,
                  source.data.custom ? ["custom", "fields"] : ["fields"],
                  next,
                ),
              }
            : s,
        ),
      };
    });
    return name;
  }
  removeField(id: string, index: number) {
    const source = this.state.snapshot.sources.find((s) => s.id === id)!;
    const fields = fieldsOf(source);
    const name = fields[index]?.name;
    if (!name) return;
    this.change("項目を削除", (snapshot) => {
      const sources = mapRecords(
        snapshot,
        source.data.custom ? null : source.data.table,
        source.data.custom ? source.data.name : null,
        (record) => {
          const next = { ...record };
          delete next[name];
          return next;
        },
      );
      return {
        ...snapshot,
        sources: sources.map((s) =>
          s.id === id
            ? {
                ...s,
                data: setIn(
                  s.data,
                  source.data.custom ? ["custom", "fields"] : ["fields"],
                  fields.filter((_, i) => i !== index),
                ),
              }
            : s,
        ),
      };
    });
  }
  renameMember(id: string, index: number, name: string) {
    const source = this.state.snapshot.sources.find((s) => s.id === id)!;
    const kind = typeKind(source)!;
    const old = source.data[kind].members[index].name;
    this.change("候補名を変更", (snapshot) => {
      const sources = snapshot.sources.map((s) => {
        if (!Array.isArray(s.data?.records)) return s;
        const schema = snapshot.sources.find(
          (f) => f.data?.kind === "schema" && f.data?.table === s.data.table,
        );
        if (!schema) return s;
        const records = s.data.records.map((record: Data) => {
          if (!record || typeof record !== "object") return record;
          let next = record;
          for (const field of fieldsOf(schema)) {
            const value = mapTypedValue(
              snapshot,
              field,
              record[field.name],
              source.data.name,
              (value) =>
                kind === "flags" && Array.isArray(value)
                  ? value.map((v) => (v === old ? name : v))
                  : value === old
                    ? name
                    : value,
            );
            if (value !== record[field.name])
              next = { ...next, [field.name]: value };
          }
          return next;
        });
        return records.some((r: Data, i: number) => r !== s.data.records[i])
          ? { ...s, data: { ...s.data, records } }
          : s;
      });
      return {
        ...snapshot,
        sources: sources.map((s) =>
          s.id === id
            ? {
                ...s,
                data: setIn(s.data, [kind, "members", index, "name"], name),
              }
            : s,
        ),
      };
    });
  }
  renameField(id: string, index: number, name: string) {
    const source = this.state.snapshot.sources.find((f) => f.id === id)!;
    const fields = fieldsOf(source);
    const old = fields[index]?.name;
    if (old === name) return;
    if (!old) {
      this.update(
        id,
        source.data?.custom
          ? ["custom", "fields", index, "name"]
          : ["fields", index, "name"],
        name,
        "項目名を修復",
      );
      return;
    }
    if (fields.some((f, i) => i !== index && f.name === name))
      throw new Error("同名の項目があります。");
    const isCustom = !!source.data?.custom;
    function rewrite(
      value: any,
      type: string,
      field: Field,
      seen: Set<string>,
    ): any {
      if (value === null || value === undefined) return value;
      if (field.array && Array.isArray(value))
        return value.map((v) =>
          rewrite(v, type, { ...field, array: false }, seen),
        );
      if (!value || typeof value !== "object" || Array.isArray(value))
        return value;
      const def = thisSnapshot.sources.find(
        (s) => s.data?.name === type && s.data?.custom,
      );
      if (!def || seen.has(type)) return value;
      const next = { ...value };
      const seenNext = new Set(seen).add(type);
      if (type === source.data.name && old in next) {
        next[name] = next[old];
        delete next[old];
      }
      for (const f of fieldsOf(def)) {
        const key = type === source.data.name && f.name === old ? name : f.name;
        next[key] = rewrite(next[key], f.type, f, seenNext);
      }
      return next;
    }
    const thisSnapshot = this.state.snapshot;
    this.change("項目名を変更", (s) => ({
      ...s,
      sources: s.sources.map((f) => {
        let data = f.data;
        if (f.id === id)
          data = setIn(
            data,
            isCustom
              ? ["custom", "fields", index, "name"]
              : ["fields", index, "name"],
            name,
          );
        if (!isCustom && f.data?.table === source.data.table) {
          if (Array.isArray(data.records))
            data = {
              ...data,
              records: data.records.map((r: any) => {
                if (!r || typeof r !== "object" || !(old in r)) return r;
                const n = { ...r, [name]: r[old] };
                delete n[old];
                return n;
              }),
            };
          for (const key of ["primaryKey", "secondaryKeys", "references"])
            if (data[key]) {
              const updateKey = (k: any) => ({
                ...k,
                fields: Array.isArray(k.fields)
                  ? k.fields.map((n: string) => (n === old ? name : n))
                  : k.fields,
              });
              data = {
                ...data,
                [key]: Array.isArray(data[key])
                  ? data[key].map(updateKey)
                  : updateKey(data[key]),
              };
            }
        }
        if (
          !isCustom &&
          Array.isArray(data?.references) &&
          data.references.some(
            (r: any) =>
              r?.target?.table === source.data.table &&
              Array.isArray(r.target.fields) &&
              r.target.fields.includes(old),
          )
        )
          data = {
            ...data,
            references: data.references.map((r: any) =>
              r?.target?.table === source.data.table
                ? {
                    ...r,
                    target: {
                      ...r.target,
                      fields: Array.isArray(r.target.fields)
                        ? r.target.fields.map((n: string) =>
                            n === old ? name : n,
                          )
                        : r.target.fields,
                    },
                  }
                : r,
            ),
          };
        if (isCustom && Array.isArray(data?.records)) {
          const table = tables(s).find((t) => t.name === data.table);
          if (table)
            data = {
              ...data,
              records: data.records.map((r: any) =>
                !r || typeof r !== "object" || Array.isArray(r)
                  ? r
                  : Object.fromEntries(
                      Object.entries(r).map(([k, v]) => {
                        const field = table.fields.find((f) => f.name === k);
                        return [
                          k,
                          field ? rewrite(v, field.type, field, new Set()) : v,
                        ];
                      }),
                    ),
              ),
            };
        }
        return data === f.data ? f : { ...f, data };
      }),
    }));
  }
  renameDeclaration(id: string, name: string) {
    const source = this.state.snapshot.sources.find((f) => f.id === id)!;
    const key = source.data?.kind === "type" ? "name" : "table";
    const old = source.data?.[key];
    if (!source.data || source.syntaxError)
      throw new Error("ソースの形式を先に修正してください。");
    this.change("定義名を変更", (s) => ({
      ...s,
      sources: s.sources.map((f) => {
        let data = f.data;
        if (
          key === "table" &&
          (f.id === id || (typeof old === "string" && data?.table === old))
        )
          data = { ...data, table: name };
        if (key === "name") {
          if (f.id === id) data = { ...data, name };
          const fields = fieldsOf(f);
          if (fields.some((f) => f.type === old))
            data = setIn(
              data,
              data.custom ? ["custom", "fields"] : ["fields"],
              fields.map((f) => (f.type === old ? { ...f, type: name } : f)),
            );
        }
        if (
          key === "table" &&
          typeof old === "string" &&
          Array.isArray(data?.references) &&
          data.references.some((r: any) => r?.target?.table === old)
        )
          data = {
            ...data,
            references: data.references.map((r: any) =>
              typeof old === "string" && r?.target?.table === old
                ? { ...r, target: { ...r.target, table: name } }
                : r,
            ),
          };
        return data === f.data ? f : { ...f, data };
      }),
    }));
  }
  addRow(schemaId: string, destinationId: string, template?: Data) {
    const table = tables(this.state.snapshot).find(
      (t) => t.source.id === schemaId,
    )!;
    const target = this.state.snapshot.sources.find(
      (s) => s.id === destinationId,
    )!;
    this.update(
      destinationId,
      ["records"],
      [
        ...(Array.isArray(target.data.records) ? target.data.records : []),
        newRecord(this.state.snapshot, table, template),
      ],
      "レコードを追加",
    );
  }
  files(ids?: string[]) {
    const changed = this.state.snapshot.sources
      .filter((s) => !ids || ids.includes(s.id))
      .map((s) => ({
        id: s.id,
        path: s.path,
        text: this.dirtySource(s)
          ? this.fileText(s)
          : this.state.saved.get(s.id)!.text,
      }));
    if (!ids) return changed;
    const result = new Map(this.state.saved);
    ids.forEach((id) => result.delete(id));
    changed.forEach((file) => result.set(file.id, file));
    return [...result.values()];
  }
  saveFolders(captured: Snapshot, ids?: string[]) {
    if (!ids) return captured.folders;
    const folders = new Set<string>(JSON.parse(this.savedFolders));
    for (const source of captured.sources.filter((s) => ids.includes(s.id))) {
      const parts = source.path.split("/").slice(0, -1);
      parts.forEach((_, i) => {
        const parent = parts.slice(0, i + 1).join("/");
        // File parents are created by the writer. Only acknowledge explicit
        // folder drafts here, so an implicit parent cannot leave a phantom edit.
        if (captured.folders.includes(parent)) folders.add(parent);
      });
    }
    return [...folders];
  }
  saveConfig(captured: Snapshot, ids?: string[]) {
    return ids ? JSON.parse(this.state.savedConfig) : captured.config;
  }
  acceptSave(result: SaveResult, captured: Snapshot, ids?: string[]) {
    // Saved bases are independent from history; a save must not invalidate Undo or overwrite edits made during I/O.
    const sources = ids
      ? new Map(this.savedSources)
      : new Map<string, Source>();
    ids?.forEach((id) => sources.delete(id));
    captured.sources
      .filter((s) => !ids || ids.includes(s.id))
      .forEach((s) => sources.set(s.id, s));
    this.savedSources = sources;
    this.savedFolders = JSON.stringify(this.saveFolders(captured, ids));
    this.state = {
      ...this.state,
      saved: new Map(result.files.map((f) => [f.id, f])),
      savedConfig: JSON.stringify(this.saveConfig(captured, ids)),
    };
    this.emit();
  }
}
export const session = new Session();
