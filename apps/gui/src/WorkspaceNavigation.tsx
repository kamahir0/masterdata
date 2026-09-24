import { useEffect, useMemo, useState, type ReactNode } from "react";
import { Button } from "antd";
import { ChevronDown, ChevronRight, Plus } from "lucide-react";

export type NavigationFile = {
  path: string;
  sourceRoot: string;
  kind: string;
  table: string | null;
  typeName: string | null;
};

export type TableNavigationItem = {
  name: string;
  schema: NavigationFile[];
  data: NavigationFile[];
};

export function groupNavigationFiles(files: NavigationFile[]): { tables: TableNavigationItem[]; types: NavigationFile[] } {
  const byTable = new Map<string, TableNavigationItem>();
  const types: NavigationFile[] = [];
  for (const file of files) {
    if (file.table && (file.kind === "schema" || file.kind === "data")) {
      const item = byTable.get(file.table) ?? { name: file.table, schema: [], data: [] };
      (file.kind === "schema" ? item.schema : item.data).push(file);
      byTable.set(file.table, item);
    }
    if (file.kind === "type" && file.typeName) types.push(file);
  }
  const byPath = (left: NavigationFile, right: NavigationFile) => left.path.localeCompare(right.path);
  const tables = [...byTable.values()].sort((left, right) => left.name.localeCompare(right.name));
  for (const table of tables) {
    table.schema.sort(byPath);
    table.data.sort(byPath);
  }
  types.sort((left, right) => left.typeName!.localeCompare(right.typeName!) || byPath(left, right));
  return { tables, types };
}

type Section = "tables" | "types" | "sources" | "project";

function basename(path: string): string {
  return path.split("/").at(-1) ?? path;
}

