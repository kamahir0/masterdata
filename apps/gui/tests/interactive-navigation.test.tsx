import React from "react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import App from "../src/App";
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
const report = { valid: true, diagnostics: [] };
const paths = ["a.yaml", "b.yaml", "c.yaml"];
const workspace = { project: { project_root: "/project", name: "Navigation", project_id: "navigation" }, sourceRoots: ["."],
  files: paths.map(path => ({ path, sourceRoot: ".", kind: "data", table: path[0], typeName: null, hasInlineRecords: false, diagnostic: null })) };
function view(path: string) {
  const field = { key: 0, name: "id", type: "int", nullable: false, array: false };
  const shape = { name: "id", typeName: "int", modifier: "required", shape: { kind: "primitive", primitive: "int" } };
  return { requestedPath: path, path, generation: 1, files: workspace.files, currentSource: null, validationPending: false, typeSnapshot: null,
    context: { table: path[0], schemaPath: `${path}.schema`, schemaContentIdentity: "schema", schemaSource: "kind: schema", recordSources: [{ path, inline: false }], selectedRecordSource: path,
      schema: { path: `${path}.schema`, schema: { table: path[0], fields: [field], primaryKey: { fields: ["id"] }, secondaryKeys: [] }, fieldTypes: ["int"], initializerShapes: {}, references: [], referenceDiagnostics: [] } },
    data: { path, table: path[0], baseSource: `id: ${path[0]}`, baseContentIdentity: path, columns: [{ name: "id", typeName: "int", editable: true, keyField: true, shape, readOnlyReason: null }],
      rows: [{ recordIndex: 0, cells: [{ field: "id", text: "10", value: { kind: "number", value: "10" }, editable: true, readOnlyReason: null }] }], validation: report } };
}
let read: (path: string) => Promise<ReturnType<typeof view>>;
beforeEach(() => {
  read = async path => view(path);
  invoke.mockReset();
  invoke.mockImplementation(async (command, args) => {
    if (command === "open_workspace" || command === "refresh_workspace") return workspace;
    if (command === "select_source") return read(args.relativePath);
    if (command === "workspace_validation") return { generation: 1, validation: report, tagCandidatesComplete: true, tables: {} };
    if (command === "migration_recovery_status") return null;
    if (command === "load_application_user_state" || command === "set_recent_projects") return {};
    if (command === "preview_data_file") return { candidateSource: "id: 42", candidateContentIdentity: "draft", changed: true, validation: report };
    if (command === "preview_schema_draft") return { candidateSource: "schema draft", candidateContentIdentity: "schema-draft", changed: true, validation: report, selectedSnapshot: null };
    throw new Error(`Unexpected native request: ${command}`);
  });
});
afterEach(cleanup);
function select(path: string) { fireEvent.click(screen.getByRole("treeitem", { name: path, exact: true })); }

test("selection paints a new target while its read is pending and late results cannot roll it back", async () => {
  render(<App sourcePollingIntervalMs={null} />);
  await screen.findByRole("gridcell", { name: /record 1 id: 10/ });
  let finishB!: (value: ReturnType<typeof view>) => void;
  read = path => path === "b.yaml" ? new Promise(resolve => { finishB = resolve; }) : Promise.resolve(view(path));
  select("b.yaml");
  expect(screen.getByRole("treeitem", { name: "b.yaml", exact: true }).getAttribute("aria-selected")).toBe("true");
  expect(screen.getByRole("heading", { name: "Loading b.yaml…" })).toBeTruthy();
  expect(screen.queryByRole("gridcell")).toBeNull();
  await waitFor(() => expect(finishB).toBeTypeOf("function"));
  select("c.yaml");
  expect(screen.getByRole("heading", { name: "Loading c.yaml…" })).toBeTruthy();
  await act(async () => { finishB(view("b.yaml")); });
  await screen.findByRole("gridcell", { name: /record 1 id: 10/ });
  expect(document.querySelector(".editor-area")?.getAttribute("data-active-source")).toBe("c.yaml");
  expect(invoke.mock.calls.filter(([command]) => command === "select_source").map(([,args]) => args.relativePath)).toEqual(paths);
  expect(invoke.mock.calls.some(([command]) => ["open_data_file", "open_table_context", "open_table"].includes(command))).toBe(false);
});

