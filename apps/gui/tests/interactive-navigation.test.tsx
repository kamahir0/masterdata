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
