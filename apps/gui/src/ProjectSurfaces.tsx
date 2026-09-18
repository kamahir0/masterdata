import { useCallback, useEffect, useRef, useState } from "react";
import { Alert, Button, Empty, Input, Select, Space, Tag } from "antd";
import { invoke } from "@tauri-apps/api/core";
import { type AuthoringValue, type ResolvedAuthoringField } from "./data-editor-types";

export type SurfaceProjectInfo = {
  project_root: string;
  config_path: string;
  name: string;
  project_id: string;
  version?: string;
  profiles?: BuildProfileInfo[];
  publish_targets?: PublishTargetInfo[];
};

export type SurfaceWorkspace = {
  project: SurfaceProjectInfo;
  files: Array<{ path: string; kind: string; table: string | null }>;
};

export type BuildProfileInfo = {
  name: string;
  include_tags: string[];
  exclude_tags: string[];
};

export type PublishTargetInfo = {
  kind: "csharp" | "binary";
  path: string;
  resolved_path: string;
};

type Diagnostic = {
  code: string;
  message: string;
  kind?: string;
};

type OverviewColumn = {
  name: string;
  typeName: string;
  keyField: boolean;
  shape: ResolvedAuthoringField | null;
};

type OverviewRow = {
  table: string;
  sourcePath: string;
  recordIndex: number;
  values: unknown[];
  selected: boolean | null;
  matchedIncludeTags: string[];
  matchedExcludeTags: string[];
  selectionReason: string | null;
};

type OverviewSnapshot = {
  status: "complete" | "partial" | "unavailable" | "stale";
  table: string;
  columns: OverviewColumn[];
  rows: OverviewRow[];
  totalCount: number;
  selectedCount: number | null;
  displayedCount: number;
  configContentIdentity: string;
  sources: Array<{ path: string; contentIdentity: string }>;
  selection: {
    profile: string | null;
    includeTags: string[];
    excludeTags: string[];
    available: boolean;
  };
  diagnostics: Diagnostic[];
};

type ConfigSnapshot = {
  projectRoot: string;
  configPath: string;
  baseSource: string;
  baseContentIdentity: string;
  configValid: boolean;
  profiles: BuildProfileInfo[];
  publishTargets: PublishTargetInfo[];
  diagnostics: Diagnostic[];
};

type ConfigEditPreview = {
  baseContentIdentity: string;
  candidateContentIdentity: string;
  candidateSource: string;
  changed: boolean;
  configValid: boolean;
  diagnostics: Diagnostic[];
};

type ConfigEditRequest =
  | { operation: "add_profile"; name: string; include_tags: string[]; exclude_tags: string[] }
  | { operation: "update_profile"; name: string; include_tags: string[]; exclude_tags: string[] }
  | { operation: "add_publish_target"; kind: "csharp" | "binary"; path: string }
  | { operation: "update_publish_target_path"; index: number; path: string };

type ConfigSaveReport = {
  status: "success" | "conflict" | "failure" | "outcome_unknown";
  snapshot: ConfigSnapshot | null;
  current: ConfigSnapshot | null;
  diagnostic: Diagnostic | null;
};

type PublishPreview = {
  artifactSetIdentity: string;
  configContentIdentity: string;
  targets: Array<{
    index: number;
    kind: "csharp" | "binary";
    configuredPath: string;
    destination: string;
    additions: string[];
    updates: string[];
    removals: string[];
    binaryReplacement: boolean;
    preflightOk: boolean;
  }>;
};

type BuildResponse = {
  profile: string | null;
  generatedFiles: string[];
  artifactRoot: string;
};

type ProjectInitReport = {
  status: "success" | "conflict" | "failure" | "outcome_unknown";
  destination: string;
  project: SurfaceProjectInfo | null;
  createdEntries: string[];
  diagnostic: Diagnostic | null;
};