test("rapid navigation retains dirty records, schema draft, and Undo/Redo history", async () => {
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const cell = await screen.findByRole("gridcell", { name: /record 1 id: 10/ });
  fireEvent.keyDown(cell, { key: "Enter" });
  const input = await screen.findByRole("textbox", { name: "record 1 id" });
  fireEvent.change(input, { target: { value: "42" } }); fireEvent.keyDown(input, { key: "Enter" });
  fireEvent.click(await screen.findByRole("button", { name: "Nullable id" }));
  await waitFor(() => expect(invoke.mock.calls.some(([command]) => command === "preview_schema_draft")).toBe(true));
  await act(async () => {
    for (let index=0; index<40; index++) select(paths[1+index%2]);
    fireEvent.click(screen.getByRole("treeitem", { name: "a.yaml, unsaved changes", exact: true }));
  });
  const restored = await screen.findByRole("gridcell", { name: /record 1 id: 42/ });
  expect(screen.getByRole("button", { name: "Nullable id" }).getAttribute("aria-pressed")).toBe("true");
  fireEvent.keyDown(restored, { key: "z", metaKey: true });
  await screen.findByRole("gridcell", { name: /record 1 id: 10/ });
  fireEvent.keyDown(window, { key: "z", metaKey: true, shiftKey: true });
  await screen.findByRole("gridcell", { name: /record 1 id: 42/ });
  expect(invoke.mock.calls.some(([command]) => /^(save_|apply_)/.test(command))).toBe(false);
}, 15000);

function typeView(path: string) {
  return { ...view(path), generation: 2, data: null, context: null,
    files: workspace.files.map(file => file.path === path ? { ...file, kind: "type", table: null, typeName: "Rarity" } : file),
    currentSource: { path, source: "kind: type", contentIdentity: "external-type" },
    typeSnapshot: { path, name: "Rarity", category: "Enum", underlying: "int", conversions: null, members: [{ name: "Rare", value: "1" }], fields: [], fieldTypes: ["int"], initializerShapes: {} } };
}

test("clean navigation uses current source ownership instead of the old Explorer classification", async () => {
  render(<App sourcePollingIntervalMs={null} />);
  await screen.findByRole("gridcell", { name: /record 1 id: 10/ });
  select("b.yaml");
  await waitFor(() => expect(document.querySelector(".editor-area")?.getAttribute("data-active-source")).toBe("b.yaml"));
  read = async path => path === "a.yaml" ? typeView(path) as any : view(path);
  select("a.yaml");
  await screen.findByRole("heading", { name: /Rarity/ });
  expect(screen.queryByRole("gridcell")).toBeNull();
});

test("changed source ownership retains the dirty buffer read-only until explicit Reload", async () => {
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const cell = await screen.findByRole("gridcell", { name: /record 1 id: 10/ });
  fireEvent.keyDown(cell, { key: "Enter" });
  const input = await screen.findByRole("textbox", { name: "record 1 id" });
  fireEvent.change(input, { target: { value: "42" } }); fireEvent.keyDown(input, { key: "Enter" });
  await screen.findByRole("gridcell", { name: /record 1 id: 42/ });
  select("b.yaml");
  await screen.findByRole("gridcell", { name: /record 1 id: 10/ });
  read = async path => path === "a.yaml" ? typeView(path) as any : view(path);
  fireEvent.click(screen.getByRole("treeitem", { name: "a.yaml, unsaved changes", exact: true }));
  const retained = await screen.findByRole("gridcell", { name: /record 1 id: 42/ });
  expect(retained.getAttribute("aria-readonly")).toBe("true");
  expect(screen.getByText("Source unavailable — local changes retained.")).toBeTruthy();
  const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);
  fireEvent.click(screen.getByRole("button", { name: "Reload", exact: true }));
  await screen.findByRole("heading", { name: /Rarity/ });
  expect(screen.queryByRole("gridcell")).toBeNull();
  expect(confirm).toHaveBeenCalledOnce(); confirm.mockRestore();
});

