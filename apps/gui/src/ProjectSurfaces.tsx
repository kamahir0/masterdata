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

type ConfigPublishTargetInfo = {
  occurrence: number;
  kind: "csharp" | "binary" | null;
  path: string | null;
  resolved_path: string | null;
  editable: boolean;
  reason: string | null;
  location: string;
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
  publishTargets: ConfigPublishTargetInfo[];
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
  publishPlanIdentity: string;
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

type PublishExecutionView = {
  status: "success" | "failure";
  report: {
    targets: Array<{
      index: number;
      kind: "csharp" | "binary";
      configured_path: string;
      destination: string;
      status: "not_attempted" | "succeeded" | "failed";
      failure?: Diagnostic | null;
    }>;
  };
  diagnostic: Diagnostic | null;
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
  onSaved,
}: {
  active: boolean;
  projectRoot: string | null;
  mutationBlocked: boolean;
  onDirtyChange: (dirty: boolean) => void;
  onRegisterSave: (save: () => Promise<boolean>) => void;
  onSaved: () => Promise<boolean>;
}) {
  const [snapshot, setSnapshot] = useState<ConfigSnapshot | null>(null);
  const [bufferSource, setBufferSource] = useState("");
  const [bufferIdentity, setBufferIdentity] = useState("");
  const [pendingRequests, setPendingRequests] = useState<ConfigEditRequest[]>([]);
  const [selectedProfile, setSelectedProfile] = useState("");
  const [profileDrafts, setProfileDrafts] = useState<Record<string, { name: string; includeTags: string; excludeTags: string }>>({});
  const [profileName, setProfileName] = useState("");
  const [includeTags, setIncludeTags] = useState("");
  const [excludeTags, setExcludeTags] = useState("");
  const [targetKind, setTargetKind] = useState<"csharp" | "binary">("csharp");
  const [targetPath, setTargetPath] = useState("");
  const [targetEditOccurrence, setTargetEditOccurrence] = useState<number | null>(null);
  const [draftKind, setDraftKind] = useState<"profile" | "target" | null>(null);
  const [preview, setPreview] = useState<ConfigEditPreview | null>(null);
  const [configConflict, setConfigConflict] = useState<ConfigSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [diagnostic, setDiagnostic] = useState<Diagnostic | null>(null);

  const isDirty = pendingRequests.length > 0 || draftKind !== null;

  const populateProfile = useCallback((next: ConfigSnapshot, name: string) => {
    const draft = profileDrafts[name];
    const profile = next.profiles.find((item) => item.name === name);
    setSelectedProfile(name);
    setProfileName(draft?.name ?? profile?.name ?? name);
    setIncludeTags(draft?.includeTags ?? profile?.include_tags.join(", ") ?? "");
    setExcludeTags(draft?.excludeTags ?? profile?.exclude_tags.join(", ") ?? "");
  }, [profileDrafts]);

  const installSnapshot = useCallback((next: ConfigSnapshot) => {
    setSnapshot(next);
    setBufferSource(next.baseSource);
    setBufferIdentity(next.baseContentIdentity);
    setPendingRequests([]);
    setPreview(null);
    setDraftKind(null);
    setTargetEditOccurrence(null);
    setTargetPath("");
    setConfigConflict(null);
    setProfileDrafts({});
    const first = next.profiles[0];
    setSelectedProfile(first?.name ?? "");
    setProfileName(first?.name ?? "");
    setIncludeTags(first?.include_tags.join(", ") ?? "");
    setExcludeTags(first?.exclude_tags.join(", ") ?? "");
    onDirtyChange(false);
  }, [onDirtyChange]);

  const load = useCallback(async (discardDirty = false) => {
    if (!projectRoot) return false;
    if (isDirty && !discardDirty) {
      const discard = window.confirm("Discard unsaved Project Settings and reload masterdata.toml?");
      if (!discard) return false;
    }
    setLoading(true);
    setDiagnostic(null);
    try {
      const next = await invoke<ConfigSnapshot>("open_project_config", { projectPath: projectRoot });
      installSnapshot(next);
      return true;
    } catch (error) {
      setDiagnostic(errorDiagnostic(error));
      return false;
    } finally {
      setLoading(false);
    }
  }, [installSnapshot, isDirty, projectRoot]);

  useEffect(() => {
    if (active && projectRoot && (!snapshot || snapshot.projectRoot !== projectRoot)) void load(true);
  }, [active, load, projectRoot, snapshot]);

  const requestForProfile = useCallback((): ConfigEditRequest => ({
    operation: selectedProfile ? "update_profile" : "add_profile",
    name: profileName.trim(),
    include_tags: splitTags(includeTags),
    exclude_tags: splitTags(excludeTags),
  }), [excludeTags, includeTags, profileName, selectedProfile]);

  const requestForTarget = useCallback((): ConfigEditRequest => (
    targetEditOccurrence === null
      ? { operation: "add_publish_target", kind: targetKind, path: targetPath.trim() }
      : { operation: "update_publish_target_path", index: targetEditOccurrence, path: targetPath.trim() }
  ), [targetEditOccurrence, targetKind, targetPath]);

  const applyRequestToBuffer = useCallback(async (
    request: ConfigEditRequest,
    currentRequests = pendingRequests,
    currentSource = bufferSource,
    currentIdentity = bufferIdentity,
  ) => {
    const next = await invoke<ConfigEditPreview>("preview_project_config_edit", {
      baseSource: currentSource,
      baseContentIdentity: currentIdentity,
      request,
    });
    const requests = next.changed ? [...currentRequests, request] : currentRequests;
    if (next.changed) {
      setPendingRequests(requests);
      setBufferSource(next.candidateSource);
      setBufferIdentity(next.candidateContentIdentity);
    }
    setPreview(next);
    setDraftKind(null);
    onDirtyChange(requests.length > 0);
    return { requests, source: next.candidateSource, identity: next.candidateContentIdentity, preview: next };
  }, [bufferIdentity, bufferSource, onDirtyChange, pendingRequests]);

  const rememberProfileDraft = useCallback((name: string) => {
    const normalized = name.trim();
    if (!normalized) return;
    setProfileDrafts((current) => ({
      ...current,
      [normalized]: { name: normalized, includeTags, excludeTags },
    }));
  }, [excludeTags, includeTags]);

  const previewRequest = useCallback(async (request: ConfigEditRequest) => {
    setLoading(true);
    setDiagnostic(null);
    try {
      await applyRequestToBuffer(request);
      return true;
    } catch (error) {
      setDiagnostic(errorDiagnostic(error));
      return false;
    } finally {
      setLoading(false);
    }
  }, [applyRequestToBuffer]);

  const commitCurrentDraft = useCallback(async () => {
    if (draftKind === "profile") {
      if (!profileName.trim()) return false;
      const request = requestForProfile();
      const committed = await previewRequest(request);
      if (committed) {
        rememberProfileDraft(request.name);
        setSelectedProfile(request.name);
      }
      return committed;
    }
    if (draftKind === "target") {
      if (!targetPath.trim()) return false;
      return previewRequest(requestForTarget());
    }
    return true;
  }, [draftKind, previewRequest, profileName, rememberProfileDraft, requestForProfile, requestForTarget, targetPath]);

  const selectProfile = useCallback(async (name: string) => {
    if (draftKind && !(await commitCurrentDraft())) return;
    if (!snapshot) return;
    populateProfile(snapshot, name);
    setConfigConflict(null);
  }, [commitCurrentDraft, draftKind, populateProfile, snapshot]);

  const markDirty = (kind: "profile" | "target") => {
    setDraftKind(kind);
    setPreview(null);
    setConfigConflict(null);
    onDirtyChange(true);
  };

  const save = useCallback(async (): Promise<boolean> => {
    if (!snapshot || mutationBlocked) return false;
    setLoading(true);
    setDiagnostic(null);
    try {
      let requests = pendingRequests;
      let source = bufferSource;
      let identity = bufferIdentity;
      if (draftKind) {
        const request = draftKind === "profile" ? requestForProfile() : requestForTarget();
        if ((draftKind === "profile" && !profileName.trim()) || (draftKind === "target" && !targetPath.trim())) {
          return false;
        }
        const applied = await applyRequestToBuffer(request, requests, source, identity);
        requests = applied.requests;
        source = applied.source;
        identity = applied.identity;
      }
      if (requests.length === 0) {
        onDirtyChange(false);
        return true;
      }
      const report = await invoke<ConfigSaveReport>("save_project_config_edit", {
        projectPath: projectRoot,
        baseSource: snapshot.baseSource,
        baseContentIdentity: snapshot.baseContentIdentity,
        requests,
      });
      if (report.status === "success" && report.snapshot) {
        installSnapshot(report.snapshot);
        const rebound = await onSaved();
        if (!rebound) {
          setDiagnostic({
            code: "E-CONFIG-REBIND",
            message: "Settings were saved, but the workspace binding could not be refreshed. Resolve configuration diagnostics before saving source files.",
          });
        }
        return rebound;
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
  }, [
    applyRequestToBuffer, bufferIdentity, bufferSource, draftKind, installSnapshot, mutationBlocked,
    onDirtyChange, onSaved, pendingRequests, profileName, projectRoot, requestForProfile,
    requestForTarget, snapshot, targetPath,
  ]);

  useEffect(() => {
    onRegisterSave(save);
  }, [onRegisterSave, save]);

  const profileOptions = Array.from(new Set([
    ...(snapshot?.profiles.map((profile) => profile.name) ?? []),
    ...Object.keys(profileDrafts),
  ])).map((name) => ({ value: name, label: name }));

  if (!active) return <div hidden aria-hidden="true" />;

  return (
    <section className="surface-panel settings-panel" aria-label="Project Settings">
      <header className="surface-header">
        <div><span className="dialog-kicker">LOSSLESS TOML EDIT</span><h2>Project Settings</h2><p>{snapshot?.configPath ?? "masterdata.toml"}</p></div>
        <Space>
          <Tag color={isDirty ? "gold" : "green"}>{isDirty ? "Config dirty" : "Saved"}</Tag>
          <Button htmlType="button" onClick={() => void load(false)} loading={loading}>Reload Settings</Button>
        </Space>
      </header>
      {diagnostic && <Alert type="error" showIcon title={diagnostic.code} description={diagnostic.message} />}
      {snapshot?.diagnostics.length ? <Alert type="warning" showIcon title="Configuration diagnostics" description={snapshot.diagnostics.map((item) => item.message).join(" ")} /> : null}
      {!snapshot && !loading && !diagnostic && <Empty description="Open Settings to inspect masterdata.toml." />}
      {snapshot && (
        <div className="settings-content">
          <section className="settings-card" aria-label="Build Profiles">
            <h3>Build Profiles</h3>
            <Select aria-label="Settings profile" allowClear placeholder="New profile" value={selectedProfile || undefined} onChange={(value) => void selectProfile(value ?? "")} options={profileOptions} />
            <Input aria-label="Profile name" placeholder="Profile name" value={profileName} onChange={(event) => { setProfileName(event.target.value); markDirty("profile"); }} />
            <Input aria-label="Include tags" placeholder="Include tags, comma separated" value={includeTags} onChange={(event) => { setIncludeTags(event.target.value); markDirty("profile"); }} />
            <Input aria-label="Exclude tags" placeholder="Exclude tags, comma separated" value={excludeTags} onChange={(event) => { setExcludeTags(event.target.value); markDirty("profile"); }} />
            <Button
              htmlType="button"
              type="primary"
              disabled={!profileName.trim() || loading}
              onClick={() => {
                const request = requestForProfile();
                void previewRequest(request).then((committed) => {
                  if (committed) {
                    rememberProfileDraft(request.name);
                    setSelectedProfile(request.name);
                  }
                });
              }}
            >Apply Profile to buffer</Button>
            {snapshot.profiles.map((profile) => <div className="settings-line" key={profile.name}><strong>{profile.name}</strong><span>include: {profile.include_tags.join(", ") || "∅"}</span><span>exclude: {profile.exclude_tags.join(", ") || "∅"}</span></div>)}
          </section>
          <section className="settings-card" aria-label="Publish Targets">
            <h3>Publish Targets</h3>
            <Select aria-label="Publish target kind" value={targetKind} onChange={(value) => { setTargetKind(value); markDirty("target"); }} options={[{ value: "csharp", label: "C# directory" }, { value: "binary", label: "Binary file" }]} />
            <Input aria-label="Publish target path" placeholder="Path relative to project or absolute" value={targetPath} onChange={(event) => { setTargetPath(event.target.value); markDirty("target"); }} />
            <Button htmlType="button" disabled={!targetPath.trim() || loading} onClick={() => void previewRequest(requestForTarget())}>{targetEditOccurrence === null ? "Apply Target to buffer" : "Apply Target Path to buffer"}</Button>
            {targetEditOccurrence !== null && <Button htmlType="button" onClick={() => { setTargetEditOccurrence(null); setTargetPath(""); setDraftKind(null); }}>Cancel target edit</Button>}
            {snapshot.publishTargets.map((target) => (
              <div className="settings-line" key={target.occurrence}>
                <strong>{target.kind ?? "unsupported"}</strong>
                <span>{target.path ?? target.location}</span>
                {target.editable && target.kind && target.path
                  ? <Button size="small" htmlType="button" onClick={() => { setTargetEditOccurrence(target.occurrence); setTargetKind(target.kind!); setTargetPath(target.path!); setDraftKind(null); setConfigConflict(null); }}>Edit path</Button>
                  : <Tag color="orange">{target.reason ?? "Read-only unsupported setting"} · {target.location}</Tag>}
              </div>
            ))}
          </section>
          {preview && (
            <section className="settings-diff" aria-label="Configuration diff">
              <div className="surface-header"><div><h3>Config buffer preview</h3><p>{preview.configValid ? "Buffered candidate parses and validates." : "Buffered candidate is domain-invalid; diagnostics remain visible."}</p></div><Button type="primary" htmlType="button" disabled={loading || mutationBlocked} onClick={() => void save()}>Save Settings</Button></div>
              {preview.diagnostics.map((item) => <Alert key={`${item.code}:${item.message}`} type="warning" title={item.code} description={item.message} />)}
              <pre>{bufferSource}</pre>
            </section>
          )}
          {configConflict && (
            <section className="settings-diff" aria-label="Configuration conflict">
              <Alert type="warning" showIcon title="Configuration changed externally" description="The local settings buffer was not written. Reload only if you explicitly want to discard the local buffer." />
              <pre>{configConflict.baseSource}</pre>
              <Space>
                <Button htmlType="button" onClick={() => void load(false)}>Reload external settings</Button>
                <Button htmlType="button" onClick={() => setConfigConflict(null)}>Keep local buffer</Button>
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
  buildBlocked,
  publishBlocked,
  onBusyChange,
}: {
  active: boolean;
  projectRoot: string | null;
  workspace: SurfaceWorkspace | null;
  dirtySourceCount: number;
  dirtyConfig: boolean;
  profile: string;
  onProfileChange: (profile: string) => void;
  buildBlocked: boolean;
  publishBlocked: boolean;
  onBusyChange: (busy: boolean) => void;
}) {
  const [buildState, setBuildState] = useState<"idle" | "running" | "succeeded" | "failed">("idle");
  const [build, setBuild] = useState<BuildResponse | null>(null);
  const [publishPreview, setPublishPreview] = useState<PublishPreview | null>(null);
  const [publishState, setPublishState] = useState<"idle" | "loading" | "succeeded" | "failed" | "unknown">("idle");
  const [publishExecution, setPublishExecution] = useState<PublishExecutionView | null>(null);
  const [diagnostic, setDiagnostic] = useState<Diagnostic | null>(null);

  const busy = buildState === "running" || publishState === "loading";
  useEffect(() => {
    onBusyChange(busy);
    return () => onBusyChange(false);
  }, [busy, onBusyChange]);

  useEffect(() => {
    if (!active) return;
    setDiagnostic(null);
  }, [active]);

  const loadPublishPreview = async () => {
    if (!projectRoot || publishBlocked) return;
    setPublishState("loading");
    setPublishExecution(null);
    setDiagnostic(null);
    try {
      const next = await invoke<PublishPreview>("publish_preview", { projectPath: projectRoot });
      setPublishPreview(next);
      setPublishState("idle");
    } catch (error) {
      setPublishPreview(null);
      setPublishState("failed");
      setDiagnostic(errorDiagnostic(error));
    }
  };

  const runBuild = async (thenPreview: boolean) => {
    if (!projectRoot || buildBlocked || buildState === "running") return;
    setBuildState("running");
    setBuild(null);
    setDiagnostic(null);
    setPublishPreview(null);
    setPublishExecution(null);
    try {
      const next = await invoke<BuildResponse>("build", { projectPath: projectRoot, dryRun: false, profile: profile || null });
      setBuild(next);
      setBuildState("succeeded");
      if (thenPreview) await loadPublishPreview();
    } catch (error) {
      setBuild(null);
      setBuildState("failed");
      setDiagnostic(errorDiagnostic(error));
    }
  };

  const confirmPublish = async () => {
    if (!projectRoot || !publishPreview || publishBlocked) return;
    const confirmed = publishPreview;
    setPublishPreview(null);
    setPublishState("loading");
    setPublishExecution(null);
    setDiagnostic(null);
    try {
      const result = await invoke<PublishExecutionView>("publish_from_preview", {
        projectPath: projectRoot,
        artifactSetIdentity: confirmed.artifactSetIdentity,
        configContentIdentity: confirmed.configContentIdentity,
        publishPlanIdentity: confirmed.publishPlanIdentity,
      });
      setPublishExecution(result);
      setPublishState(result.status === "success" ? "succeeded" : "failed");
      if (result.diagnostic) setDiagnostic(result.diagnostic);
    } catch (error) {
      setPublishState("unknown");
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
        <Button type="primary" htmlType="button" disabled={!projectRoot || buildBlocked || profileMissing || buildState === "running"} loading={buildState === "running"} onClick={() => void runBuild(false)}>Build saved input</Button>
        <Button htmlType="button" disabled={!projectRoot || buildBlocked || profileMissing || buildState === "running"} onClick={() => void runBuild(true)}>Build and review Publish</Button>
        <Button htmlType="button" disabled={!projectRoot || publishBlocked || publishState === "loading"} loading={publishState === "loading"} onClick={() => void loadPublishPreview()}>Publish preview</Button>
      </div>
      {profileMissing && <Alert type="warning" showIcon title="Profile unavailable" description={`Profile “${profile}” is not available in the current project configuration. Choose another profile before Build.`} />}
      {diagnostic && <Alert type="error" showIcon title={diagnostic.code} description={diagnostic.message} />}
      {buildState === "succeeded" && build && <Alert type="success" showIcon title={`Build succeeded (${build.profile ?? "unfiltered"})`} description={`${build.generatedFiles.length} generated files · ${build.artifactRoot}`} />}
      {buildState === "failed" && <Alert type="warning" showIcon title="Build failed" description="No previous successful Build is shown as the current operation result." />}
      {publishPreview && <section className="publish-preview" aria-label="Publish preview"><div className="surface-header"><div><h3>Publish preview</h3><p>Receipt, destination ownership, and all target preflight are bound to this confirmation.</p></div><Button type="primary" htmlType="button" disabled={publishBlocked || publishState === "loading"} onClick={() => void confirmPublish()}>Confirm Publish</Button></div>{publishPreview.targets.length === 0 && <p>0 targets: successful no-op.</p>}{publishPreview.targets.map((target) => <div className="publish-target" key={target.index}><strong>{target.kind}</strong><span>{target.configuredPath}</span><span>{target.additions.length} additions · {target.updates.length} updates · {target.removals.length} removals{target.binaryReplacement ? " · binary replacement" : ""}</span></div>)}</section>}
      {publishExecution && (
        <section className="publish-preview" aria-label="Publish execution result">
          <h3>Publish result</h3>
          {publishExecution.report.targets.map((target) => (
            <div className="publish-target" key={target.index}>
              <strong>{target.kind}</strong><span>{target.configured_path}</span><Tag>{target.status}</Tag>
              {target.failure && <span>{target.failure.code}: {target.failure.message}</span>}
            </div>
          ))}
        </section>
      )}
      {publishState === "succeeded" && <Alert type="success" title="Publish completed" />}
      {publishState === "failed" && <Alert type="warning" title="Publish did not complete" description="The previous confirmation is expired. Review target results and create a new preview before retrying." />}
      {publishState === "unknown" && <Alert type="error" title="Publish outcome unknown" description="The previous confirmation is expired. Recheck destination state and create a fresh preview before any retry." />}
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