function errorDiagnostic(error: unknown): Diagnostic {
  if (typeof error === "object" && error !== null && "diagnostic" in error) {
    const diagnostic = (error as { diagnostic?: Diagnostic }).diagnostic;
    if (diagnostic) return diagnostic;
  }
  return { code: "E-GUI-SURFACE", kind: "external_tool", message: String(error) };
}

function splitTags(value: string): string[] {
  return value.split(",").map((tag) => tag.trim()).filter((tag) => tag.length > 0);
}

function queryInputValue(column: OverviewColumn | undefined, text: string): AuthoringValue {
  const shape = column?.shape?.shape;
  if (!shape) return { kind: "string", value: text };
  if (shape.kind === "primitive") {
    if (shape.primitive === "bool" && (text === "true" || text === "false")) {
      return { kind: "bool", value: text === "true" };
    }
    return shape.primitive === "string"
      ? { kind: "string", value: text }
      : { kind: "number", value: text };
  }
  if (shape.kind === "value_object" && shape.underlying !== "string") {
    return shape.underlying === "bool" && (text === "true" || text === "false")
      ? { kind: "bool", value: text === "true" }
      : { kind: "number", value: text };
  }
  return { kind: "string", value: text };
}

function valueLabel(value: unknown): string {
  if (value == null) return "null";
  if (typeof value === "string") return value;
  if (typeof value === "object" && value !== null && "value" in value) {
    const inner = (value as { value?: unknown }).value;
    return typeof inner === "string" ? inner : JSON.stringify(inner);
  }
  return JSON.stringify(value);
}