test("explicit Conflict Reload installs the external records and clears the old dirty history", async () => {
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const cell = await screen.findByRole("gridcell", { name: /record 1 id: 10/ });
  fireEvent.keyDown(cell, { key: "Enter" });
  const input = await screen.findByRole("textbox", { name: "record 1 id" });
  fireEvent.change(input, { target: { value: "42" } }); fireEvent.keyDown(input, { key: "Enter" });
  await screen.findByRole("gridcell", { name: /record 1 id: 42/ });
  select("b.yaml"); await screen.findByRole("gridcell", { name: /record 1 id: 10/ });
  read = async path => {
    const next = view(path);
    if (path === "a.yaml") { next.data.baseContentIdentity = "external"; next.data.rows[0].cells[0].text = "99"; next.data.rows[0].cells[0].value.value = "99"; }
    return next;
  };
  fireEvent.click(screen.getByRole("treeitem", { name: "a.yaml, unsaved changes", exact: true }));
  await screen.findByText("File changed outside masterdata.");
  expect(screen.getByRole("gridcell", { name: /record 1 id: 42/ })).toBeTruthy();
  const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);
  fireEvent.click(screen.getByRole("button", { name: "Reload", exact: true }));
  await screen.findByRole("gridcell", { name: /record 1 id: 99/ });
  expect(screen.queryByText("File changed outside masterdata.")).toBeNull();
  expect(screen.getByRole("treeitem", { name: "a.yaml", exact: true })).toBeTruthy();
  confirm.mockRestore();
});

