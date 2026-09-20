import { callRust } from "./web-wasm";

type Config = { project: { id: string; name: string; version: string }; sources: { roots: string[] }; build: { profiles: Record<string, { include_tags: string[]; exclude_tags: string[] }> } };
type Entry = { path: string; root: string; handle: FileSystemFileHandle };
type Workspace = { directory: FileSystemDirectoryHandle; config: Config; entries: Map<string, Entry>; folders: Array<{ path: string; sourceRoot: string }> };
let workspace: Workspace | null = null;

function failure(code: string, message: string) {
  return { diagnostic: { code, kind: "validation", message, source: null, line: null, column: null, schemaPath: null, valuePath: null, recordIdentity: null, suggestion: null, relatedRequirements: [] } };
}

function parts(path: string): string[] {
  const normalized = path.replace(/\/$/, "");
  const result = normalized.split("/");
  if (!normalized || normalized.startsWith("/") || normalized.includes("\\") || result.some((part) => !part || part === "." || part === ".." || part.includes(":"))) {
    throw failure("E-WEB-SOURCE-PATH", `Unsafe project-relative path: ${path}`);
  }
  return result;
}

async function discover(directory: FileSystemDirectoryHandle, config: Config) {
  const entries = new Map<string, Entry>();
  const folders: Workspace["folders"] = [];
  for (const root of config.sources.roots) {
    const logicalRoot = root === "." ? "." : parts(root).join("/");
    let current = directory;
    for (const segment of logicalRoot === "." ? [] : logicalRoot.split("/")) current = await current.getDirectoryHandle(segment);
    const visit = async (handle: FileSystemDirectoryHandle, path: string): Promise<void> => {
      for await (const child of (handle as FileSystemDirectoryHandle & { values(): AsyncIterableIterator<FileSystemHandle> }).values()) {
        const childPath = path === "." ? child.name : `${path}/${child.name}`;
        if (child.kind === "directory") {
          folders.push({ path: childPath, sourceRoot: logicalRoot });
          await visit(child as FileSystemDirectoryHandle, childPath);
        } else if (/\.ya?ml$/i.test(child.name)) {
          entries.set(childPath, { path: childPath, root: logicalRoot, handle: child as FileSystemFileHandle });
        }
      }
    };
    await visit(current, logicalRoot);
  }
  return { entries, folders };
}

async function files(current: Workspace, override?: { path: string; source: string }) {
  const all = [];
  for (const entry of [...current.entries.values()].sort((a, b) => a.path.localeCompare(b.path))) {
    all.push({ path: entry.path, source: entry.path === override?.path ? override.source : await (await entry.handle.getFile()).text() });
  }
  return all;
}

function selected(): Workspace {
  if (!workspace) throw failure("E-WEB-WORKSPACE-NOT-OPEN", "Choose a local Masterdata workspace first.");
  return workspace;
}

function entryAt(args: Record<string, unknown>): Entry {
  const path = String(args.relativePath ?? "");
  parts(path);
  const entry = selected().entries.get(path);
  if (!entry) throw failure("E-WEB-SOURCE-NOT-FOUND", `Source is outside the selected workspace: ${path}`);
  return entry;
}

function profiles(config: Config) {
  return Object.entries(config.build.profiles ?? {}).map(([name, profile]) => ({ name, include_tags: profile.include_tags, exclude_tags: profile.exclude_tags }));
}