export function ProjectOverviewPanel({
  active,
  projectRoot,
  workspace,
  table,
  dirtySourceCount,
  dirtyConfig,
  profile,
  onProfileChange,
  onNavigate,
}: {
  active: boolean;
  projectRoot: string | null;
  workspace: SurfaceWorkspace | null;
  table: string | null;
  dirtySourceCount: number;
  dirtyConfig: boolean;
  profile: string;
  onProfileChange: (profile: string) => void;
  onNavigate: (path: string, recordIndex: number, expectedIdentity: string) => void;
}) {
  const [search, setSearch] = useState("");
  const [filterField, setFilterField] = useState("");
  const [filterValue, setFilterValue] = useState("");
  const [filterOperator, setFilterOperator] = useState("contains");
  const [sortField, setSortField] = useState("");
  const [sortDirection, setSortDirection] = useState("ascending");
  const [selectedOnly, setSelectedOnly] = useState(false);
  const [snapshot, setSnapshot] = useState<OverviewSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [diagnostic, setDiagnostic] = useState<Diagnostic | null>(null);
  const requestSequence = useRef(0);

  const load = useCallback(async () => {
    if (!projectRoot || !table) return;
    const requestId = ++requestSequence.current;
    setLoading(true);
    setDiagnostic(null);
    try {
      const noValueOperator = filterOperator === "is-null" || filterOperator === "is-invalid";
      const filters = filterField && (noValueOperator || filterValue.length > 0)
        ? [{
            field: filterField,
            operator: filterOperator,
            ...(noValueOperator ? {} : { value: queryInputValue(snapshot?.columns.find((column) => column.name === filterField), filterValue) }),
          }]
        : [];
      const next = await invoke<OverviewSnapshot>("table_overview", {
        projectPath: projectRoot,
        request: {
          table,
          profile: profile || null,
          query: {
            search,
            filters,
            sort: sortField ? { field: sortField, direction: sortDirection } : null,
          },
          selectedOnly,
        },
      });
      if (requestId === requestSequence.current) setSnapshot(next);
    } catch (error) {
      if (requestId === requestSequence.current) {
        setDiagnostic(errorDiagnostic(error));
        setSnapshot(null);
      }
    } finally {
      if (requestId === requestSequence.current) setLoading(false);
    }
  }, [filterField, filterOperator, filterValue, profile, projectRoot, search, selectedOnly, snapshot, sortDirection, sortField, table]);

  useEffect(() => {
    if (active && projectRoot && table) void load();
    // Filters are applied explicitly from the toolbar; only project/table
    // changes should trigger the initial snapshot load.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, projectRoot, table]);

  const columns = snapshot?.columns ?? [];
  const profiles = workspace?.project.profiles ?? [];
  const profileMissing = profile.length > 0 && !profiles.some((item) => item.name === profile);

  if (!active) return <div hidden aria-hidden="true" />;

  return (
    <section className="surface-panel overview-panel" aria-label="Table Overview">
      <header className="surface-header">
        <div>
          <span className="dialog-kicker">READ-ONLY SNAPSHOT</span>
          <h2>Table Overview</h2>
          <p>{table ?? "Select a Table"} · saved source snapshot</p>
        </div>
        <Space>
          <Button htmlType="button" onClick={() => void load()} loading={loading} disabled={!table}>Refresh</Button>
          <Tag color={dirtySourceCount || dirtyConfig ? "gold" : "green"}>
            {dirtySourceCount || dirtyConfig ? "Unsaved input not included" : "Saved input"}
          </Tag>
        </Space>
      </header>
      <div className="surface-toolbar">
        <Input aria-label="Overview search" placeholder="Search all displayed fields" value={search} onChange={(event) => setSearch(event.target.value)} onPressEnter={() => void load()} />
        <Select aria-label="Overview profile" value={profile || "__unfiltered"} onChange={(value) => onProfileChange(value === "__unfiltered" ? "" : value)} options={[{ value: "__unfiltered", label: "Unfiltered" }, ...profiles.map((item) => ({ value: item.name, label: item.name }))]} />
        <Select aria-label="Overview filter field" allowClear placeholder="Filter field" value={filterField || undefined} onChange={(value) => setFilterField(value ?? "")} options={columns.map((column) => ({ value: column.name, label: column.name }))} />
        <Select aria-label="Overview filter operator" value={filterOperator} onChange={setFilterOperator} options={[{ value: "contains", label: "contains" }, { value: "equals", label: "equals" }, { value: "not-equals", label: "not equals" }, { value: "less-than", label: "<" }, { value: "greater-than", label: ">" }, { value: "is-null", label: "is null" }, { value: "is-invalid", label: "is invalid" }]} />
        <Input aria-label="Overview filter value" placeholder="Filter value" value={filterValue} onChange={(event) => setFilterValue(event.target.value)} />
        <Select aria-label="Overview sort field" allowClear placeholder="Sort by" value={sortField || undefined} onChange={(value) => setSortField(value ?? "")} options={columns.map((column) => ({ value: column.name, label: column.name }))} />
        <Select aria-label="Overview sort direction" value={sortDirection} onChange={setSortDirection} options={[{ value: "ascending", label: "A→Z" }, { value: "descending", label: "Z→A" }]} />
        <Button htmlType="button" onClick={() => void load()} type="primary">Apply</Button>
        <Button htmlType="button" onClick={() => setSelectedOnly((value) => !value)} aria-pressed={selectedOnly}>{selectedOnly ? "All rows" : "Selected only"}</Button>
      </div>
      {profileMissing && <Alert type="warning" showIcon title="Profile unavailable" description={`Profile “${profile}” is not available in the current project configuration. Refresh or choose another profile.`} />}
      {diagnostic && <Alert type="error" showIcon title={diagnostic.code} description={diagnostic.message} />}
      {snapshot && snapshot.status !== "complete" && <Alert type={snapshot.status === "stale" ? "warning" : "info"} showIcon title={`Overview ${snapshot.status}`} description={snapshot.diagnostics.map((item) => item.message).join(" ")} />}
      {loading && <div className="surface-loading">Loading saved snapshot…</div>}
      {!loading && !snapshot && !diagnostic && <Empty description="Open Overview for a saved Table snapshot." />}
      {snapshot && (
        <>
          <div className="surface-metrics" aria-label="Overview metrics">
            <span><strong>{snapshot.displayedCount}</strong> displayed / {snapshot.totalCount} total</span>
            <span>{snapshot.selectedCount == null ? "Selection unavailable" : `${snapshot.selectedCount} selected`}</span>
            <span>{snapshot.sources.length} source files</span>
            <span>Profile: {snapshot.selection.profile ?? "unfiltered"}</span>
          </div>
          <div className="surface-table-scroll">
            <table className="surface-table">
              <thead><tr><th>Source occurrence</th><th>Selection</th>{snapshot.columns.map((column) => <th key={column.name}>{column.name}{column.keyField ? " · KEY" : ""}</th>)}</tr></thead>
              <tbody>
                {snapshot.rows.map((row) => (
                  <tr key={`${row.sourcePath}:${row.recordIndex}`}>
                    <td><Button type="link" htmlType="button" onClick={() => onNavigate(row.sourcePath, row.recordIndex, snapshot.sources.find((source) => source.path === row.sourcePath)?.contentIdentity ?? "")}>{row.sourcePath} · record {row.recordIndex + 1}</Button></td>
                    <td>{row.selected === true ? "selected" : row.selected === false ? "excluded" : "unavailable"}{row.selectionReason && <small>{row.selectionReason}</small>}</td>
                    {row.values.map((value, index) => <td key={`${row.sourcePath}:${row.recordIndex}:${index}`}>{valueLabel(value)}</td>)}
                  </tr>
                ))}
              </tbody>
            </table>
            {snapshot.rows.length === 0 && <Empty description="No rows match the current query." />}
          </div>
        </>
      )}
    </section>
  );
}

export function ProjectSettingsPanel({
  active,
  projectRoot,
  mutationBlocked,
  onDirtyChange,
  onRegisterSave,
}: {
  active: boolean;
  projectRoot: string | null;
  mutationBlocked: boolean;
  onDirtyChange: (dirty: boolean) => void;
  onRegisterSave: (save: () => Promise<boolean>) => void;
}) {
  const [snapshot, setSnapshot] = useState<ConfigSnapshot | null>(null);
  const [selectedProfile, setSelectedProfile] = useState("");
  const [profileName, setProfileName] = useState("");
  const [includeTags, setIncludeTags] = useState("");
  const [excludeTags, setExcludeTags] = useState("");
  const [targetKind, setTargetKind] = useState<"csharp" | "binary">("csharp");
  const [targetPath, setTargetPath] = useState("");
  const [targetEditIndex, setTargetEditIndex] = useState<number | null>(null);
  const [pendingRequest, setPendingRequest] = useState<ConfigEditRequest | null>(null);
  const [preview, setPreview] = useState<ConfigEditPreview | null>(null);
  const [configConflict, setConfigConflict] = useState<ConfigSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [diagnostic, setDiagnostic] = useState<Diagnostic | null>(null);

  const load = useCallback(async () => {
    if (!projectRoot) return;
    setLoading(true);
    setDiagnostic(null);
    try {
      const next = await invoke<ConfigSnapshot>("open_project_config", { projectPath: projectRoot });
      setSnapshot(next);
      const first = next.profiles[0];
      if (first) {
        setSelectedProfile(first.name);
        setProfileName(first.name);
        setIncludeTags(first.include_tags.join(", "));
        setExcludeTags(first.exclude_tags.join(", "));
      }
      setPreview(null);
      setPendingRequest(null);
      setTargetEditIndex(null);
      setConfigConflict(null);
      onDirtyChange(false);
    } catch (error) {
      setDiagnostic(errorDiagnostic(error));
    } finally {
      setLoading(false);
    }
  }, [onDirtyChange, projectRoot]);

  useEffect(() => {
    if (active && projectRoot && (!snapshot || snapshot.projectRoot !== projectRoot)) void load();
  }, [active, load, projectRoot, snapshot]);

  const selectProfile = (name: string) => {
    setSelectedProfile(name);
    const profile = snapshot?.profiles.find((item) => item.name === name);
    setProfileName(profile?.name ?? name);
    setIncludeTags(profile?.include_tags.join(", ") ?? "");
    setExcludeTags(profile?.exclude_tags.join(", ") ?? "");
    setPreview(null);
    setConfigConflict(null);
  };

  const markDirty = () => {
    setPreview(null);
    setPendingRequest(null);
    onDirtyChange(true);
  };

  const requestForProfile = (): ConfigEditRequest => ({
    operation: selectedProfile ? "update_profile" : "add_profile",
    name: profileName.trim(),
    include_tags: splitTags(includeTags),
    exclude_tags: splitTags(excludeTags),
  });

  const previewRequest = useCallback(async (request: ConfigEditRequest) => {
    if (!snapshot) return false;
    setLoading(true);
    setDiagnostic(null);
    try {
      const next = await invoke<ConfigEditPreview>("preview_project_config_edit", {
        baseSource: snapshot.baseSource,
        baseContentIdentity: snapshot.baseContentIdentity,
        request,
      });
      setPendingRequest(request);
      setPreview(next);
      onDirtyChange(next.changed);
      return true;
    } catch (error) {
      setDiagnostic(errorDiagnostic(error));
      return false;
    } finally {
      setLoading(false);
    }
  }, [onDirtyChange, snapshot]);

  const save = useCallback(async (): Promise<boolean> => {
    if (!snapshot || !pendingRequest || !preview || mutationBlocked) return false;
    setLoading(true);
    setDiagnostic(null);
    try {
      const report = await invoke<ConfigSaveReport>("save_project_config_edit", {
        projectPath: projectRoot,
        baseSource: snapshot.baseSource,
        baseContentIdentity: snapshot.baseContentIdentity,
        request: pendingRequest,
        overwriteExpectedIdentity: null,
      });
      if (report.status === "success" && report.snapshot) {
        setSnapshot(report.snapshot);
        setPreview(null);
        setPendingRequest(null);
        onDirtyChange(false);
        return true;
      }
      if (report.status === "conflict" && report.current) setConfigConflict(report.current);
      setDiagnostic(report.diagnostic ?? { code: `E-CONFIG-${report.status.toUpperCase()}`, message: "Configuration was not saved." });
      return false;
    } catch (error) {
      setDiagnostic(errorDiagnostic(error));
      return false;
    } finally {
      setLoading(false);
    }
  }, [mutationBlocked, onDirtyChange, pendingRequest, preview, projectRoot, snapshot]);

  useEffect(() => {
    onRegisterSave(save);
  }, [onRegisterSave, save]);

  const addTarget = () => void previewRequest(targetEditIndex === null
    ? { operation: "add_publish_target", kind: targetKind, path: targetPath.trim() }
    : { operation: "update_publish_target_path", index: targetEditIndex, path: targetPath.trim() });
  const profileOptions = snapshot?.profiles.map((profile) => ({ value: profile.name, label: profile.name })) ?? [];

  if (!active) return <div hidden aria-hidden="true" />;

  return (
    <section className="surface-panel settings-panel" aria-label="Project Settings">
      <header className="surface-header">
        <div><span className="dialog-kicker">LOSSLESS TOML EDIT</span><h2>Project Settings</h2><p>{snapshot?.configPath ?? "masterdata.toml"}</p></div>
        <Space><Tag color={pendingRequest ? "gold" : "green"}>{pendingRequest ? "Config dirty" : "Saved"}</Tag><Button htmlType="button" onClick={() => void load()} loading={loading}>Reload Settings</Button></Space>
      </header>
      {diagnostic && <Alert type="error" showIcon title={diagnostic.code} description={diagnostic.message} />}
      {snapshot?.diagnostics.length ? <Alert type="warning" showIcon title="Configuration diagnostics" description={snapshot.diagnostics.map((item) => item.message).join(" ")} /> : null}
      {!snapshot && !loading && !diagnostic && <Empty description="Open Settings to inspect masterdata.toml." />}
      {snapshot && (
        <div className="settings-content">
          <section className="settings-card" aria-label="Build Profiles">
            <h3>Build Profiles</h3>
            <Select aria-label="Settings profile" allowClear placeholder="New profile" value={selectedProfile || undefined} onChange={(value) => selectProfile(value ?? "")} options={profileOptions} />
            <Input aria-label="Profile name" placeholder="Profile name" value={profileName} onChange={(event) => { setProfileName(event.target.value); markDirty(); }} />
            <Input aria-label="Include tags" placeholder="Include tags, comma separated" value={includeTags} onChange={(event) => { setIncludeTags(event.target.value); markDirty(); }} />
            <Input aria-label="Exclude tags" placeholder="Exclude tags, comma separated" value={excludeTags} onChange={(event) => { setExcludeTags(event.target.value); markDirty(); }} />
            <Button htmlType="button" type="primary" disabled={!profileName.trim() || loading} onClick={() => void previewRequest(requestForProfile())}>Preview Profile</Button>
            {snapshot.profiles.map((profile) => <div className="settings-line" key={profile.name}><strong>{profile.name}</strong><span>include: {profile.include_tags.join(", ") || "∅"}</span><span>exclude: {profile.exclude_tags.join(", ") || "∅"}</span></div>)}
          </section>
          <section className="settings-card" aria-label="Publish Targets">
            <h3>Publish Targets</h3>
            <Select aria-label="Publish target kind" value={targetKind} onChange={setTargetKind} options={[{ value: "csharp", label: "C# directory" }, { value: "binary", label: "Binary file" }]} />
            <Input aria-label="Publish target path" placeholder="Path relative to project or absolute" value={targetPath} onChange={(event) => { setTargetPath(event.target.value); markDirty(); }} />
            <Button htmlType="button" disabled={!targetPath.trim() || loading} onClick={addTarget}>{targetEditIndex === null ? "Preview Target" : "Preview Target Path"}</Button>
            {targetEditIndex !== null && <Button htmlType="button" onClick={() => { setTargetEditIndex(null); setTargetPath(""); }}>Cancel target edit</Button>}
            {snapshot.publishTargets.map((target, index) => <div className="settings-line" key={`${target.kind}:${index}`}><strong>{target.kind}</strong><span>{target.path}</span><Button size="small" htmlType="button" onClick={() => { setTargetEditIndex(index); setTargetKind(target.kind); setTargetPath(target.path); setConfigConflict(null); }}>Edit path</Button></div>)}
          </section>
          {preview && (
            <section className="settings-diff" aria-label="Configuration diff">
              <div className="surface-header"><div><h3>Pending config preview</h3><p>{preview.configValid ? "Candidate parses and validates." : "Candidate is domain-invalid; diagnostics remain visible."}</p></div><Button type="primary" htmlType="button" disabled={loading || mutationBlocked} onClick={() => void save()}>Save Settings</Button></div>
              {preview.diagnostics.map((item) => <Alert key={`${item.code}:${item.message}`} type="warning" title={item.code} description={item.message} />)}
              <pre>{preview.candidateSource}</pre>
            </section>
          )}
          {configConflict && (
            <section className="settings-diff" aria-label="Configuration conflict">
              <Alert type="warning" showIcon title="Configuration changed externally" description="The local settings preview was not written. Compare the external snapshot, reload it explicitly, or cancel this conflict." />
              <pre>{configConflict.baseSource}</pre>
              <Space>
                <Button htmlType="button" onClick={() => void load()}>Reload external settings</Button>
                <Button htmlType="button" onClick={() => setConfigConflict(null)}>Cancel conflict</Button>
              </Space>
            </section>
          )}
        </div>
      )}
    </section>
  );
}

export function DeliveryPanel({
  active,
  projectRoot,
  workspace,
  dirtySourceCount,
  dirtyConfig,
  profile,
  onProfileChange,
  mutationBlocked,
}: {
  active: boolean;
  projectRoot: string | null;
  workspace: SurfaceWorkspace | null;
  dirtySourceCount: number;
  dirtyConfig: boolean;
  profile: string;
  onProfileChange: (profile: string) => void;
  mutationBlocked: boolean;
}) {
  const [buildState, setBuildState] = useState<"idle" | "running" | "succeeded" | "failed">("idle");
  const [build, setBuild] = useState<BuildResponse | null>(null);
  const [publishPreview, setPublishPreview] = useState<PublishPreview | null>(null);
  const [publishState, setPublishState] = useState<"idle" | "loading" | "succeeded" | "failed">("idle");
  const [diagnostic, setDiagnostic] = useState<Diagnostic | null>(null);

  useEffect(() => {
    if (!active) return;
    setDiagnostic(null);
  }, [active]);

  const runBuild = async (thenPreview: boolean) => {
    if (!projectRoot || mutationBlocked || buildState === "running") return;
    setBuildState("running");
    setDiagnostic(null);
    setPublishPreview(null);
    try {
      const next = await invoke<BuildResponse>("build", { projectPath: projectRoot, dryRun: false, profile: profile || null });
      setBuild(next);
      setBuildState("succeeded");
      if (thenPreview) await loadPublishPreview();
    } catch (error) {
      setBuildState("failed");
      setDiagnostic(errorDiagnostic(error));
    }
  };

  const loadPublishPreview = async () => {
    if (!projectRoot) return;
    setPublishState("loading");
    setDiagnostic(null);
    try {
      const next = await invoke<PublishPreview>("publish_preview", { projectPath: projectRoot });
      setPublishPreview(next);
      setPublishState("idle");
    } catch (error) {
      setPublishState("failed");
      setDiagnostic(errorDiagnostic(error));
    }
  };

  const confirmPublish = async () => {
    if (!projectRoot || !publishPreview || mutationBlocked) return;
    setPublishState("loading");
    setDiagnostic(null);
    try {
      await invoke("publish_from_preview", {
        projectPath: projectRoot,
        artifactSetIdentity: publishPreview.artifactSetIdentity,
        configContentIdentity: publishPreview.configContentIdentity,
      });
      setPublishState("succeeded");
    } catch (error) {
      setPublishState("failed");
      setDiagnostic(errorDiagnostic(error));
    }
  };

  const profiles = workspace?.project.profiles ?? [];
  const profileMissing = profile.length > 0 && !profiles.some((item) => item.name === profile);
  if (!active) return <div hidden aria-hidden="true" />;

  return (
    <section className="surface-panel delivery-panel" aria-label="Build and Publish">
      <header className="surface-header"><div><span className="dialog-kicker">SAVED INPUT WORKFLOW</span><h2>Build / Publish</h2><p>Build and Publish are separate confirmed operations.</p></div><Tag color={dirtySourceCount || dirtyConfig ? "gold" : "green"}>{dirtySourceCount || dirtyConfig ? "Dirty input excluded" : "Saved input"}</Tag></header>
      <div className="delivery-summary"><span>Project: {workspace?.project.name ?? "No project"}</span><span>Native Build: {workspace ? "available" : "unavailable"}</span><span>Sources dirty: {dirtySourceCount}</span><span>Config dirty: {dirtyConfig ? "yes" : "no"}</span></div>
      <div className="surface-toolbar delivery-toolbar">
        <Select aria-label="Delivery profile" value={profile || "__unfiltered"} onChange={(value) => onProfileChange(value === "__unfiltered" ? "" : value)} options={[{ value: "__unfiltered", label: "Unfiltered" }, ...profiles.map((item) => ({ value: item.name, label: item.name }))]} />
        <Button type="primary" htmlType="button" disabled={!projectRoot || mutationBlocked || profileMissing || buildState === "running"} loading={buildState === "running"} onClick={() => void runBuild(false)}>Build saved input</Button>
        <Button htmlType="button" disabled={!projectRoot || mutationBlocked || profileMissing || buildState === "running"} onClick={() => void runBuild(true)}>Build and review Publish</Button>
        <Button htmlType="button" disabled={!projectRoot || publishState === "loading"} loading={publishState === "loading"} onClick={() => void loadPublishPreview()}>Publish preview</Button>
      </div>
      {profileMissing && <Alert type="warning" showIcon title="Profile unavailable" description={`Profile “${profile}” is not available in the current project configuration. Choose another profile before Build.`} />}
      {diagnostic && <Alert type="error" showIcon title={diagnostic.code} description={diagnostic.message} />}
      {build && <Alert type="success" showIcon title={`Build succeeded (${build.profile ?? "unfiltered"})`} description={`${build.generatedFiles.length} generated files · ${build.artifactRoot}`} />}
      {publishPreview && <section className="publish-preview" aria-label="Publish preview"><div className="surface-header"><div><h3>Publish preview</h3><p>Receipt and all target preflight completed. Confirm to mutate destinations.</p></div><Button type="primary" htmlType="button" disabled={mutationBlocked || publishState === "loading"} onClick={() => void confirmPublish()}>Confirm Publish</Button></div>{publishPreview.targets.length === 0 && <p>0 targets: successful no-op.</p>}{publishPreview.targets.map((target) => <div className="publish-target" key={target.index}><strong>{target.kind}</strong><span>{target.configuredPath}</span><span>{target.additions.length} additions · {target.updates.length} updates · {target.removals.length} removals{target.binaryReplacement ? " · binary replacement" : ""}</span></div>)}</section>}
      {publishState === "succeeded" && <Alert type="success" title="Publish completed" />}
      {publishState === "failed" && <Alert type="warning" title="Publish did not complete" description="Review the destination state and start a new preview before retrying." />}
    </section>
  );
}

export function ProjectCreatePanel({
  active,
  onCreated,
}: {
  active: boolean;
  onCreated: (projectRoot: string) => void;
}) {
  const [destination, setDestination] = useState("");
  const [projectId, setProjectId] = useState("");
  const [name, setName] = useState("");
  const [version, setVersion] = useState("0.1.0");
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<ProjectInitReport | null>(null);

  useEffect(() => {
    if (active) setReport(null);
  }, [active]);

  const create = async () => {
    setBusy(true);
    setReport(null);
    try {
      const next = await invoke<ProjectInitReport>("create_project", { request: { destination: destination.trim(), projectId: projectId.trim(), name: name.trim(), version: version.trim() } });
      setReport(next);
      if (next.status === "success" && next.project) onCreated(next.project.project_root);
    } catch (error) {
      setReport({ status: "failure", destination, project: null, createdEntries: [], diagnostic: errorDiagnostic(error) });
    } finally {
      setBusy(false);
    }
  };

  if (!active) return <div hidden aria-hidden="true" />;

  return (
    <section className="surface-panel create-panel" aria-label="Create Project">
      <header className="surface-header"><div><span className="dialog-kicker">EXCLUSIVE INITIALIZATION</span><h2>Create Project</h2><p>Only an empty existing directory or a new child directory is accepted.</p></div></header>
      <div className="create-form">
        <label>Destination<Input aria-label="Project destination" value={destination} onChange={(event) => setDestination(event.target.value)} placeholder="/path/to/project" /></label>
        <label>Project ID<Input aria-label="New project ID" value={projectId} onChange={(event) => setProjectId(event.target.value)} placeholder="game.masterdata" /></label>
        <label>Name<Input aria-label="New project name" value={name} onChange={(event) => setName(event.target.value)} placeholder="Game Master Data" /></label>
        <label>Version<Input aria-label="New project version" value={version} onChange={(event) => setVersion(event.target.value)} /></label>
        <Button type="primary" htmlType="button" loading={busy} disabled={!destination.trim() || !projectId.trim() || !name.trim() || !version.trim()} onClick={() => void create()}>Create Project</Button>
      </div>
      {report?.diagnostic && <Alert type="error" showIcon title={report.diagnostic.code} description={report.diagnostic.message} />}
      {report?.status === "success" && <Alert type="success" showIcon title="Project created" description={report.destination} />}
    </section>
  );
}