test("a changed Project binding pauses authoring and keyboard Save while retaining local changes", async () => {
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const cell = await screen.findByRole("gridcell", { name: /record 1 id: 10/ });
  fireEvent.keyDown(cell, { key: "Enter" });
  const input = await screen.findByRole("textbox", { name: "record 1 id" });
  fireEvent.change(input, { target: { value: "42" } }); fireEvent.keyDown(input, { key: "Enter" });
  await screen.findByRole("gridcell", { name: /record 1 id: 42/ });
  select("b.yaml"); await screen.findByRole("gridcell", { name: /record 1 id: 10/ });
  read = async () => { throw { diagnostic: { code: "E-WORKSPACE-BINDING-CHANGED", kind: "validation", message: "Reload the Project", source: null } }; };
  fireEvent.click(screen.getByRole("treeitem", { name: "a.yaml, unsaved changes", exact: true }));
  const retained = await screen.findByRole("gridcell", { name: /record 1 id: 42/ });
  expect(retained.getAttribute("aria-readonly")).toBe("true");
  expect((screen.getByRole("button", { name: "Save", exact: true }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.keyDown(window, { key: "s", metaKey: true });
  expect(invoke.mock.calls.some(([command]) => /^(save_|apply_)/.test(command))).toBe(false);
});

test("old dirty-preview diagnostics cannot become current after a workspace generation changes", async () => {
  const original = invoke.getMockImplementation()!;
  let finishOld!: (value: unknown) => void;
  let previews = 0;
  invoke.mockImplementation((command, args) => command === "preview_data_file"
    ? new Promise(resolve => { if (++previews === 1) finishOld = resolve; }) : original(command, args));
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const cell = await screen.findByRole("gridcell", { name: /record 1 id: 10/ });
  fireEvent.keyDown(cell, { key: "Enter" });
  const input = await screen.findByRole("textbox", { name: "record 1 id" });
  fireEvent.change(input, { target: { value: "42" } }); fireEvent.keyDown(input, { key: "Enter" });
  await waitFor(() => expect(finishOld).toBeTypeOf("function"));
  read = async path => ({ ...view(path), generation: 2 });
  select("b.yaml"); await screen.findByRole("gridcell", { name: /record 1 id: 10/ });
  await waitFor(() => expect(previews).toBeGreaterThan(1));
  await act(async () => finishOld({ candidateSource: "id: 42", changed: true, validation: { valid: false,
    diagnostics: [{ code: "E-OLD", kind: "validation", message: "obsolete validation", source: "/project/a.yaml" }] } }));
  read = () => new Promise(() => {});
  fireEvent.click(screen.getByRole("treeitem", { name: "a.yaml, unsaved changes", exact: true }));
  await screen.findByRole("heading", { name: "Loading a.yaml…" });
  expect(screen.getByRole("button", { name: /PROBLEMS 0/ })).toBeTruthy();
});

test("a fresh generation restores the applied query while preserving unsubmitted search text", async () => {
  const original = invoke.getMockImplementation()!;
  invoke.mockImplementation(async (command, args) => command === "query_data_file"
    ? { orderedRecordIndices: [0], totalCount: 1, displayedCount: 1, query: args.request.query } : original(command, args));
  render(<App sourcePollingIntervalMs={null} />);
  const search = await screen.findByRole("searchbox", { name: "Data search" });
  fireEvent.change(search, { target: { value: "10" } });
  fireEvent.keyDown(search, { key: "Enter", code: "Enter" });
  await waitFor(() => expect(invoke.mock.calls.filter(([command]) => command === "query_data_file")).toHaveLength(1));
  await waitFor(() => expect(document.querySelector(".ant-input-search .ant-btn-loading")).toBeNull());
  fireEvent.change(search, { target: { value: "unsubmitted" } });
  read = async path => ({ ...view(path), generation: 2 });
  select("b.yaml");
  await screen.findByRole("gridcell", { name: /record 1 id: 10/ });
  select("a.yaml");
  await waitFor(() => expect(invoke.mock.calls.filter(([command]) => command === "query_data_file")).toHaveLength(2));
  expect(invoke.mock.calls.filter(([command]) => command === "query_data_file").at(-1)?.[1].request.query.search).toBe("10");
  expect((screen.getByRole("searchbox", { name: "Data search" }) as HTMLInputElement).value).toBe("unsubmitted");
  await waitFor(() => expect(document.querySelector(".ant-input-search .ant-btn-loading")).toBeNull());
});

test("an explicit disk validation cannot restore diagnostics from an older read generation", async () => {
  const original = invoke.getMockImplementation()!;
  let finish!: (value: unknown) => void;
  invoke.mockImplementation((command, args) => command === "validate"
    ? new Promise(resolve => { finish = resolve; }) : original(command, args));
  render(<App sourcePollingIntervalMs={null} />);
  await screen.findByRole("gridcell", { name: /record 1 id: 10/ });
  fireEvent.click(screen.getByRole("button", { name: "Project menu" }));
  fireEvent.click(await screen.findByRole("menuitem", { name: "Validate", exact: true }));
  await waitFor(() => expect(finish).toBeTypeOf("function"));
  read = async path => ({ ...view(path), generation: 2 });
  select("b.yaml");
  await screen.findByRole("gridcell", { name: /record 1 id: 10/ });
  await act(async () => finish({ valid: false, diagnostics: [{ code: "E-OLD", kind: "validation", message: "obsolete disk validation" }] }));
  expect(screen.getByRole("button", { name: /PROBLEMS 0/ })).toBeTruthy();
});

test("unchanged record bytes cannot restore edit permission while shared dependency reads fail", async () => {
  const original = invoke.getMockImplementation()!;
  invoke.mockImplementation(async (command, args) => {
    if (command === "workspace_status") return { generation: 1, workspace };
    if (command === "source_content") return { path: args.relativePath, source: view(args.relativePath).data.baseSource, contentIdentity: args.relativePath };
    return original(command, args);
  });
  render(<App sourcePollingIntervalMs={50} previewDelayMs={0} />);
  const cell = await screen.findByRole("gridcell", { name: /record 1 id: 10/ });
  fireEvent.keyDown(cell, { key: "Enter" });
  const input = await screen.findByRole("textbox", { name: "record 1 id" });
  fireEvent.change(input, { target: { value: "42" } }); fireEvent.keyDown(input, { key: "Enter" });
  select("b.yaml"); await screen.findByRole("gridcell", { name: /record 1 id: 10/ });
  read = async path => { if (path === "a.yaml") throw { diagnostic: { code: "E-DEPENDENCY", kind: "validation", message: "schema is unavailable" } }; return view(path); };
  fireEvent.click(screen.getByRole("treeitem", { name: "a.yaml, unsaved changes", exact: true }));
  const retained = await screen.findByRole("gridcell", { name: /record 1 id: 42/ });
  await waitFor(() => expect(invoke.mock.calls.filter(([command, args]) => command === "select_source" && args.relativePath === "a.yaml").length).toBeGreaterThan(2));
  expect(retained.getAttribute("aria-readonly")).toBe("true");
  read = async path => view(path);
  await waitFor(() => expect(screen.getByRole("gridcell", { name: /record 1 id: 42/ }).getAttribute("aria-readonly")).toBe("false"));
  expect(invoke.mock.calls.some(([command]) => /^(save_|apply_)/.test(command))).toBe(false);
});