async function openWorkspace(pick: boolean) {
  if (pick) {
    const choose = (window as Window & { showDirectoryPicker?: (options?: { mode: "read" }) => Promise<FileSystemDirectoryHandle> }).showDirectoryPicker;
    if (!choose) throw failure("E-WEB-FILESYSTEM-UNAVAILABLE", "This browser does not provide the local directory picker.");
    const directory = await choose.call(window, { mode: "read" });
    const configFile = await directory.getFileHandle("masterdata.toml");
    const { config } = await callRust<{ config: Config }>({ op: "parse_config", source: await (await configFile.getFile()).text() });
    workspace = { directory, config, ...await discover(directory, config) };
  }
  const current = selected();
  if (!pick) Object.assign(current, await discover(current.directory, current.config));
  const { files: analyzed } = await callRust<{ files: Array<{ path: string; kind: string; table: string | null; typeName: string | null; diagnostic: unknown }> }>({ op: "analyze", files: await files(current) });
  return {
    project: { project_root: "browser-workspace", config_path: "masterdata.toml", project_id: current.config.project.id, name: current.config.project.name, version: current.config.project.version, source_roots: current.config.sources.roots, artifact_root: "", csharp_output: "", binary_output: "", cache: "", profiles: profiles(current.config), publish_targets: [] },
    sourceRoots: current.config.sources.roots,
    files: analyzed.map((file) => ({ ...file, sourceRoot: current.entries.get(file.path)?.root ?? "" })),
    folders: current.folders,
    capabilities: { workspaceRead: true, workspaceWrite: false, validate: true, build: false },
  };
}

async function run(command: string, args: Record<string, unknown>) {
  if (command === "authoring_workspace") return openWorkspace(args.projectPath === "browser-picker");
  if (command === "migration_recovery_status" || command === "recheck_migration") return null;
  if (command === "validate") {
    const { validation } = await callRust<{ validation: unknown }>({ op: "analyze", files: await files(selected()) });
    return validation;
  }
  if (command === "open_data_file") {
    const entry = entryAt(args);
    const { snapshot } = await callRust<{ snapshot: unknown }>({ op: "open_data", files: await files(selected()), target: entry.path, profiles: profiles(selected().config) });
    return snapshot;
  }
  if (command === "open_table" || command === "open_type") {
    const entry = entryAt(args);
    const { snapshot } = await callRust<{ snapshot: unknown }>({ op: command, files: await files(selected()), target: entry.path });
    return snapshot;
  }
  if (command === "preview_data_file") {
    const entry = entryAt(args);
    const { preview } = await callRust<{ preview: unknown }>({ op: "preview_data", files: await files(selected(), { path: entry.path, source: String(args.baseSource) }), target: entry.path, mutation: { edits: args.edits ?? [], addedRecords: args.addedRecords ?? [], deletedRecordIndices: args.deletedRecordIndices ?? [], tagEdits: args.tagEdits ?? [] } });
    return preview;
  }
  if (command === "query_data_file") {
    const request = args.request as Record<string, unknown>;
    const entry = entryAt(request);
    const { result } = await callRust<{ result: unknown }>({
      op: "query_data",
      files: await files(selected(), { path: entry.path, source: String(request.baseSource) }),
      target: entry.path,
      mutation: request.mutation ?? {},
      query: request.query ?? {},
      profiles: profiles(selected().config),
    });
    return result;
  }
  if (command === "source_content") {
    const entry = entryAt(args);
    const source = await (await entry.handle.getFile()).text();
    const { identity } = await callRust<{ identity: string }>({ op: "identity", source });
    return { path: entry.path, source, contentIdentity: identity };
  }
  throw failure("E-WEB-CAPABILITY-UNAVAILABLE", `Standalone Web does not provide ${command}.`);
}

export async function invokeBrowser<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  try { return await run(command, args) as T; }
  catch (error) {
    if (typeof error === "object" && error !== null && "diagnostic" in error) {
      const diagnostic = error.diagnostic as Record<string, unknown>;
      throw { diagnostic: { ...diagnostic, schemaPath: diagnostic.schemaPath ?? diagnostic.schema_path ?? null, valuePath: diagnostic.valuePath ?? diagnostic.value_path ?? null, recordIdentity: diagnostic.recordIdentity ?? diagnostic.record_identity ?? null, relatedRequirements: diagnostic.relatedRequirements ?? diagnostic.related_requirements ?? [] } };
    }
    const message = typeof error === "object" && error !== null && "message" in error
      ? String(error.message) : String(error);
    throw failure("E-WEB-OPERATION", message);
  }
}