export function WorkspaceNavigation({
  files,
  activePath,
  selectedTable,
  revealSourcePath,
  surface,
  dirtyPaths,
  canWrite,
  buildBlocked,
  validating,
  building,
  onSelectFile,
  onSelectTable,
  onSettings,
  onDelivery,
  onValidate,
  onBuild,
  onCreate,
  onCreateData,
  sources,
}: {
  files: NavigationFile[];
  activePath: string | null;
  selectedTable: string | null;
  revealSourcePath: string | null;
  surface: "editor" | "overview" | "settings" | "delivery" | "create";
  dirtyPaths: Set<string>;
  canWrite: boolean;
  buildBlocked: boolean;
  validating: boolean;
  building: boolean;
  onSelectFile: (path: string) => void;
  onSelectTable: (table: string) => void;
  onSettings: () => void;
  onDelivery: () => void;
  onValidate: () => void;
  onBuild: () => void;
  onCreate: (kind: "table" | "type" | "source") => void;
  onCreateData: (table: string) => void;
  sources: ReactNode;
}) {
  const { tables, types } = useMemo(() => groupNavigationFiles(files), [files]);
  const [open, setOpen] = useState<Record<Section, boolean>>({ tables: true, types: false, sources: false, project: false });
  const [openTables, setOpenTables] = useState<Set<string>>(() => new Set());

  useEffect(() => {
    if (!selectedTable) return;
    setOpenTables((current) => current.has(selectedTable) ? current : new Set([...current, selectedTable]));
  }, [selectedTable]);

  useEffect(() => {
    const file = files.find((candidate) => candidate.path === activePath);
    if (file && !((file.table && (file.kind === "schema" || file.kind === "data")) || (file.kind === "type" && file.typeName))) {
      setOpen((current) => current.sources ? current : { ...current, sources: true });
    }
    if (file?.kind === "type" && file.typeName) setOpen((current) => current.types ? current : { ...current, types: true });
  }, [activePath, files]);

  useEffect(() => {
    if (revealSourcePath) setOpen((current) => current.sources ? current : { ...current, sources: true });
  }, [revealSourcePath]);

  const toggle = (section: Section) => setOpen((current) => ({ ...current, [section]: !current[section] }));
  const toggleTable = (table: string) => setOpenTables((current) => {
    const next = new Set(current);
    if (next.has(table)) next.delete(table); else next.add(table);
    return next;
  });
  const disclosure = (expanded: boolean) => expanded ? <ChevronDown size={14} aria-hidden="true" /> : <ChevronRight size={14} aria-hidden="true" />;

  return (
    <nav className="workspace-navigation" aria-label="Project navigation">
      <div className="nav-section">
        <div className="nav-section-heading">
          <button className="nav-disclosure" aria-expanded={open.tables} aria-controls="nav-tables" onClick={() => toggle("tables")}>{disclosure(open.tables)} Tables <span>{tables.length}</span></button>
          <Button size="small" type="text" aria-label="New Table" disabled={!canWrite} icon={<Plus size={14} />} onClick={() => onCreate("table")} />
        </div>
        <div id="nav-tables" hidden={!open.tables}>
          {tables.length === 0 && <p className="nav-empty">No Tables yet</p>}
          {tables.map((table) => {
            const expanded = openTables.has(table.name);
            const tableDirty = [...table.schema, ...table.data].some((file) => dirtyPaths.has(file.path));
            return <div className="nav-table" key={table.name}>
              <button className="nav-table-heading" aria-expanded={expanded} aria-controls={`nav-table-${encodeURIComponent(table.name)}`} onClick={() => toggleTable(table.name)}>
                {disclosure(expanded)} <span>{table.name}</span> {tableDirty && <span className="nav-dirty" aria-label="unsaved changes">●</span>}
              </button>
              <div id={`nav-table-${encodeURIComponent(table.name)}`} hidden={!expanded} className="nav-children">
                <button className={`nav-item ${surface === "overview" && selectedTable === table.name ? "selected" : ""}`} aria-label={`${table.name} Overview`} aria-current={surface === "overview" && selectedTable === table.name ? "page" : undefined} onClick={() => onSelectTable(table.name)}>Overview</button>
                {table.schema.map((file) => <button key={file.path} className={`nav-item ${surface === "editor" && activePath === file.path ? "selected" : ""}`} aria-current={surface === "editor" && activePath === file.path ? "page" : undefined} title={file.path} onClick={() => onSelectFile(file.path)}>Schema · {basename(file.path)}{dirtyPaths.has(file.path) && <span className="nav-dirty" aria-label="unsaved changes"> ●</span>}</button>)}
                {table.data.map((file) => <button key={file.path} className={`nav-item ${surface === "editor" && activePath === file.path ? "selected" : ""}`} aria-current={surface === "editor" && activePath === file.path ? "page" : undefined} title={file.path} onClick={() => onSelectFile(file.path)}>Data · {basename(file.path)}{dirtyPaths.has(file.path) && <span className="nav-dirty" aria-label="unsaved changes"> ●</span>}</button>)}
                <Button className="nav-add-data" size="small" type="text" disabled={!canWrite} onClick={() => onCreateData(table.name)}>+ Data file</Button>
              </div>
            </div>;
          })}
        </div>
      </div>

      <div className="nav-section">
        <div className="nav-section-heading">
          <button className="nav-disclosure" aria-expanded={open.types} aria-controls="nav-types" onClick={() => toggle("types")}>{disclosure(open.types)} Types <span>{types.length}</span></button>
          <Button size="small" type="text" aria-label="New Type" disabled={!canWrite} icon={<Plus size={14} />} onClick={() => onCreate("type")} />
        </div>
        <div id="nav-types" hidden={!open.types}>
          {types.length === 0 && <p className="nav-empty">No Types yet</p>}
          {types.map((file) => <button key={file.path} className={`nav-item ${surface === "editor" && activePath === file.path ? "selected" : ""}`} aria-current={surface === "editor" && activePath === file.path ? "page" : undefined} title={file.path} onClick={() => onSelectFile(file.path)}>{file.typeName}{dirtyPaths.has(file.path) && <span className="nav-dirty" aria-label="unsaved changes"> ●</span>}</button>)}
        </div>
      </div>

      <div className="nav-section">
        <div className="nav-section-heading">
          <button className="nav-disclosure" aria-expanded={open.sources} aria-controls="nav-sources" onClick={() => toggle("sources")}>{disclosure(open.sources)} Source Files</button>
          <Button size="small" type="text" aria-label="New source artifact" disabled={!canWrite} icon={<Plus size={14} />} onClick={() => onCreate("source")} />
        </div>
        <div id="nav-sources" hidden={!open.sources}>{sources}</div>
      </div>

      <div className="nav-section nav-project-section">
        <div className="nav-section-heading">
          <button className="nav-disclosure" aria-expanded={open.project} aria-controls="nav-project" onClick={() => toggle("project")}>{disclosure(open.project)} Project</button>
        </div>
        <div id="nav-project" hidden={!open.project}>
          <button className={`nav-item ${surface === "settings" ? "selected" : ""}`} aria-current={surface === "settings" ? "page" : undefined} onClick={onSettings}>Settings</button>
          <button className={`nav-item ${surface === "delivery" ? "selected" : ""}`} aria-current={surface === "delivery" ? "page" : undefined} onClick={onDelivery}>Build &amp; Publish</button>
          <Button className="nav-action" disabled={validating} loading={validating} onClick={onValidate}>Validate</Button>
          <Button className="nav-action" disabled={buildBlocked || building} loading={building} onClick={onBuild}>Build</Button>
        </div>
      </div>
    </nav>
  );
}
