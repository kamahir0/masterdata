import { installWorkspaceFixture } from "./workspace-fixtures";
import React from 'react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import App from '../src/App';
const { invoke, openDialog, desktopWindow } = vi.hoisted(() => ({ invoke: vi.fn(), openDialog: vi.fn(), desktopWindow: { onCloseRequested: vi.fn(), destroy: vi.fn() } }));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => desktopWindow }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: openDialog }));
const validation = { valid: true, diagnostics: [] };
// Full-App jsdom/Ant Design flows have meaningful cross-platform CI variance.
 // This timeout is a hang guard, not a product performance budget.
const APP_INTEGRATION_TEST_TIMEOUT_MS = 30_000;
const numberShape = { name: 'weight', typeName: 'ulong', modifier: 'required', shape: { kind: 'primitive', primitive: 'ulong' } };
const snapshot = () => ({ path: 'data.yaml', table: 'item', baseSource: 'weight: 10', baseContentIdentity: 'base',
  columns: [{ name: 'weight', typeName: 'ulong', editable: true, keyField: false, shape: numberShape, readOnlyReason: null }],
  rows: [{ recordIndex: 0, cells: [{ field: 'weight', text: '10', value: { kind: 'number', value: '10' }, editable: true, readOnlyReason: null }] }], validation });
const mutationSnapshot = (rows = [{ recordIndex: 0, cells: [
  { field: 'id', text: '1', editable: true },
  { field: 'weight', text: '10', editable: true },
  { field: 'note', text: 'first', editable: true },
] }]) => ({
  path: 'data.yaml', table: 'item', baseSource: 'kind: data\ntable: item\nrecords: []\n', baseContentIdentity: 'base',
  columns: [
    { name: 'id', typeName: 'ulong', editable: true, keyField: true, shape: { name: 'id', typeName: 'ulong', modifier: 'required', shape: { kind: 'primitive', primitive: 'ulong' } }, readOnlyReason: null },
    { name: 'weight', typeName: 'ulong', editable: true, keyField: false, shape: numberShape, readOnlyReason: null },
    { name: 'note', typeName: 'string', editable: true, keyField: false, shape: { name: 'note', typeName: 'string', modifier: 'required', shape: { kind: 'primitive', primitive: 'string' } }, readOnlyReason: null },
  ],
  rows: rows.map((row) => ({ ...row, cells: row.cells.map((cell) => ({
    ...cell,
    value: cell.value ?? (cell.field === 'note'
      ? { kind: 'string', value: cell.text }
      : { kind: 'number', value: cell.text }),
    readOnlyReason: null,
  })) })),
  addRow: { supported: true, reason: null }, validation,
});
const workspace = { project: { project_root: '/project', name: 'Demo', project_id: 'demo' }, sourceRoots: ['.'],
  files: [{ path: 'data.yaml', sourceRoot: '.', kind: 'data', table: 'item', typeName: null, hasInlineRecords: false }] };
let openSnapshot: ReturnType<typeof snapshot>;
let preview: (args: any) => Promise<any>;
const originalPointerEvent = window.PointerEvent;
beforeEach(() => {
  // jsdom has no native PointerEvent constructor; retain real pointer coordinates.
  Object.defineProperty(window, 'PointerEvent', { configurable: true, value: MouseEvent });
  window.localStorage.clear();
  desktopWindow.onCloseRequested.mockReset();
  desktopWindow.onCloseRequested.mockResolvedValue(() => {});
  desktopWindow.destroy.mockReset();
  desktopWindow.destroy.mockResolvedValue(undefined);
  openDialog.mockReset();
  openDialog.mockResolvedValue(null);
  openSnapshot = snapshot();
  preview = async () => ({ candidateSource: 'weight: 20', changed: true, validation });
  invoke.mockReset();
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'load_application_user_state') return {};
    if (command === 'set_theme_preference') return {};
    if (command === 'set_recent_projects') return {};
    if (command === 'migration_recovery_status') return null;
    if (command === 'authoring_workspace') return workspace;
    if (command === 'open_table_context') return { table: 'item', schemaPath: 'schema.yaml', schemaContentIdentity: 'schema-base', schemaSource: 'kind: schema\ntable: item\n', recordSources: [{ path: 'data.yaml', inline: false }], selectedRecordSource: 'data.yaml', schema: { path: 'schema.yaml', schema: { table: 'item', fields: [{ key: 0, name: 'weight', type: 'ulong', nullable: false, array: false }], primaryKey: { fields: ['weight'] }, secondaryKeys: [] }, fieldTypes: ['ulong', 'string'] } };
    if (command === 'preview_schema_draft') return { candidateSource: 'kind: schema\ntable: item\n# draft\n', candidateContentIdentity: 'schema-candidate', changed: true, validation, selectedSnapshot: null };
    if (command === 'open_data_file') return structuredClone(openSnapshot);
    if (command === 'preview_data_file') return preview(args);
    if (command === 'save_data_file') return { status: 'success', snapshot: snapshot() };
    if (command === 'save_current_table_context') return { files: [
      ...(args.request.schemaDraft ? [{ path: 'schema.yaml', status: 'success', candidateContentIdentity: 'schema-candidate', current: null, diagnostic: null }] : []),
      ...(args.request.recordDraft ? [{ path: args.request.selectedRecordSource, status: 'success', candidateContentIdentity: 'base', current: null, diagnostic: null }] : []),
    ] };
    if (command === 'source_content') return { contentIdentity: 'base', source: 'weight: 10' };
    if (command === 'build') return { generatedFiles: [] };
    throw new Error(`Unexpected command: ${command}`);
  });
});
afterEach(() => {
  cleanup();
  Object.defineProperty(window, 'PointerEvent', { configurable: true, value: originalPointerEvent });
  window.localStorage.clear();
});
async function open(label = 'record 1 weight') { render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />); const cell = await screen.findByRole('gridcell', { name: new RegExp(`^${label}:`) }); fireEvent.keyDown(cell, { key: 'Enter' }); return await screen.findByRole('textbox', { name: label }); }
function commit(input: HTMLElement) { fireEvent.keyDown(input, { key: 'Enter' }); }
async function edit(label: string) { const cell = await screen.findByRole('gridcell', { name: new RegExp(`^${label}:`) }); fireEvent.keyDown(cell, { key: 'Enter' }); return await screen.findByRole('textbox', { name: label }); }
function navigation() { return within(screen.getByRole('complementary', { name: 'Explorer' })); }
function openProjectCommands() {
  fireEvent.click(within(document.querySelector('.titlebar')!).getByRole('button', { name: 'Project menu' }));
}
async function dataAction(name: string) {
  fireEvent.click(await screen.findByRole('button', { name: 'More data actions' }));
  fireEvent.click(await screen.findByRole('menuitem', { name, exact: true }));
}
function openSourceFiles() { /* Explorer is the persistent source tree. */ }
function reloadProject() {
  fireEvent.click(within(document.querySelector('.titlebar')!).getByRole('button', { name: 'Project menu' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Reload Project' }));
}

// Run the 2,000-row fixture before repeated full-App mounts: this keeps the
// bounded-rendering check from depending on accumulated jsdom test load.
// Its fixture and accessibility queries take about 6s locally and may exceed
// the default 10s test limit on a CI runner; the DOM row bound is still asserted.
test('large record grid mounts only nearby rows and navigates after scrolling', async () => {
  openSnapshot = mutationSnapshot(Array.from({ length: 2_000 }, (_, recordIndex) => ({
    recordIndex,
    cells: [
      { field: 'id', text: String(recordIndex + 1), editable: true },
      { field: 'weight', text: String(recordIndex + 1), editable: true },
      { field: 'note', text: `record ${recordIndex + 1}`, editable: true },
    ],
  }))) as ReturnType<typeof snapshot>;
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  await screen.findByRole('gridcell', { name: /^record 1 weight:/ });
  const scroll = document.querySelector<HTMLElement>('.grid-scroll')!;
  expect(scroll.querySelectorAll('tbody tr:not(.virtual-spacer)').length).toBeLessThan(60);
  scroll.scrollTop = 1_500 * 32;
  fireEvent.scroll(scroll);
  const farCell = await screen.findByRole('gridcell', { name: /^record 1501 weight:/ });
  expect(scroll.querySelectorAll('tbody tr:not(.virtual-spacer)').length).toBeLessThan(60);
  fireEvent.keyDown(farCell, { key: 'ArrowDown' });
  await waitFor(() => expect(document.activeElement?.getAttribute('aria-label')).toMatch(/^record 1502 weight:/));
}, 20_000);

test('initial Project-not-found is a Welcome state without an Explorer error', async () => {
  installWorkspaceFixture(invoke, async (command) => {
    if (command === 'authoring_workspace') throw { diagnostic: { code: 'E-PROJECT-NOT-FOUND', message: 'No project here' } };
    throw new Error(`Unexpected command: ${command}`);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  expect(await screen.findByRole('heading', { name: 'Start with a Masterdata project' })).toBeTruthy();
  await waitFor(() => expect(screen.queryByText('Looking for a configured project…')).toBeNull());
  expect(screen.queryByText('E-PROJECT-NOT-FOUND')).toBeNull();
  expect(screen.queryByRole('complementary', { name: 'Explorer' })).toBeNull();
  expect(screen.getByRole('complementary', { name: 'Recent Projects' })).toBeTruthy();
});

test('Open Project uses the native directory picker and remembers a successful selection', async () => {
  openDialog.mockResolvedValue('/project');
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'load_application_user_state') return {};
    if (command === 'set_theme_preference') return {};
    if (command === 'set_recent_projects') return {};
    if (command === 'authoring_workspace' && args.projectPath === null) {
      throw { diagnostic: { code: 'E-PROJECT-NOT-FOUND', message: 'No project here' } };
    }
    if (command === 'authoring_workspace') return workspace;
    if (command === 'migration_recovery_status') return null;
    if (command === 'open_data_file') return structuredClone(openSnapshot);
    throw new Error(`Unexpected command: ${command}`);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  await screen.findByRole('heading', { name: 'Start with a Masterdata project' });
  fireEvent.click(screen.getAllByRole('button', { name: 'Open Project', exact: true })[0]);
  expect(await screen.findByRole('complementary', { name: 'Explorer' })).toBeTruthy();
  expect(openDialog).toHaveBeenCalledWith(expect.objectContaining({ directory: true, multiple: false }));
  expect(invoke).toHaveBeenCalledWith('set_recent_projects', {
    projects: [{ root: '/project', name: 'Demo' }],
  });
});

test('cancelling the native Project picker leaves the Welcome state unchanged', async () => {
  installWorkspaceFixture(invoke, async (command) => {
    if (command === 'load_application_user_state') return {};
    if (command === 'set_theme_preference') return {};
    if (command === 'set_recent_projects') return {};
    if (command === 'authoring_workspace') throw { diagnostic: { code: 'E-PROJECT-NOT-FOUND', message: 'No project here' } };
    throw new Error(`Unexpected command: ${command}`);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  await screen.findByRole('heading', { name: 'Start with a Masterdata project' });
  const initialCalls = invoke.mock.calls.length;
  fireEvent.click(screen.getAllByRole('button', { name: 'Open Project', exact: true })[0]);
  await waitFor(() => expect(openDialog).toHaveBeenCalledOnce());
  expect(screen.getByRole('heading', { name: 'Start with a Masterdata project' })).toBeTruthy();
  expect(screen.queryByText('E-PROJECT-NOT-FOUND')).toBeNull();
  expect(invoke.mock.calls).toHaveLength(initialCalls);
});

test('explicit Project open failure stays on Welcome with a persistent diagnostic', async () => {
  openDialog.mockResolvedValue('/broken');
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'load_application_user_state') return {};
    if (command === 'set_theme_preference') return {};
    if (command === 'set_recent_projects') return {};
    if (command === 'authoring_workspace' && args.projectPath === null) {
      throw { diagnostic: { code: 'E-PROJECT-NOT-FOUND', message: 'No project here' } };
    }
    if (command === 'authoring_workspace') {
      throw { diagnostic: { code: 'E-PROJECT-CONFIG', message: 'masterdata.toml is invalid' } };
    }
    throw new Error(`Unexpected command: ${command}`);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  await screen.findByRole('heading', { name: 'Start with a Masterdata project' });
  fireEvent.click(screen.getAllByRole('button', { name: 'Open Project', exact: true })[0]);
  expect(await screen.findByText('E-PROJECT-CONFIG')).toBeTruthy();
  expect(screen.getByText('masterdata.toml is invalid')).toBeTruthy();
  expect(screen.queryByRole('complementary', { name: 'Explorer' })).toBeNull();
});

test('Recent Project removal changes only user-local history', async () => {
  let storedRecent: Array<{ root: string; name: string }> = [{ root: '/recent', name: 'Recent Demo' }];
  installWorkspaceFixture(invoke, async (command, args: any) => {
    if (command === 'load_application_user_state') return { recentProjects: storedRecent };
    if (command === 'set_recent_projects') {
      storedRecent = args.projects;
      return { recentProjects: storedRecent };
    }
    if (command === 'set_theme_preference') return {};
    if (command === 'authoring_workspace') throw { diagnostic: { code: 'E-PROJECT-NOT-FOUND', message: 'No project here' } };
    throw new Error(`Unexpected command: ${command}`);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  expect(await screen.findByText('Recent Demo')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Remove Recent Demo from Recent Projects' }));
  expect(screen.queryByText('Recent Demo')).toBeNull();
  expect(storedRecent).toEqual([]);
  expect(invoke).toHaveBeenCalledWith('set_recent_projects', { projects: [] });
  // Verify no disk mutation command was called
  expect(invoke.mock.calls.some(([cmd]) => cmd === 'save_data_file' || cmd === 'save_project_config_edit')).toBe(false);
});

test('late pre-save no-op preview cannot discard a new edit after Save resets revision', async () => {
  let resolveOld!: (value: unknown) => void;
  preview = () => new Promise(resolve => { resolveOld = resolve; });
  const input = await open();
  fireEvent.change(input, { target: { value: '10 ' } });
  commit(input);
  await waitFor(() => expect(resolveOld).toBeDefined());
  fireEvent.click(screen.getAllByRole('button', { name: 'Save', exact: true })[0]);
  await waitFor(() => expect(screen.getByRole('gridcell', { name: /record 1 weight: 10/ })).toBeTruthy());
  const nextInput = await edit('record 1 weight');
  fireEvent.change(nextInput, { target: { value: '20' } });
  commit(nextInput);
  await act(async () => resolveOld({ changed: false, candidateSource: 'weight: 10', validation }));
  expect(await screen.findByRole('gridcell', { name: /record 1 weight: 20/ })).toBeTruthy();
  expect(screen.getByText('1 dirty')).toBeTruthy();
});

test('project-wide diagnostics mark only the source file that owns the cell', async () => {
  openSnapshot.validation = { valid: false, diagnostics: [{ code: 'E-TABLE-INVALID-RECORD-VALUE', source: '/project/other/data.yaml', record_identity: 'record[0]', message: 'field `weight` is invalid' }] } as any;
  const input = await open();
  expect(input.closest('td')?.classList.contains('invalid')).toBe(false);
  fireEvent.click(screen.getByRole('button', { name: /PROBLEMS 1/ }));
  expect(screen.getByText('field `weight` is invalid')).toBeTruthy();
});

test('diagnostic belonging to the selected source marks its cell', async () => {
  openSnapshot.validation = { valid: false, diagnostics: [{ code: 'E-TABLE-INVALID-RECORD-VALUE', source: '/project/data.yaml', record_identity: 'record[0]', message: 'field `weight` is invalid' }] } as any;
  const input = await open();
  expect(input.closest('td')?.classList.contains('invalid')).toBe(true);
});

test('Explorer keeps source-root children visible and F2 opens source move', async () => {
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const tree = await screen.findByRole('tree', { name: 'Project source files' });
  const root = within(tree).getByRole('treeitem', { name: '. source root' });
  expect(root.hasAttribute('aria-expanded')).toBe(false);
  fireEvent.click(navigation().getByRole('button', { name: 'Collapse folders' }));
  expect(within(tree).getByRole('treeitem', { name: 'data.yaml', exact: true })).toBeTruthy();
  expect(navigation().queryByRole('button', { name: 'Tables' })).toBeNull();
  fireEvent.keyDown(within(tree).getByRole('treeitem', { name: 'data.yaml', exact: true }), { key: 'F2' });
  expect(await screen.findByRole('dialog')).toBeTruthy();
  expect(screen.getByText('Rename or move source')).toBeTruthy();
});

test('new diagnostics update the Problems count without moving the editor', async () => {
  preview = async () => ({ candidateSource: 'weight: invalid', changed: true,
    validation: { valid: false, diagnostics: [{ code: 'E-TABLE-INVALID-RECORD-VALUE', source: '/project/data.yaml', record_identity: 'record[0]', message: 'field `weight` is invalid' }] } });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const cell = await screen.findByRole('gridcell', { name: /record 1 weight:/ });
  const panel = document.querySelector('.problems-panel')!;
  expect(panel.classList.contains('collapsed')).toBe(true);
  fireEvent.keyDown(cell, { key: 'Enter' });
  const input = await screen.findByRole('textbox', { name: 'record 1 weight' });
  fireEvent.change(input, { target: { value: 'invalid' } });
  commit(input);
  await waitFor(() => expect(within(panel as HTMLElement).getByRole('button', { name: /PROBLEMS 1/ })).toBeTruthy());
  expect(within(panel as HTMLElement).queryByText('field `weight` is invalid')).toBeNull();
  fireEvent.click(within(panel as HTMLElement).getByRole('button', { name: /PROBLEMS 1/ }));
  expect(panel.classList.contains('open')).toBe(true);
  expect(within(panel as HTMLElement).getByText('field `weight` is invalid')).toBeTruthy();
});

test('Explorer Enter opens a source and moves keyboard focus into the editor', async () => {
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const file = await screen.findByRole('treeitem', { name: 'data.yaml', exact: true });
  fireEvent.keyDown(file, { key: 'Enter' });
  await waitFor(() => expect(document.activeElement?.closest('.editor-area')).not.toBeNull());
});

test('scalar edit stays local until commit, Escape cancels, and Tab moves to the next cell', async () => {
  openSnapshot = mutationSnapshot() as any;
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const idCell = await screen.findByRole('gridcell', { name: /record 1 id: 1/ });
  fireEvent.keyDown(idCell, { key: 'Enter' });
  const input = await screen.findByRole('textbox', { name: 'record 1 id' });
  fireEvent.change(input, { target: { value: '2' } });
  fireEvent.keyDown(input, { key: 'Escape' });
  expect(screen.getByRole('gridcell', { name: /record 1 id: 1/ })).toBeTruthy();
  expect(screen.queryByText('1 dirty')).toBeNull();
  fireEvent.keyDown(idCell, { key: 'Enter' });
  const reopened = await screen.findByRole('textbox', { name: 'record 1 id' });
  fireEvent.change(reopened, { target: { value: '2' } });
  fireEvent.keyDown(reopened, { key: 'Tab' });
  expect(screen.getByRole('gridcell', { name: /record 1 id: 2/ })).toBeTruthy();
  expect((document.activeElement as HTMLElement).getAttribute('aria-label')).toContain('record 1 weight');
});

test('complex values remain summarized in rows and keyboard opens the nested editor', async () => {
  const arrayShape = { name: 'values', typeName: 'int', modifier: 'array', shape: { kind: 'primitive', primitive: 'int' } };
  openSnapshot = { ...snapshot(), columns: [{ name: 'values', typeName: 'int[]', editable: true, keyField: false, shape: arrayShape, readOnlyReason: null }], rows: [
    { recordIndex: 0, cells: [{ field: 'values', text: '[1, 2]', value: { kind: 'sequence', sourceIdentity: true, items: [{ sourceIndex: 0, value: { kind: 'number', value: '1' } }, { sourceIndex: 1, value: { kind: 'number', value: '2' } }] }, editable: true, readOnlyReason: null }] },
    { recordIndex: 1, cells: [{ field: 'values', text: '[3]', value: { kind: 'sequence', sourceIdentity: true, items: [{ sourceIndex: 0, value: { kind: 'number', value: '3' } }] }, editable: true, readOnlyReason: null }] },
  ] } as any;
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const first = await screen.findByRole('gridcell', { name: /record 1 values:/ });
  expect(screen.getByRole('gridcell', { name: /record 2 values:/ })).toBeTruthy();
  expect(screen.queryByRole('textbox', { name: 'record 1 values item 1' })).toBeNull();
  fireEvent.keyDown(first, { key: 'Enter' });
  const nested = await screen.findByRole('textbox', { name: 'record 1 values item 1' });
  await waitFor(() => expect(document.activeElement).toBe(nested));
  fireEvent.keyDown(nested, { key: 'Escape' });
  expect(screen.queryByRole('textbox', { name: 'record 1 values item 1' })).toBeNull();
  expect(screen.getByRole('gridcell', { name: /record 2 values:/ })).toBeTruthy();
  expect(screen.queryByText('1 dirty')).toBeNull();
});

test('Array controls commit separately, nested typing commits once, and Close keeps committed edits', async () => {
  const shape = { name: 'numbers', typeName: 'int', modifier: 'array', shape: { kind: 'primitive', primitive: 'int' } };
  openSnapshot = { ...snapshot(), columns: [{ name: 'numbers', typeName: 'int[]', editable: true, keyField: false, shape, readOnlyReason: null }], rows: [
    { recordIndex: 0, cells: [{ field: 'numbers', text: '[1]', value: { kind: 'sequence', sourceIdentity: true, items: [{ sourceIndex: 0, value: { kind: 'number', value: '1' } }] }, editable: true, readOnlyReason: null }] },
  ] } as any;
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const cell = await screen.findByRole('gridcell', { name: /^record 1 numbers:/ });
  fireEvent.keyDown(cell, { key: 'Enter' });
  fireEvent.click(await screen.findByRole('button', { name: 'Add record 1 numbers item' }));
  expect(screen.queryByRole('button', { name: 'Apply to buffer' })).toBeNull();
  await waitFor(() => expect(screen.getByRole('gridcell', { name: /^record 1 numbers: \[1, null\]/ })).toBeTruthy());
  const item = await screen.findByRole('textbox', { name: 'record 1 numbers item 2' });
  fireEvent.change(item, { target: { value: '42' } });
  expect(screen.getByRole('gridcell', { name: /^record 1 numbers: \[1, null\]/ })).toBeTruthy();
  fireEvent.blur(item);
  await waitFor(() => expect(screen.getByRole('gridcell', { name: /^record 1 numbers: \[1, 42\]/ })).toBeTruthy());
  fireEvent.click(screen.getByRole('button', { name: 'Close', exact: true }));
  expect(screen.getByRole('gridcell', { name: /^record 1 numbers: \[1, 42\]/ })).toBeTruthy();
  await waitFor(() => expect(document.activeElement).toBe(screen.getByRole('gridcell', { name: /^record 1 numbers: \[1, 42\]/ })));
  await dataAction('Undo');
  expect(screen.getByRole('gridcell', { name: /^record 1 numbers: \[1, null\]/ })).toBeTruthy();
  await dataAction('Undo');
  expect(screen.getByRole('gridcell', { name: /^record 1 numbers: \[1\]/ })).toBeTruthy();
  await dataAction('Redo');
  expect(screen.getByRole('gridcell', { name: /^record 1 numbers: \[1, null\]/ })).toBeTruthy();
}, APP_INTEGRATION_TEST_TIMEOUT_MS);

test('clicking outside a complex editor closes it after committing pending nested text', async () => {
  const shape = { name: 'numbers', typeName: 'int', modifier: 'array', shape: { kind: 'primitive', primitive: 'int' } };
  openSnapshot = { ...snapshot(), columns: [{ name: 'numbers', typeName: 'int[]', editable: true, keyField: false, shape, readOnlyReason: null }], rows: [
    { recordIndex: 0, cells: [{ field: 'numbers', text: '[1]', value: { kind: 'sequence', sourceIdentity: true, items: [{ sourceIndex: 0, value: { kind: 'number', value: '1' } }] }, editable: true, readOnlyReason: null }] },
  ] } as any;
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  fireEvent.keyDown(await screen.findByRole('gridcell', { name: /^record 1 numbers:/ }), { key: 'Enter' });
  const input = await screen.findByRole('textbox', { name: 'record 1 numbers item 1' });
  fireEvent.change(input, { target: { value: '42' } });
  fireEvent.blur(input);
  fireEvent.click(document.body);
  await waitFor(() => expect(screen.queryByRole('textbox', { name: 'record 1 numbers item 1' })).toBeNull());
  expect(screen.getByRole('gridcell', { name: /^record 1 numbers: \[42\]/ })).toBeTruthy();
});

test('Array move and remove are independent reversible source operations', async () => {
  const shape = { name: 'numbers', typeName: 'int', modifier: 'array', shape: { kind: 'primitive', primitive: 'int' } };
  openSnapshot = { ...snapshot(), columns: [{ name: 'numbers', typeName: 'int[]', editable: true, keyField: false, shape, readOnlyReason: null }], rows: [
    { recordIndex: 0, cells: [{ field: 'numbers', text: '[1, 2]', value: { kind: 'sequence', sourceIdentity: true, items: [
      { sourceIndex: 0, value: { kind: 'number', value: '1' } }, { sourceIndex: 1, value: { kind: 'number', value: '2' } },
    ] }, editable: true, readOnlyReason: null }] },
  ] } as any;
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  fireEvent.keyDown(await screen.findByRole('gridcell', { name: /^record 1 numbers:/ }), { key: 'Enter' });
  fireEvent.click(await screen.findByRole('button', { name: 'Actions for record 1 numbers item 2' }));
  fireEvent.click(await screen.findByRole('menuitem', { name: 'Move record 1 numbers item 2 up' }));
  await waitFor(() => expect(screen.getByRole('gridcell', { name: /^record 1 numbers: \[2, 1\]/ })).toBeTruthy());
  fireEvent.click(screen.getByRole('button', { name: 'Actions for record 1 numbers item 1' }));
  fireEvent.click(await screen.findByRole('menuitem', { name: 'Remove record 1 numbers item 1' }));
  await waitFor(() => expect(screen.getByRole('gridcell', { name: /^record 1 numbers: \[1\]/ })).toBeTruthy());
  fireEvent.click(screen.getByRole('button', { name: 'Close', exact: true }));
  await dataAction('Undo');
  expect(screen.getByRole('gridcell', { name: /^record 1 numbers: \[2, 1\]/ })).toBeTruthy();
  await dataAction('Undo');
  expect(screen.getByRole('gridcell', { name: /^record 1 numbers: \[1, 2\]/ })).toBeTruthy();
  await dataAction('Redo');
  expect(screen.getByRole('gridcell', { name: /^record 1 numbers: \[2, 1\]/ })).toBeTruthy();
}, APP_INTEGRATION_TEST_TIMEOUT_MS);

test('Array pointer grab commits once on release and ignores a click without movement', async () => {
  const shape = { name: 'numbers', typeName: 'int', modifier: 'array', shape: { kind: 'primitive', primitive: 'int' } };
  openSnapshot = { ...snapshot(), columns: [{ name: 'numbers', typeName: 'int[]', editable: true, keyField: false, shape, readOnlyReason: null }], rows: [
    { recordIndex: 0, cells: [{ field: 'numbers', text: '[1, 2, 3]', value: { kind: 'sequence', sourceIdentity: true, items: [
      { sourceIndex: 0, value: { kind: 'number', value: '1' } }, { sourceIndex: 1, value: { kind: 'number', value: '2' } }, { sourceIndex: 2, value: { kind: 'number', value: '3' } },
    ] }, editable: true, readOnlyReason: null }] },
  ] } as any;
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  fireEvent.keyDown(await screen.findByRole('gridcell', { name: /^record 1 numbers:/ }), { key: 'Enter' });
  const first = screen.getByRole('button', { name: 'Drag record 1 numbers item 1 to reorder' }).closest('.array-value-item') as HTMLElement;
  vi.spyOn(first, 'getBoundingClientRect').mockReturnValue({ top: 0, height: 20 } as DOMRect);
  const originalElementFromPoint = document.elementFromPoint;
  Object.defineProperty(document, 'elementFromPoint', { configurable: true, value: vi.fn(() => first) });
  const last = screen.getByRole('button', { name: 'Drag record 1 numbers item 3 to reorder' });
  fireEvent.pointerDown(last, { button: 0, clientX: 1, clientY: 50 });
  fireEvent.pointerUp(window, { clientX: 1, clientY: 50 });
  expect(screen.getByRole('gridcell', { name: /^record 1 numbers: \[1, 2, 3\]/ })).toBeTruthy();
  fireEvent.pointerDown(last, { button: 0, clientX: 1, clientY: 50 });
  fireEvent.pointerMove(window, { clientX: 1, clientY: 1 });
  expect(document.elementFromPoint).toHaveBeenCalled();
  fireEvent.pointerUp(window, { clientX: 1, clientY: 1 });
  await waitFor(() => expect(screen.getByRole('gridcell', { name: /^record 1 numbers: \[3, 1, 2\]/ })).toBeTruthy());
  Object.defineProperty(document, 'elementFromPoint', { configurable: true, value: originalElementFromPoint });
});

test('Row context move and positional insert preserve source occurrence order in the mutation request', async () => {
  openSnapshot = mutationSnapshot([
    { recordIndex: 0, cells: [{ field: 'id', text: '1', editable: true }, { field: 'weight', text: '10', editable: true }, { field: 'note', text: 'A', editable: true }] },
    { recordIndex: 1, cells: [{ field: 'id', text: '2', editable: true }, { field: 'weight', text: '20', editable: true }, { field: 'note', text: 'B', editable: true }] },
    { recordIndex: 2, cells: [{ field: 'id', text: '3', editable: true }, { field: 'weight', text: '30', editable: true }, { field: 'note', text: 'C', editable: true }] },
  ]) as any;
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  fireEvent.click(await screen.findByRole('button', { name: 'Actions for record 3' }));
  fireEvent.click(await screen.findByRole('menuitem', { name: 'Move row up' }));
  await waitFor(() => expect(invoke.mock.calls.some(([command, args]) => command === 'preview_data_file' && args.recordOrder?.[1]?.index === 2)).toBe(true));
  await waitFor(() => expect(document.activeElement?.getAttribute('data-row-grab')).toBe('existing:2'));
  const notes = () => within(document.querySelector('.record-grid tbody') as HTMLElement).getAllByRole('gridcell', { name: / note:/ }).map(cell => cell.getAttribute('aria-label'));
  expect(notes()).toEqual(['record 1 note: "A"', 'record 3 note: "C"', 'record 2 note: "B"']);
  fireEvent.click(screen.getByRole('button', { name: 'Actions for record 2' }));
  fireEvent.click(await screen.findByRole('menuitem', { name: 'Insert row above' }));
  await waitFor(() => expect(screen.getByRole('gridcell', { name: /new record id:/ })).toBeTruthy());
  expect(notes()[1]).toContain('new record');
});

test('Row pointer drag uses the same source occurrence operation and keyboard context remains reachable', async () => {
  openSnapshot = mutationSnapshot([
    { recordIndex: 0, cells: [{ field: 'id', text: '1', editable: true }, { field: 'weight', text: '10', editable: true }, { field: 'note', text: 'A', editable: true }] },
    { recordIndex: 1, cells: [{ field: 'id', text: '2', editable: true }, { field: 'weight', text: '20', editable: true }, { field: 'note', text: 'B', editable: true }] },
    { recordIndex: 2, cells: [{ field: 'id', text: '3', editable: true }, { field: 'weight', text: '30', editable: true }, { field: 'note', text: 'C', editable: true }] },
  ]) as any;
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const lastGrab = await screen.findByRole('button', { name: 'Drag row 3 to reorder' });
  const firstRow = screen.getByRole('button', { name: 'Drag row 1 to reorder' }).closest('tr') as HTMLElement;
  const secondRow = screen.getByRole('button', { name: 'Drag row 2 to reorder' }).closest('tr') as HTMLElement;
  const lastRow = lastGrab.closest('tr') as HTMLElement;
  vi.spyOn(firstRow, 'getBoundingClientRect').mockReturnValue({ top: 0, height: 32 } as DOMRect);
  vi.spyOn(secondRow, 'getBoundingClientRect').mockReturnValue({ top: 32, height: 32 } as DOMRect);
  vi.spyOn(lastRow, 'getBoundingClientRect').mockReturnValue({ top: 64, height: 32 } as DOMRect);
  const scroll = firstRow.closest<HTMLElement>('.grid-scroll')!;
  vi.spyOn(scroll, 'getBoundingClientRect').mockReturnValue({ left: 0, top: 0, right: 500, bottom: 400 } as DOMRect);
  Object.defineProperties(scroll, { clientWidth: { configurable: true, value: 500 }, clientHeight: { configurable: true, value: 400 } });
  const commandsBeforeCancel = invoke.mock.calls.length;
  fireEvent.pointerDown(lastGrab, { button: 0, pointerId: 1, clientX: 50, clientY: 80 });
  fireEvent.pointerMove(window, { pointerId: 1, clientX: 50, clientY: 31 });
  expect(firstRow.closest('table')?.dataset.dragDestination).toBe('0');
  expect(firstRow.style.transform).toBe('translateY(32px)');
  expect(secondRow.style.transform).toBe('translateY(32px)');
  expect(document.querySelector('.grid-drag-ghost')).toBeTruthy();
  expect(invoke.mock.calls.length).toBe(commandsBeforeCancel);
  fireEvent.pointerCancel(window, { pointerId: 1 });
  expect(document.querySelector('.grid-drag-overlay')).toBeNull();
  expect(firstRow.style.transform).toBe('');
  expect(invoke.mock.calls.length).toBe(commandsBeforeCancel);
  fireEvent.pointerDown(lastGrab, { button: 0, pointerId: 1, clientX: 50, clientY: 80 });
  fireEvent.pointerMove(window, { pointerId: 1, clientX: 50, clientY: 1 });
  fireEvent.pointerUp(window, { pointerId: 1, clientX: 50, clientY: 1 });
  await waitFor(() => expect(invoke.mock.calls.some(([command, args]) => command === 'preview_data_file' && args.recordOrder?.[0]?.index === 2)).toBe(true));
  await waitFor(() => expect(document.activeElement?.getAttribute('data-row-grab')).toBe('existing:2'));
  fireEvent.keyDown(window, { ctrlKey: true, key: 'z' });
  await waitFor(() => expect([...document.querySelectorAll('.record-grid .cell-wrap[data-cell$=":note"]')].map(cell => cell.getAttribute('aria-label'))).toEqual(['record 1 note: "A"', 'record 2 note: "B"', 'record 3 note: "C"']));
  fireEvent.keyDown(window, { ctrlKey: true, key: 'y' });
  await waitFor(() => expect(document.querySelector('.record-grid .cell-wrap[data-cell$=":note"]')?.getAttribute('aria-label')).toBe('record 3 note: "C"'));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Drag row 1 to reorder' }), { key: 'F10', shiftKey: true });
  expect(await screen.findByRole('menuitem', { name: 'Insert row above' })).toBeTruthy();
});

test('Cmd+S in a pending nested scalar commits its text before current Table Save', async () => {
  const shape = { name: 'numbers', typeName: 'int', modifier: 'array', shape: { kind: 'primitive', primitive: 'int' } };
  openSnapshot = { ...snapshot(), columns: [{ name: 'numbers', typeName: 'int[]', editable: true, keyField: false, shape, readOnlyReason: null }], rows: [
    { recordIndex: 0, cells: [{ field: 'numbers', text: '[1]', value: { kind: 'sequence', sourceIdentity: true, items: [{ sourceIndex: 0, value: { kind: 'number', value: '1' } }] }, editable: true, readOnlyReason: null }] },
  ] } as any;
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const cell = await screen.findByRole('gridcell', { name: /^record 1 numbers:/ });
  fireEvent.keyDown(cell, { key: 'Enter' });
  const input = await screen.findByRole('textbox', { name: 'record 1 numbers item 1' });
  fireEvent.change(input, { target: { value: '42' } });
  fireEvent.keyDown(input, { key: 's', metaKey: true });
  await waitFor(() => expect(invoke.mock.calls.find(([command]) => command === 'save_current_table_context')?.[1].request.recordDraft.mutation.edits[0].value.items[0].value).toEqual({ kind: 'number', value: '42' }));
});

test('Flags operations keep unknown members and Escape cancels only pending nested typing', async () => {
  const shape = { name: 'flags', typeName: 'Tags', modifier: 'required', shape: { kind: 'flags', name: 'Tags', members: ['None', 'A', 'B'] } };
  openSnapshot = { ...snapshot(), columns: [{ name: 'flags', typeName: 'Tags', editable: true, keyField: false, shape, readOnlyReason: null }], rows: [
    { recordIndex: 0, cells: [{ field: 'flags', text: '[A, Mystery]', value: { kind: 'sequence', sourceIdentity: true, items: [
      { sourceIndex: 0, value: { kind: 'string', value: 'A' } }, { sourceIndex: 1, value: { kind: 'string', value: 'Mystery' } },
    ] }, editable: true, readOnlyReason: null }] },
  ] } as any;
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const cell = await screen.findByRole('gridcell', { name: /^record 1 flags:/ });
  fireEvent.keyDown(cell, { key: 'Enter' });
  fireEvent.click(await screen.findByRole('checkbox', { name: 'record 1 flags B' }));
  await waitFor(() => expect(screen.getByRole('gridcell', { name: /Mystery.*B|B.*Mystery/ })).toBeTruthy());
  expect(screen.getByText(/Unrecognized flag value:.*Mystery/)).toBeTruthy();
  fireEvent.click(screen.getByRole('checkbox', { name: 'record 1 flags A' }));
  await waitFor(() => expect(screen.getByRole('gridcell', { name: /Mystery.*B|B.*Mystery/ })).toBeTruthy());
  expect((screen.getByRole('checkbox', { name: 'record 1 flags A' }) as HTMLInputElement).checked).toBe(false);
  fireEvent.keyDown(screen.getByRole('checkbox', { name: 'record 1 flags B' }), { key: 'Escape' });
  expect(screen.getByRole('gridcell', { name: /Mystery.*B|B.*Mystery/ })).toBeTruthy();
  await dataAction('Undo');
  expect(screen.getByRole('gridcell', { name: /A.*Mystery.*B/ })).toBeTruthy();
  await dataAction('Undo');
  expect(screen.getByRole('gridcell', { name: /A.*Mystery/ })).toBeTruthy();
  await dataAction('Redo');
  expect(screen.getByRole('gridcell', { name: /A.*Mystery.*B/ })).toBeTruthy();
}, APP_INTEGRATION_TEST_TIMEOUT_MS);

test('Problems opens nested Custom control; Escape cancels only its pending text and preserves unknown members', async () => {
  const nested = { name: 'detail', typeName: 'Detail', modifier: 'required', shape: { kind: 'custom', name: 'Detail', fields: [
    { name: 'label', typeName: 'string', modifier: 'required', shape: { kind: 'primitive', primitive: 'string' } },
  ] } };
  const shape = { name: 'reward', typeName: 'Reward', modifier: 'required', shape: { kind: 'custom', name: 'Reward', fields: [nested] } };
  const diagnostic = { code: 'E-TABLE-INVALID-RECORD-VALUE', source: '/project/data.yaml', record_identity: 'record[0]',
    message: 'field `reward` detail label is invalid', value_path: '/detail/label' };
  openSnapshot = { ...snapshot(), columns: [
    { name: 'id', typeName: 'int', editable: true, keyField: false, shape: { name: 'id', typeName: 'int', modifier: 'required', shape: { kind: 'primitive', primitive: 'int' } }, readOnlyReason: null },
    { name: 'reward', typeName: 'Reward', editable: true, keyField: false, shape, readOnlyReason: null },
  ], rows: [
    { recordIndex: 0, cells: [{ field: 'id', text: '1', value: { kind: 'number', value: '1' }, editable: true, readOnlyReason: null },
    { field: 'reward', text: '{detail: {label: bad}, legacy: keep}', value: { kind: 'mapping', entries: [
      { name: 'detail', value: { kind: 'mapping', entries: [{ name: 'label', value: { kind: 'string', value: 'bad' } }] } },
      { name: 'legacy', value: { kind: 'string', value: 'keep' } },
    ] }, editable: true, readOnlyReason: null }] },
  ], validation: { valid: false, diagnostics: [diagnostic] } } as any;
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  fireEvent.keyDown(await screen.findByRole('gridcell', { name: /^record 1 reward:/ }), { key: 'Enter' });
  await screen.findByRole('textbox', { name: 'label' });
  fireEvent.click(await screen.findByRole('button', { name: /PROBLEMS 1/ }));
  await waitFor(() => expect(screen.queryByRole('textbox', { name: 'label' })).toBeNull());
  fireEvent.click(screen.getByRole('button', { name: /field `reward` detail label is invalid/ }));
  const input = await screen.findByRole('textbox', { name: 'label' });
  await waitFor(() => expect(document.activeElement).toBe(input));
  expect(screen.getByText('Unknown source members')).toBeTruthy();
  fireEvent.change(input, { target: { value: 'discard me' } });
  fireEvent.keyDown(input, { key: 'Escape' });
  expect((input as HTMLInputElement).value).toBe('bad');
  expect(screen.getByRole('textbox', { name: 'label' })).toBeTruthy();
  fireEvent.change(input, { target: { value: 'fixed' } });
  fireEvent.keyDown(input, { key: 'Enter' });
  await waitFor(() => expect(screen.getByRole('gridcell', { name: /fixed.*legacy.*keep/ })).toBeTruthy());
  fireEvent.click(screen.getByRole('button', { name: 'Close', exact: true }));
  expect(screen.getByRole('gridcell', { name: /fixed.*legacy.*keep/ })).toBeTruthy();
  await dataAction('Undo');
  expect(screen.getByRole('gridcell', { name: /bad.*legacy.*keep/ })).toBeTruthy();
}, APP_INTEGRATION_TEST_TIMEOUT_MS);

test('Added Row uses the same Array operation history as an existing record', async () => {
  const shape = { name: 'numbers', typeName: 'int', modifier: 'array', shape: { kind: 'primitive', primitive: 'int' } };
  openSnapshot = { ...snapshot(), columns: [{ name: 'numbers', typeName: 'int[]', editable: true, keyField: false, shape, readOnlyReason: null }], rows: [
    { recordIndex: 0, cells: [{ field: 'numbers', text: '[]', value: { kind: 'sequence', sourceIdentity: true, items: [] }, editable: true, readOnlyReason: null }] },
  ], addRow: { supported: true, reason: null } } as any;
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  await screen.findByRole('gridcell', { name: /^record 1 numbers:/ });
  fireEvent.click(screen.getByRole('button', { name: /Add Row/ }));
  const added = await screen.findByRole('gridcell', { name: /^new record numbers:/ });
  fireEvent.keyDown(added, { key: 'Enter' });
  fireEvent.click(await screen.findByRole('button', { name: 'Make empty array for new record numbers' }));
  await waitFor(() => expect(screen.getByRole('gridcell', { name: /^new record numbers: \[\]/ })).toBeTruthy());
  fireEvent.click(screen.getByRole('button', { name: 'Close', exact: true }));
  await dataAction('Undo');
  expect(screen.getByRole('gridcell', { name: /^new record numbers: null/ })).toBeTruthy();
  await dataAction('Redo');
  expect(screen.getByRole('gridcell', { name: /^new record numbers: \[\]/ })).toBeTruthy();
}, APP_INTEGRATION_TEST_TIMEOUT_MS);

test('nullable Set null is a direct source operation and Undo restores the value', async () => {
  const shape = { name: 'alias', typeName: 'string', modifier: 'nullable', shape: { kind: 'primitive', primitive: 'string' } };
  openSnapshot = { ...snapshot(), columns: [{ name: 'alias', typeName: 'string', editable: true, keyField: false, shape, readOnlyReason: null }], rows: [
    { recordIndex: 0, cells: [{ field: 'alias', text: 'kept', value: { kind: 'string', value: 'kept' }, editable: true, readOnlyReason: null }] },
  ] } as any;
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const cell = await screen.findByRole('gridcell', { name: /^record 1 alias:/ });
  fireEvent.keyDown(cell, { key: 'Enter' });
  fireEvent.click(await screen.findByRole('button', { name: 'Set record 1 alias to null' }));
  await waitFor(() => expect(screen.getByRole('gridcell', { name: /^record 1 alias: null/ })).toBeTruthy());
  await dataAction('Undo');
  expect(screen.getByRole('gridcell', { name: /^record 1 alias: "kept"/ })).toBeTruthy();
});

test('unknown Enum member stays intact until selection commits one Undo operation', async () => {
  const shape = { name: 'rarity', typeName: 'Rarity', modifier: 'required', shape: { kind: 'enum', name: 'Rarity', underlying: 'int', members: ['Common', 'Rare'] } };
  openSnapshot = { ...snapshot(), columns: [{ name: 'rarity', typeName: 'Rarity', editable: true, keyField: false, shape, readOnlyReason: null }], rows: [
    { recordIndex: 0, cells: [{ field: 'rarity', text: 'Legacy', value: { kind: 'string', value: 'Legacy' }, editable: true, readOnlyReason: null }] },
  ] } as any;
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const cell = await screen.findByRole('gridcell', { name: /^record 1 rarity: "Legacy"/ });
  fireEvent.keyDown(cell, { key: 'Enter' });
  expect(screen.getAllByText('Legacy (unknown member)').length).toBeGreaterThan(0);
  fireEvent.mouseDown(screen.getByRole('combobox', { name: 'record 1 rarity' }));
  fireEvent.click((await screen.findAllByText('Rare', { selector: '.ant-select-item-option-content' })).at(-1)!);
  await waitFor(() => expect(screen.getByRole('gridcell', { name: /^record 1 rarity: "Rare"/ })).toBeTruthy());
  await dataAction('Undo');
  expect(screen.getByRole('gridcell', { name: /^record 1 rarity: "Legacy"/ })).toBeTruthy();
});

test('filter options stay out of the default grid and retain their draft when reopened', async () => {
  await open();
  expect(screen.queryByRole('textbox', { name: 'Data filter value' })).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Filter & sort' }));
  const value = screen.getByRole('textbox', { name: 'Data filter value' }) as HTMLInputElement;
  expect(value.closest('.authoring-tools-shell')).toBeTruthy();
  expect(value.closest('.grid-scroll')).toBeNull();
  fireEvent.change(value, { target: { value: 'rare' } });
  fireEvent.click(screen.getByRole('button', { name: 'Filter & sort' }));
  expect(screen.queryByRole('textbox', { name: 'Data filter value' })).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Filter & sort' }));
  expect((await screen.findByRole('textbox', { name: 'Data filter value' }) as HTMLInputElement).value).toBe('rare');
  fireEvent.keyDown(screen.getByRole('textbox', { name: 'Data filter value' }), { key: 'Escape' });
  expect(screen.queryByRole('textbox', { name: 'Data filter value' })).toBeNull();
  expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Filter & sort' }));
});

test('search runs from its input without a second toolbar button', async () => {
  const normal = invoke.getMockImplementation()!;
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'query_data_file') return {
      orderedRecordIndices: [0], totalCount: 1, displayedCount: 1, query: args.request.query,
    };
    return normal(command, args);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const search = await screen.findByRole('searchbox', { name: 'Data search' });
  fireEvent.change(search, { target: { value: '10' } });
  fireEvent.keyDown(search, { key: 'Enter', code: 'Enter' });
  await waitFor(() => expect(invoke.mock.calls.find(([command]) => command === 'query_data_file')?.[1].request.query.search).toBe('10'));
  const searchControl = document.querySelector<HTMLElement>('.authoring-toolbar > .ant-input-search');
  expect(searchControl).toBeTruthy();
  const searchButton = within(searchControl!).getByRole('button', { name: 'Search data' });
  await waitFor(() => expect(searchButton.classList.contains('ant-btn-loading')).toBe(false));
  fireEvent.click(searchButton);
  await waitFor(() => expect(invoke.mock.calls.filter(([command]) => command === 'query_data_file')).toHaveLength(2));
  expect(document.querySelectorAll('.authoring-toolbar:not(.authoring-options) > .ant-btn')).toHaveLength(1);
}, APP_INTEGRATION_TEST_TIMEOUT_MS);

test('Data query draft survives visiting a Project area and returning to the same source', async () => {
  await open();
  fireEvent.click(screen.getByRole('button', { name: 'Filter & sort' }));
  fireEvent.change(screen.getByRole('textbox', { name: 'Data filter value' }), { target: { value: 'rare' } });
  openProjectCommands();
  fireEvent.click(screen.getByRole('menuitem', { name: 'Build & Publish' }));
  expect(await screen.findByRole('heading', { name: 'Build / Publish' })).toBeTruthy();
  openSourceFiles();
  fireEvent.click(navigation().getByRole('treeitem', { name: 'data.yaml', exact: true }));
  expect((await screen.findByRole('textbox', { name: 'Data filter value' }) as HTMLInputElement).value).toBe('rare');
});

test('Data query drafts remain file-local when switching between Data documents', async () => {
  const multiFileWorkspace = { ...workspace, files: [
    { ...workspace.files[0], table: 'item', typeName: null },
    { path: 'other.yaml', sourceRoot: '.', kind: 'data', table: 'item', typeName: null },
  ] };
  const normalInvoke = invoke.getMockImplementation()!;
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'authoring_workspace') return multiFileWorkspace;
    if (command === 'open_data_file') return { ...structuredClone(openSnapshot), path: args.relativePath };
    return normalInvoke(command, args);
  });
  await open();
  fireEvent.click(screen.getByRole('button', { name: 'Filter & sort' }));
  fireEvent.change(screen.getByRole('textbox', { name: 'Data filter value' }), { target: { value: 'rare' } });
  fireEvent.click(navigation().getByRole('treeitem', { name: 'other.yaml', exact: true }));
  expect(await screen.findByRole('gridcell', { name: /record 1 weight:/ })).toBeTruthy();
  expect(screen.queryByRole('textbox', { name: 'Data filter value' })).toBeNull();
  fireEvent.click(navigation().getByRole('treeitem', { name: 'data.yaml', exact: true }));
  expect((await screen.findByRole('textbox', { name: 'Data filter value' }) as HTMLInputElement).value).toBe('rare');
});

test('Table context opens Data creation with its Table already selected', async () => {
  const contextWorkspace = { ...workspace, files: [{ ...workspace.files[0], table: 'item', typeName: null }] };
  const normalInvoke = invoke.getMockImplementation()!;
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'authoring_workspace') return contextWorkspace;
    if (command === 'creation_context') return { roots: [{ index: 0, label: '/project', folders: [''] }], choices: { fieldTypes: ['int'], tables: ['item'], valueObjectUnderlyings: ['int'], enumUnderlyings: ['int'] } };
    return normalInvoke(command, args);
  });
  await open();
  await dataAction('New data file');
  expect((await screen.findByRole('combobox', { name: 'Existing Table' })).closest('.ant-select')?.textContent).toContain('item');
  expect(screen.getByRole('textbox', { name: 'New source filename' })).toBeTruthy();
  expect(screen.getByLabelText('New Data')).toBeTruthy();
});

test('Table Overview follows the selected logical Table rather than the previously active file', async () => {
  const multiTableWorkspace = { ...workspace, files: [
    { ...workspace.files[0], table: 'item', typeName: null },
    { path: 'enemy-data.yaml', sourceRoot: '.', kind: 'data', table: 'enemy', typeName: null },
  ] };
  const normalInvoke = invoke.getMockImplementation()!;
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'authoring_workspace') return multiTableWorkspace;
    if (command === 'table_overview') return { status: 'complete', table: args.request.table, columns: [], rows: [], totalCount: 0, selectedCount: 0, displayedCount: 0, configContentIdentity: 'config', sources: [], selection: { profile: null, includeTags: [], excludeTags: [], available: true }, diagnostics: [] };
    return normalInvoke(command, args);
  });
  await open();
  fireEvent.click(navigation().getByRole('treeitem', { name: 'enemy-data.yaml', exact: true }));
  await dataAction('Table Overview');
  await waitFor(() => expect(invoke.mock.calls.some(([command, args]) => command === 'table_overview' && args.request.table === 'enemy')).toBe(true));
});

test('empty Project offers a contextual Folder action with a folder-safe name', async () => {
  const normalInvoke = invoke.getMockImplementation()!;
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'authoring_workspace') return { ...workspace, files: [] };
    if (command === 'creation_context') return { roots: [{ index: 0, label: '/project', folders: [''] }], choices: { fieldTypes: ['int'], tables: [], valueObjectUnderlyings: ['int'], enumUnderlyings: ['int'] } };
    return normalInvoke(command, args);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  expect(await screen.findByRole('heading', { name: 'Start with a Table' })).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Create Folder' }));
  expect((await screen.findByRole('textbox', { name: 'New folder name' }) as HTMLInputElement).value).toBe('new-folder');
});

test('Ant Design unsaved dialog Cancel preserves edits and does not reload', async () => {
  const input = await open();
  fireEvent.change(input, { target: { value: '20' } });
  commit(input);
  reloadProject();
  expect(await screen.findByRole('dialog')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Cancel', exact: true }));
  expect(await screen.findByRole('gridcell', { name: /record 1 weight: 20/ })).toBeTruthy();
  expect(invoke.mock.calls.filter(([command]) => command === 'open_workspace')).toHaveLength(1);
});

test('dirty Build invokes only saved-source Build and preserves exact 64-bit input', async () => {
  const input = await open();
  fireEvent.change(input, { target: { value: '18446744073709551615' } });
  openProjectCommands();
  fireEvent.click(screen.getByRole('menuitem', { name: 'Build', exact: true }));
  await waitFor(() => expect(invoke.mock.calls.some(([command]) => command === 'build')).toBe(true));
  expect(invoke.mock.calls.some(([command]) => command === 'save_data_file')).toBe(false);
  expect((input as HTMLInputElement).value).toBe('18446744073709551615');
});

test('current preview updates the Diff after old snapshot responses are rejected', async () => {
  const input = await open();
  fireEvent.change(input, { target: { value: '20' } });
  commit(input);
  await waitFor(() => expect(invoke.mock.calls.some(([command]) => command === 'preview_data_file')).toBe(true));
  fireEvent.click(screen.getByRole('tab', { name: 'Diff', exact: true }));
  expect(await screen.findByText('weight: 20')).toBeTruthy();
});

test('Ant Design Save All dialog preserves input when source commit fails', async () => {
  const normalInvoke = invoke.getMockImplementation()!;
  installWorkspaceFixture(invoke, async (command, args) => command === 'save_data_file'
    ? { status: 'failure', snapshot: null, current: null, diagnostic: { code: 'E-IO', message: 'Cannot write source' } }
    : normalInvoke(command, args));
  const input = await open();
  fireEvent.change(input, { target: { value: '20' } });
  commit(input);
  reloadProject();
  fireEvent.click(await screen.findByRole('button', { name: 'Save All', exact: true }));
  await waitFor(() => expect(screen.getByText('Save failed.')).toBeTruthy());
  expect(screen.getByRole('dialog')).toBeTruthy();
  expect(await screen.findByRole('gridcell', { name: /record 1 weight: 20/ })).toBeTruthy();
  expect(invoke.mock.calls.filter(([command]) => command === 'open_workspace')).toHaveLength(1);
});

test('creation refresh selects the new source without discarding an existing dirty buffer', async () => {
  const normalInvoke = invoke.getMockImplementation()!;
  let created = false;
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'creation_context') return { roots: [{ index: 0, label: '/project', folders: [''] }], choices: { fieldTypes: ['int'], tables: ['item'], valueObjectUnderlyings: ['int'], enumUnderlyings: ['int'] } };
    if (command === 'default_creation_proposal') return { identity: 'weapon', request: { sourceRoot: '.', destination: args.intent.destination, artifact: { category: 'table', table: 'weapon', inlineRecords: true, fields: [{ key: 0, name: 'id', type: 'int' }], primaryKey: { fields: ['id'] } } } };
    if (command === 'create_source') { created = true; return { status: 'success', path: 'weapon.yaml', folder: false, diagnostic: null }; }
    if (command === 'authoring_workspace' && created) return { ...workspace, files: [...workspace.files, { path: 'weapon.yaml', sourceRoot: '.', kind: 'schema' }] };
    return normalInvoke(command, args);
  });
  const input = await open(); fireEvent.change(input, { target: { value: '20' } }); commit(input);
  openSourceFiles();
  fireEvent.click(screen.getByRole('button', { name: 'New source artifact' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Table' }));
  const filename = await screen.findByRole('textbox', { name: 'New source filename' });
  fireEvent.change(filename, { target: { value: 'weapon.yaml' } });
  fireEvent.keyDown(filename, { key: 'Enter' });
  await waitFor(() => expect(screen.getByRole('treeitem', { name: 'weapon.yaml', exact: true }).getAttribute('aria-selected')).toBe('true'));
  expect(invoke.mock.calls.some(([command, args]) => command === 'create_source' && args.request.artifact.table === 'weapon')).toBe(true);
  fireEvent.click(screen.getByRole('treeitem', { name: 'data.yaml, unsaved changes', exact: true }));
  expect(await screen.findByRole('gridcell', { name: /record 1 weight: 20/ })).toBeTruthy();
  expect(invoke.mock.calls.some(([command]) => command === 'save_data_file')).toBe(false);
}, APP_INTEGRATION_TEST_TIMEOUT_MS);

test('Explorer Escape cancels inline creation without a source mutation', async () => {
  const normalInvoke = invoke.getMockImplementation()!;
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'creation_context') return { roots: [{ index: 0, label: '.', folders: [''] }], choices: { fieldTypes: ['int'], tables: [], valueObjectUnderlyings: ['int'], enumUnderlyings: ['int'] } };
    if (command === 'default_creation_proposal') return { identity: 'New', request: { sourceRoot: '.', destination: args.intent.destination, artifact: { category: 'value_object', name: 'New', underlying: 'int', conversions: {} } } };
    return normalInvoke(command, args);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  await screen.findByRole('complementary', { name: 'Explorer' });
  fireEvent.click(screen.getByRole('button', { name: 'New source artifact' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Value Object' }));
  const filename = await screen.findByRole('textbox', { name: 'New source filename' });
  fireEvent.keyDown(filename, { key: 'Escape' });
  expect(screen.queryByRole('textbox', { name: 'New source filename' })).toBeNull();
  expect(invoke.mock.calls.some(([command]) => command === 'create_source')).toBe(false);
});

test('Existing primary key direct edit uses the ordinary cell mutation lifecycle', async () => {
  openSnapshot = mutationSnapshot();
  let previewArgs: any;
  preview = async (args) => {
    previewArgs = args;
    return { candidateSource: 'id: 2', changed: true, validation };
  };
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const id = await edit('record 1 id') as HTMLInputElement;
  expect(id.readOnly).toBe(false);
  fireEvent.change(id, { target: { value: '2' } });
  commit(id);
  await waitFor(() => expect(previewArgs?.edits?.[0]).toEqual({
    recordIndex: 0,
    field: 'id',
    value: { kind: 'number', value: '2' },
  }));
  expect(screen.queryByRole('dialog')).toBeNull();
});

test('Explorer move refreshes selection and the open data editor at the new path', async () => {
  openSnapshot = { ...mutationSnapshot(), path: 'data.yaml' };
  let moved = false;
  const movedWorkspace = { ...workspace, files: [{ ...workspace.files[0], path: 'moved.yaml' }] };
  const normalInvoke = invoke.getMockImplementation()!;
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'authoring_workspace') return moved ? movedWorkspace : workspace;
    if (command === 'rename_source_file') {
      expect(args.request).toEqual({ sourcePath: 'data.yaml', destinationPath: 'moved.yaml' });
      moved = true;
      return { status: 'success', sourcePath: 'data.yaml', destinationPath: 'moved.yaml', sourceState: null, destinationState: null, diagnostic: null };
    }
    if (command === 'open_data_file') return { ...structuredClone(openSnapshot), path: args.relativePath };
    return normalInvoke(command, args);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  await screen.findByRole('complementary', { name: 'Explorer' });
  openSourceFiles();
  fireEvent.click(screen.getByRole('button', { name: 'More Explorer actions' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Rename or Move Source' }));
  const destination = screen.getByRole('textbox', { name: 'Source destination path' });
  fireEvent.change(destination, { target: { value: 'moved.yaml' } });
  fireEvent.click(screen.getAllByRole('button', { name: 'Move', exact: true }).at(-1)!);
  await waitFor(() => expect(screen.getByRole('treeitem', { name: 'moved.yaml', exact: true })).toBeTruthy());
  expect(screen.queryByRole('treeitem', { name: 'data.yaml', exact: true })).toBeNull();
  expect(invoke.mock.calls.some(([command, args]) => command === 'select_source' && args.relativePath === 'moved.yaml')).toBe(true);
});

test('complex table scope disables Add Row with a reason but keeps existing Delete available', async () => {
  openSnapshot = { ...mutationSnapshot(), addRow: { supported: false, reason: 'Nullable fields are outside the initial Add Row scope.' } };
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const add = await screen.findByRole('button', { name: /Add Row/ });
  expect((add as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByText('Nullable fields are outside the initial Add Row scope.')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Actions for record 1' }));
  expect(screen.getByRole('menuitem', { name: 'Delete Record' })).toBeTruthy();
});

test('structural mutation state survives Failure, Conflict, and Outcome Unknown recovery', async () => {
  openSnapshot = mutationSnapshot([]);
  preview = async () => ({ candidateSource: 'kind: data\ntable: item\nrecords:\n  - id: 1\n', changed: true, validation });
  let saveResponse: any = { status: 'failure', snapshot: null, current: null, diagnostic: { code: 'E-IO', message: 'Cannot write source' } };
  let sourceResponse = { contentIdentity: 'base', source: openSnapshot.baseSource };
  const normalInvoke = invoke.getMockImplementation()!;
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'save_current_table_context') return { files: [{ path: 'data.yaml', status: saveResponse.status, candidateContentIdentity: 'candidate', current: saveResponse.current, diagnostic: saveResponse.diagnostic }] };
    if (command === 'save_data_file') return saveResponse;
    if (command === 'source_content') return sourceResponse;
    return normalInvoke(command, args);
  });

  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  fireEvent.click(await screen.findByRole('button', { name: /Add Row/ }));
  expect(screen.queryByText('Query cleared after Add Row.')).toBeNull();
  const draftId = await edit('new record id') as HTMLInputElement;
  fireEvent.change(draftId, { target: { value: '1' } });
  commit(draftId);
  fireEvent.click(screen.getAllByRole('button', { name: 'Save', exact: true })[0]);
  await waitFor(() => expect(screen.getByText('Save failed.')).toBeTruthy());
  expect(screen.getByRole('gridcell', { name: /new record id: 1/ })).toBeTruthy();

  saveResponse = {
    status: 'conflict',
    snapshot: null,
    current: { path: 'data.yaml', contentIdentity: 'external', source: 'external: true' },
    diagnostic: { code: 'E-SOURCE-EDIT-CONFLICT', message: 'source changed' },
  };
  sourceResponse = { contentIdentity: 'external', source: 'external: true' };
  fireEvent.click(screen.getByRole('button', { name: 'Retry Save', exact: true }));
  await waitFor(() => expect(screen.getByText('File changed outside masterdata.')).toBeTruthy());
  fireEvent.click(screen.getByRole('tab', { name: 'Data', exact: true }));
  expect(screen.getByRole('gridcell', { name: /new record id: 1/ })).toBeTruthy();

  saveResponse = {
    status: 'outcome_unknown',
    snapshot: null,
    current: null,
    diagnostic: { code: 'E-SOURCE-EDIT-WRITE-VERIFY', message: 'could not verify write' },
  };
  fireEvent.click(screen.getByRole('button', { name: 'Overwrite', exact: true }));
  await waitFor(() => expect(screen.getByText('Previous save outcome is unknown.')).toBeTruthy());
  expect(screen.getByRole('gridcell', { name: /new record id: 1/ })).toBeTruthy();
}, APP_INTEGRATION_TEST_TIMEOUT_MS);

const tableSnapshot = {path:'schema.yaml',schema:{table:'item',fields:[{key:0,name:'id',type:'int',nullable:false,array:false}],primaryKey:{fields:['id']},secondaryKeys:[]},fieldTypes:['int','string']};
const tableWorkspace = {...workspace,files:[...workspace.files,{path:'other.yaml',sourceRoot:'.',kind:'data',table:'item',typeName:null,hasInlineRecords:false},{path:'schema.yaml',sourceRoot:'.',kind:'schema',table:'item',typeName:null,hasInlineRecords:false}]};
const tablePlan = {token:'table-plan',table:'item',operation:'RenameField',field:'id',destructive:false,affectedRecordCount:1,files:[{path:'schema.yaml',before:'name: id',after:'name: itemId'},{path:'data.yaml',before:'id: 1',after:'itemId: 1'}],diagnostics:[]};
const recoveryRequired = { state:'recovery_required',files:['schema.yaml'],diagnostic:{code:'E-IO-ACCESS',message:'rollback failed'},recoveryWorkspace:'/recovery' };
test('inline Table file opens record grid and its schema editor in one surface', async () => {
  const normal = invoke.getMockImplementation()!;
  const inlineWorkspace = {...workspace, files: [{path:'schema.yaml',sourceRoot:'.',kind:'schema',table:'item',typeName:null,hasInlineRecords:true}]};
  installWorkspaceFixture(invoke, async (command,args) => {
    if (command === 'authoring_workspace') return inlineWorkspace;
    if (command === 'open_table') return tableSnapshot;
    if (command === 'open_table_context') return { table: 'item', schemaPath: 'schema.yaml', schemaContentIdentity: 'schema-base', schemaSource: 'kind: schema\ntable: item\n', recordSources: [{ path: 'schema.yaml', inline: true }], selectedRecordSource: 'schema.yaml', schema: { ...tableSnapshot, schema: { ...tableSnapshot.schema, fields: [{ key: 1, name: 'weight', type: 'ulong', nullable: false, array: false }] }, fieldTypes: ['ulong', 'string'] } };
    if (command === 'open_data_file') return {...snapshot(), path:'schema.yaml', baseSource:'kind: schema\ntable: item\nrecords:\n  - weight: 10\n'};
    return normal(command,args);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  expect(await screen.findByRole('gridcell',{name:/record 1 weight:/})).toBeTruthy();
  expect(await screen.findByRole('textbox',{name:'Field name weight'})).toBeTruthy();
  expect(screen.getByRole('button',{name:'Add column'})).toBeTruthy();
  expect(screen.getByRole('gridcell',{name:/record 1 weight:/})).toBeTruthy();
}, APP_INTEGRATION_TEST_TIMEOUT_MS);
test('split Data file exposes its Table schema without leaving the record grid', async () => {
  const normal = invoke.getMockImplementation()!;
  installWorkspaceFixture(invoke, async (command,args) => {
    if (command === 'authoring_workspace') return tableWorkspace;
    if (command === 'open_table') return tableSnapshot;
    if (command === 'open_table_context') return { table: 'item', schemaPath: 'schema.yaml', schemaContentIdentity: 'schema-base', schemaSource: 'kind: schema\ntable: item\n', recordSources: [{ path: 'data.yaml', inline: false }, { path: 'other.yaml', inline: false }], selectedRecordSource: args?.relativePath === 'other.yaml' ? 'other.yaml' : 'data.yaml', schema: { ...tableSnapshot, schema: { ...tableSnapshot.schema, fields: [{ key: 1, name: 'weight', type: 'ulong', nullable: false, array: false }] }, fieldTypes: ['ulong', 'string'] } };
    return normal(command,args);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  expect(await screen.findByRole('gridcell',{name:/record 1 weight:/})).toBeTruthy();
  expect(await screen.findByRole('textbox',{name:'Field name weight'})).toBeTruthy();
  expect(screen.getByRole('combobox',{name:'Record set'})).toBeTruthy();
  expect(screen.getByRole('gridcell',{name:/record 1 weight:/})).toBeTruthy();
}, APP_INTEGRATION_TEST_TIMEOUT_MS);
test('Column pointer drag changes declaration order and retains keyboard context actions', async () => {
  const normal = invoke.getMockImplementation()!;
  openSnapshot = mutationSnapshot() as any;
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'open_table_context') return {
      table: 'item', schemaPath: 'schema.yaml', schemaContentIdentity: 'schema-base', schemaSource: 'kind: schema\ntable: item\n',
      recordSources: [{ path: 'data.yaml', inline: false }], selectedRecordSource: 'data.yaml',
      schema: { schema: { table: 'item', fields: [
        { key: 0, name: 'id', type: 'ulong', nullable: false, array: false },
        { key: 1, name: 'weight', type: 'ulong', nullable: false, array: false },
        { key: 2, name: 'note', type: 'string', nullable: false, array: false },
      ] }, fieldTypes: ['ulong', 'string'] },
    };
    return normal(command, args);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const grab = await screen.findByRole('button', { name: 'Drag column note to reorder' }, { timeout: 10_000 });
  const first = screen.getByRole('textbox', { name: 'Field name id' }).closest('th') as HTMLElement;
  const second = screen.getByRole('textbox', { name: 'Field name weight' }).closest('th') as HTMLElement;
  const last = grab.closest('th') as HTMLElement;
  vi.spyOn(first, 'getBoundingClientRect').mockReturnValue({ left: 0, width: 100 } as DOMRect);
  vi.spyOn(second, 'getBoundingClientRect').mockReturnValue({ left: 100, width: 100 } as DOMRect);
  vi.spyOn(last, 'getBoundingClientRect').mockReturnValue({ left: 200, width: 100 } as DOMRect);
  const scroll = first.closest<HTMLElement>('.grid-scroll')!;
  vi.spyOn(scroll, 'getBoundingClientRect').mockReturnValue({ left: 0, top: 0, right: 500, bottom: 400 } as DOMRect);
  Object.defineProperties(scroll, { clientWidth: { configurable: true, value: 500 }, clientHeight: { configurable: true, value: 400 } });
  const commandsBeforeCancel = invoke.mock.calls.length;
  fireEvent.pointerDown(grab, { button: 0, pointerId: 1, clientX: 250, clientY: 10 });
  fireEvent.pointerMove(window, { pointerId: 1, clientX: 1, clientY: 10 });
  expect(first.closest('table')?.dataset.dragDestination).toBe('0');
  expect(first.style.transform).toBe('translateX(100px)');
  expect(second.style.transform).toBe('translateX(100px)');
  expect(document.querySelector('.grid-drag-ghost')).toBeTruthy();
  expect(invoke.mock.calls.length).toBe(commandsBeforeCancel);
  fireEvent.keyDown(window, { key: 'Escape' });
  expect(document.querySelector('.grid-drag-overlay')).toBeNull();
  expect(first.style.transform).toBe('');
  expect(invoke.mock.calls.length).toBe(commandsBeforeCancel);
  fireEvent.pointerDown(grab, { button: 0, pointerId: 1, clientX: 250, clientY: 10 });
  fireEvent.pointerMove(window, { pointerId: 1, clientX: 1, clientY: 10 });
  fireEvent.pointerUp(window, { pointerId: 1, clientX: 1, clientY: 10 });
  await waitFor(() => expect(invoke.mock.calls.find(([command]) => command === 'preview_schema_draft')?.[1].fields.map((field: any) => field.name)).toEqual(['note', 'id', 'weight']));
  await waitFor(() => expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Drag column note to reorder' })));
  fireEvent.keyDown(document.activeElement as HTMLElement, { key: 'F10', shiftKey: true });
  expect(await screen.findByRole('menuitem', { name: 'Insert column right' })).toBeTruthy();
}, APP_INTEGRATION_TEST_TIMEOUT_MS);
test('switching record files restores each file selection without mixing grid state', async () => {
  const normal = invoke.getMockImplementation()!;
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'authoring_workspace') return tableWorkspace;
    if (command === 'open_data_file') return { ...mutationSnapshot(), path: args.relativePath };
    if (command === 'open_table_context') return {
      table: 'item', schemaPath: 'schema.yaml', schemaContentIdentity: 'schema-base', schemaSource: 'kind: schema\ntable: item\n',
      recordSources: [{ path: 'data.yaml', inline: false }, { path: 'other.yaml', inline: false }],
      selectedRecordSource: args.relativePath,
      schema: { ...tableSnapshot, schema: { ...tableSnapshot.schema, fields: [{ key: 1, name: 'weight', type: 'ulong', nullable: false, array: false }] }, fieldTypes: ['ulong'] },
    };
    return normal(command, args);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const cell = await screen.findByRole('gridcell', { name: /^record 1 weight:/ });
  fireEvent.mouseDown(cell, { button: 0 });
  expect(screen.getByText('1 cells selected')).toBeTruthy();
  fireEvent.click(screen.getByRole('treeitem', { name: 'other.yaml', exact: true }));
  await waitFor(() => expect(screen.getByRole('treeitem', { name: 'other.yaml', exact: true }).getAttribute('aria-selected')).toBe('true'));
  expect(screen.getByText('One cell active')).toBeTruthy();
  fireEvent.click(screen.getByRole('treeitem', { name: 'data.yaml', exact: true }));
  await waitFor(() => expect(screen.getByText('1 cells selected')).toBeTruthy());
}, APP_INTEGRATION_TEST_TIMEOUT_MS);
test('mixed Table Save composes dirty inline records with the selected separate source', async () => {
  const normal = invoke.getMockImplementation()!;
  const mixedWorkspace = { ...workspace, files: [
    { path: 'data.yaml', sourceRoot: '.', kind: 'data', table: 'item', typeName: null, hasInlineRecords: false },
    { path: 'schema.yaml', sourceRoot: '.', kind: 'schema', table: 'item', typeName: null, hasInlineRecords: true },
  ] };
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'authoring_workspace') return mixedWorkspace;
    if (command === 'open_table_context') return {
      table: 'item', schemaPath: 'schema.yaml', schemaContentIdentity: 'schema-base', schemaSource: 'kind: schema\ntable: item\nrecords:\n  - weight: 10\n',
      recordSources: [{ path: 'schema.yaml', inline: true }, { path: 'data.yaml', inline: false }],
      selectedRecordSource: args.relativePath === 'schema.yaml' ? 'schema.yaml' : 'data.yaml',
      schema: { schema: { table: 'item', fields: [{ key: 0, name: 'weight', type: 'ulong', nullable: false, array: false }] }, fieldTypes: ['ulong'] },
    };
    if (command === 'open_data_file') return { ...snapshot(), path: args.relativePath,
      baseSource: args.relativePath === 'schema.yaml' ? 'kind: schema\ntable: item\nrecords:\n  - weight: 10\n' : 'weight: 10',
      baseContentIdentity: args.relativePath === 'schema.yaml' ? 'schema-base' : 'base' };
    return normal(command, args);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  fireEvent.click(await screen.findByRole('treeitem', { name: 'schema.yaml', exact: true }));
  const inlineInput = await edit('record 1 weight');
  fireEvent.change(inlineInput, { target: { value: '21' } }); commit(inlineInput);
  await screen.findByRole('gridcell', { name: /^record 1 weight: 21/ });
  fireEvent.click(screen.getByRole('treeitem', { name: 'data.yaml', exact: true }));
  const separateInput = await edit('record 1 weight');
  fireEvent.change(separateInput, { target: { value: '22' } }); commit(separateInput);
  await screen.findByRole('gridcell', { name: /^record 1 weight: 22/ });
  fireEvent.click(screen.getByRole('button', { name: 'Save', exact: true }));
  await waitFor(() => expect(invoke.mock.calls.some(([command]) => command === 'save_current_table_context')).toBe(true));
  const request = invoke.mock.calls.find(([command]) => command === 'save_current_table_context')![1].request;
  expect(request.selectedRecordSource).toBe('data.yaml');
  expect(request.inlineRecordDraft.mutation.edits[0].value).toEqual({ kind: 'number', value: '21' });
  expect(request.recordDraft.mutation.edits[0].value).toEqual({ kind: 'number', value: '22' });
}, APP_INTEGRATION_TEST_TIMEOUT_MS);
test('Cmd+S uses the same current Table Save intent as the header', async () => {
  const input = await open();
  fireEvent.change(input, { target: { value: '20' } }); commit(input);
  await screen.findByRole('gridcell', { name: /^record 1 weight: 20/ });
  fireEvent.keyDown(window, { key: 's', metaKey: true });
  await waitFor(() => expect(invoke.mock.calls.some(([command]) => command === 'save_current_table_context')).toBe(true));
  expect(invoke.mock.calls.find(([command]) => command === 'save_current_table_context')![1].request).toMatchObject({
    schemaPath: 'schema.yaml', selectedRecordSource: 'data.yaml', schemaDraft: null,
    recordDraft: { mutation: { edits: [{ field: 'weight', value: { kind: 'number', value: '20' } }] } },
  });
}, APP_INTEGRATION_TEST_TIMEOUT_MS);
test('Cmd+S inside a cell commits its current text before the Table Save intent', async () => {
  const input = await open();
  fireEvent.change(input, { target: { value: '20' } });
  fireEvent.keyDown(input, { key: 's', metaKey: true });
  await waitFor(() => expect(invoke.mock.calls.some(([command]) => command === 'save_current_table_context')).toBe(true));
  expect(invoke.mock.calls.find(([command]) => command === 'save_current_table_context')![1].request.recordDraft.mutation.edits)
    .toMatchObject([{ field: 'weight', value: { kind: 'number', value: '20' } }]);
}, APP_INTEGRATION_TEST_TIMEOUT_MS);
test('column header commits rename and modifier through one safe intent without Plan UI', async () => {
  const normal = invoke.getMockImplementation()!;
  let field = { key: 0, name: 'weight', type: 'ulong', nullable: false, array: false };
  const intents: any[] = [];
  installWorkspaceFixture(invoke, async (command,args) => {
    if (command === 'open_table_context') return { table: 'item', schemaPath: 'schema.yaml', schemaContentIdentity: field.name === 'weight' ? 'schema-base' : 'schema-renamed', schemaSource: `kind: schema\ntable: item\n# ${field.name}\n`, recordSources: [{ path: 'data.yaml', inline: false }], selectedRecordSource: 'data.yaml', schema: { path: 'schema.yaml', schema: { table: 'item', fields: [field], primaryKey: { fields: [] }, secondaryKeys: [] }, fieldTypes: ['ulong', 'string'] } };
    if (command === 'apply_table_intent') {
      intents.push(args.input);
      field = { ...field, ...(args.input.operation === 'rename' ? { name: args.input.newName } : { nullable: args.input.nullable, array: args.input.array }) };
      return { state: 'success', files: ['schema.yaml'] };
    }
    if (command === 'open_data_file' && field.name === 'mass') {
      const next = snapshot(); next.columns[0].name = 'mass'; next.rows[0].cells[0].field = 'mass'; return next;
    }
    return normal(command,args);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const name = await screen.findByRole('textbox', { name: 'Field name weight' });
  fireEvent.change(name, { target: { value: 'mass' } }); fireEvent.blur(name);
  await waitFor(() => expect(intents[0]).toMatchObject({ operation: 'rename', table: 'item', field: 'weight', newName: 'mass' }));
  expect(invoke.mock.calls.find(([command]) => command === 'apply_table_intent')?.[1].expectedSources).toEqual([
    { path: 'schema.yaml', contentIdentity: 'schema-base' },
    { path: 'data.yaml', contentIdentity: 'base' },
  ]);
  const nullable = await screen.findByRole('button', { name: 'Nullable mass' });
  fireEvent.click(nullable);
  await waitFor(() => expect(invoke.mock.calls.find(([command]) => command === 'preview_schema_draft')?.[1].fields[0]).toMatchObject({ name: 'mass', nullable: true, array: false }));
  expect(intents).toHaveLength(1);
  expect(screen.queryByRole('region', { name: 'Migration Plan' })).toBeNull();
}, APP_INTEGRATION_TEST_TIMEOUT_MS);
test('schema and record drafts save through one current Table command', async () => {
  const normal = invoke.getMockImplementation()!;
  let saved = false;
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'open_table_context') return {
      table: 'item', schemaPath: 'schema.yaml', schemaContentIdentity: saved ? 'schema-new' : 'schema-base',
      schemaSource: saved ? 'kind: schema\ntable: item\n# saved\n' : 'kind: schema\ntable: item\n',
      recordSources: [{ path: 'data.yaml', inline: false }], selectedRecordSource: 'data.yaml',
      schema: { schema: { table: 'item', fields: [{ key: 0, name: 'weight', type: 'ulong', nullable: saved, array: false }] }, fieldTypes: ['ulong', 'string'] },
    };
    if (command === 'preview_schema_draft') return { candidateSource: 'kind: schema\ntable: item\n# saved\n', candidateContentIdentity: 'schema-new', changed: true,
      validation: { valid: false, diagnostics: [{ code: 'E-TABLE-INVALID-RECORD-VALUE', kind: 'validation', message: 'field `weight` is invalid', source: 'data.yaml', record_identity: 'record[0]' }] },
      selectedSnapshot: null };
    if (command === 'save_current_table_context') { saved = true; return { files: [
      { path: 'schema.yaml', status: 'success', candidateContentIdentity: 'schema-new', current: null, diagnostic: null },
      { path: 'data.yaml', status: 'success', candidateContentIdentity: 'base', current: null, diagnostic: null },
    ] }; }
    if (command === 'source_content' && args.relativePath === 'schema.yaml') return { path: 'schema.yaml', source: 'kind: schema\ntable: item\n# saved\n', contentIdentity: 'schema-new' };
    if (command === 'open_data_file' && saved) {
      const next = snapshot(); next.columns[0].shape.modifier = 'nullable';
      next.rows[0].cells[0].text = '20'; next.rows[0].cells[0].value = { kind: 'number', value: '20' };
      return next;
    }
    return normal(command, args);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const cell = await screen.findByRole('gridcell', { name: /^record 1 weight: 10/ });
  fireEvent.keyDown(cell, { key: 'Enter' });
  const value = await screen.findByRole('textbox', { name: 'record 1 weight' });
  fireEvent.change(value, { target: { value: '20' } });
  fireEvent.keyDown(value, { key: 'Enter' });
  await screen.findByRole('gridcell', { name: /^record 1 weight: 20/ });
  const nullable = await screen.findByRole('button', { name: 'Nullable weight' });
  fireEvent.click(nullable);
  await waitFor(() => expect(invoke.mock.calls.some(([command]) => command === 'preview_schema_draft')).toBe(true));
  expect(screen.getByRole('button', { name: 'Schema unsaved changes and actions' }).closest('.editor-tabs')).toBeTruthy();
  expect(document.querySelector('.schema-draft-strip')).toBeNull();
  fireEvent.keyDown(nullable, { key: 'z', metaKey: true });
  await waitFor(() => expect(screen.getByRole('button', { name: 'Schema redo available' })).toBeTruthy());
  fireEvent.keyDown(nullable, { key: 'z', metaKey: true, shiftKey: true });
  await screen.findByRole('button', { name: 'Schema unsaved changes and actions' });
  fireEvent.click(screen.getByRole('button', { name: 'Save', exact: true }));
  await waitFor(() => expect(invoke.mock.calls.find(([command]) => command === 'save_current_table_context')?.[1].request.schemaDraft.fields[0].nullable).toBe(true));
  expect(invoke.mock.calls.find(([command]) => command === 'save_current_table_context')?.[1].request.recordDraft.mutation.edits).toMatchObject([{ field: 'weight', value: { kind: 'number', value: '20' } }]);
  await waitFor(() => expect(invoke.mock.calls.filter(([command]) => command === 'select_source').length).toBeGreaterThan(1));
  expect(screen.getByRole('gridcell', { name: /^record 1 weight: 20/ })).toBeTruthy();
  expect(invoke.mock.calls.some(([command]) => command === 'save_schema_draft' || command === 'save_data_file')).toBe(false);
  expect(invoke.mock.calls.some(([command]) => command === 'apply_table_intent')).toBe(false);
}, APP_INTEGRATION_TEST_TIMEOUT_MS);
test('partial Table Save reports each source and retains the failed schema draft', async () => {
  const normal = invoke.getMockImplementation()!;
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'save_current_table_context') return { files: [
      { path: 'data.yaml', status: 'success', candidateContentIdentity: 'base', current: { path: 'data.yaml', source: 'weight: 20', contentIdentity: 'base' }, diagnostic: null },
      { path: 'schema.yaml', status: 'failure', candidateContentIdentity: 'schema-candidate', current: { path: 'schema.yaml', source: 'kind: schema\ntable: item\n', contentIdentity: 'schema-base' }, diagnostic: { code: 'E-IO', kind: 'io', message: 'Schema write failed' } },
    ] };
    return normal(command, args);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const cell = await screen.findByRole('gridcell', { name: /^record 1 weight:/ });
  fireEvent.keyDown(cell, { key: 'Enter' });
  const input = await screen.findByRole('textbox', { name: 'record 1 weight' });
  fireEvent.change(input, { target: { value: '20' } }); commit(input);
  fireEvent.click(await screen.findByRole('button', { name: 'Nullable weight' }));
  await screen.findByRole('button', { name: 'Schema unsaved changes and actions' });
  fireEvent.click(screen.getByRole('button', { name: 'Save', exact: true }));
  await waitFor(() => expect(invoke.mock.calls.some(([command]) => command === 'save_current_table_context')).toBe(true));
  const dialog = (await screen.findByText('Save incomplete')).closest('[role="dialog"]')!;
  expect(within(dialog).getByText('data.yaml').closest('li')?.textContent).toContain('saved');
  expect(within(dialog).getByText('schema.yaml').closest('li')?.textContent).toContain('failure');
  expect(screen.getByRole('button', { name: 'Schema unsaved changes and actions' })).toBeTruthy();
  expect(invoke.mock.calls.some(([command]) => command === 'save_schema_draft' || command === 'save_data_file')).toBe(false);
}, APP_INTEGRATION_TEST_TIMEOUT_MS);

test('schema diagnostic propagation leaves the separate record source clean', async () => {
  const normal = invoke.getMockImplementation()!;
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'preview_schema_draft') return {
      candidateSource: 'kind: schema\ntable: item\n# draft\n',
      candidateContentIdentity: 'schema-candidate',
      changed: true,
      validation: { valid: false, diagnostics: [{
        code: 'E-TABLE-INVALID-RECORD-VALUE', kind: 'validation',
        message: 'weight cannot be interpreted', source: 'data.yaml', record_identity: 'record[0]',
      }] },
      selectedSnapshot: null,
    };
    return normal(command, args);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  await screen.findByRole('gridcell', { name: /^record 1 weight:/ }, { timeout: 10_000 });
  const nullable = await screen.findByRole('button', { name: 'Nullable weight' });
  expect(screen.getByRole('button', { name: 'Save', exact: true }).hasAttribute('disabled')).toBe(true);
  fireEvent.click(nullable);
  await screen.findByRole('button', { name: /PROBLEMS 1/ }, { timeout: 10_000 });
  expect(screen.getByRole('button', { name: 'Schema unsaved changes and actions' })).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Save', exact: true }).hasAttribute('disabled')).toBe(false);
  fireEvent.keyDown(nullable, { key: 'z', metaKey: true });
  await screen.findByRole('button', { name: /PROBLEMS 0/ }, { timeout: 10_000 });
  expect(screen.getByRole('button', { name: 'Save', exact: true }).hasAttribute('disabled')).toBe(true);
  expect(invoke.mock.calls.some(([command]) => command === 'save_data_file')).toBe(false);
}, APP_INTEGRATION_TEST_TIMEOUT_MS);
test('Migration recovery result blocks Create and Build',async()=>{
  const normal=invoke.getMockImplementation()!;
  installWorkspaceFixture(invoke, async(command,args)=>{
    if(command==='authoring_workspace')return tableWorkspace;
    if(command==='open_table')return tableSnapshot;
    if(command==='apply_table_intent')return recoveryRequired;
    return normal(command,args);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const input=await screen.findByRole('textbox',{name:'Field name weight'});
  fireEvent.change(input,{target:{value:'mass'}});fireEvent.blur(input);
  await screen.findByText('Recovery Required — source changes and Build are blocked');
  openProjectCommands();
  expect((screen.getByRole('button',{name:'New source artifact'}) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole('button',{name:'More Explorer actions'}));
  expect(screen.getByRole('menuitem',{name:'Rename or Move Source'}).getAttribute('aria-disabled')).toBe('true');
  expect(screen.getByRole('menuitem',{name:'Build',exact:true}).getAttribute('aria-disabled')).toBe('true');
}, APP_INTEGRATION_TEST_TIMEOUT_MS);
test('Recovery Required blocks Save until host recheck succeeds',async()=>{
  const normal=invoke.getMockImplementation()!;
  let recovery:any=recoveryRequired;
  installWorkspaceFixture(invoke, async(command,args)=>{
    if(command==='authoring_workspace')return tableWorkspace;
    if(command==='migration_recovery_status')return recovery;
    if(command==='recheck_migration'){recovery=null;return null;}
    return normal(command,args);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  await screen.findByRole('gridcell', { name: /record 1 weight: 10/ });
  await screen.findByText('Recovery Required — source changes and Build are blocked');
  openProjectCommands();
  openSourceFiles();
  expect((screen.getByRole('button',{name:'New source artifact'}) as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByRole('menuitem',{name:'Build',exact:true}).getAttribute('aria-disabled')).toBe('true');
  fireEvent.click(screen.getByRole('treeitem',{name:'data.yaml',exact:true}));
  expect(screen.getAllByRole('button',{name:'Save',exact:true}).every(button=>(button as HTMLButtonElement).disabled)).toBe(true);
  fireEvent.keyDown(window,{key:'s',metaKey:true});
  expect(invoke.mock.calls.some(([command])=>command==='save_data_file')).toBe(false);
  fireEvent.click(screen.getByRole('button',{name:'Recheck recovered source'}));
  await waitFor(()=>expect(screen.getByRole('menuitem',{name:'Build',exact:true}).getAttribute('aria-disabled')).not.toBe('true'));
  expect(await screen.findByRole('gridcell',{name:/record 1 weight: 10/})).toBeTruthy();
}, APP_INTEGRATION_TEST_TIMEOUT_MS);


test('Cmd/Ctrl+S on Settings saves masterdata.toml and never the active YAML editor', async () => {
  const normalInvoke = invoke.getMockImplementation()!;
  const config = {
    projectRoot: '/project',
    configPath: '/project/masterdata.toml',
    baseSource: 'base0',
    baseContentIdentity: 'config0',
    configValid: true,
    profiles: [{ name: 'prod', include_tags: ['old'], exclude_tags: [] }],
    publishTargets: [],
    diagnostics: [],
  };
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'open_project_config') return structuredClone(config);
    if (command === 'preview_project_config_edit') {
      return {
        baseContentIdentity: args.baseContentIdentity,
        candidateContentIdentity: 'config1',
        candidateSource: 'base1',
        changed: true,
        configValid: true,
        diagnostics: [],
      };
    }
    if (command === 'save_project_config_edit') {
      return {
        status: 'success',
        snapshot: { ...structuredClone(config), baseSource: 'base1', baseContentIdentity: 'config1', profiles: [{ name: 'prod', include_tags: ['new'], exclude_tags: [] }] },
        current: null,
        diagnostic: null,
      };
    }
    return normalInvoke(command, args);
  });

  await open();
  openProjectCommands();
  fireEvent.click(screen.getByRole('menuitem', { name: 'Project Settings', exact: true }));
  await screen.findByRole('heading', { name: 'Project Settings' });
  fireEvent.change(screen.getByLabelText('Include tags'), { target: { value: 'new' } });
  fireEvent.keyDown(window, { key: 's', ctrlKey: true });

  await waitFor(() => expect(invoke.mock.calls.some(([command]) => command === 'save_project_config_edit')).toBe(true));
  expect(invoke.mock.calls.some(([command]) => command === 'save_data_file')).toBe(false);
});


test('grid paste applies one shared batch to the local buffer and Undo restores it', async () => {
  const normalInvoke = invoke.getMockImplementation()!;
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'authoring_clipboard_shape') return { rows: 1, columns: 1 };
    if (command === 'preview_data_file_batch') return {
      source: { candidateSource: 'weight: 20', candidateContentIdentity: 'batch', changed: true, validation },
      targetCount: 1,
      changedCellCount: 1,
      changes: [{ recordIndex: 0, addedRecordIndex: null, field: 'weight', before: { kind: 'number', value: '10' }, after: { kind: 'number', value: '20' } }],
    };
    return normalInvoke(command, args);
  });
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { readText: vi.fn(async () => '20'), writeText: vi.fn(async () => {}) } });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const cell = await screen.findByRole('gridcell', { name: /^record 1 weight:/ });
  fireEvent.mouseDown(cell, { button: 0 });
  fireEvent.keyDown(cell, { key: 'v', ctrlKey: true });
  expect(await screen.findByRole('gridcell', { name: /^record 1 weight: 20/ })).toBeTruthy();
  expect(screen.queryByRole('dialog', { name: 'Scalar range preview' })).toBeNull();
  await dataAction('Undo');
  expect(await screen.findByRole('gridcell', { name: /^record 1 weight: 10/ })).toBeTruthy();
});

test('single-cell Cmd+C uses shared copy instead of an empty native selection', async () => {
  const normalInvoke = invoke.getMockImplementation()!;
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'copy_data_file_batch') return { clipboardText: '10', targetCount: 1 };
    return normalInvoke(command, args);
  });
  const writeText = vi.fn(async () => {});
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { readText: vi.fn(), writeText } });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const cell = await screen.findByRole('gridcell', { name: /^record 1 weight:/ });
  fireEvent.mouseDown(cell, { button: 0 });
  fireEvent.keyDown(cell, { key: 'c', metaKey: true });
  await waitFor(() => expect(writeText).toHaveBeenCalledWith('10'));
  expect(invoke.mock.calls.find(([command]) => command === 'copy_data_file_batch')![1].request.targets).toEqual([{ recordIndex: 0, field: 'weight' }]);
});

test('2x2 Paste derives a 2x2 target rectangle from the active cell instead of flattening the selection', async () => {
  openSnapshot = mutationSnapshot([
    { recordIndex: 0, cells: [
      { field: 'id', text: '1', editable: true },
      { field: 'weight', text: '10', editable: true },
      { field: 'note', text: 'a', editable: true },
    ] },
    { recordIndex: 1, cells: [
      { field: 'id', text: '2', editable: true },
      { field: 'weight', text: '20', editable: true },
      { field: 'note', text: 'b', editable: true },
    ] },
  ]);
  const normalInvoke = invoke.getMockImplementation()!;
  let batchArgs: any;
  installWorkspaceFixture(invoke, async (command, args) => {
    if (command === 'authoring_clipboard_shape') return { rows: 2, columns: 2 };
    if (command === 'preview_data_file_batch') {
      batchArgs = args;
      return {
        source: {
          candidateSource: openSnapshot.baseSource,
          candidateContentIdentity: 'batch-preview',
          changed: false,
          validation,
        },
        targetCount: 4,
        changedCellCount: 0,
        changes: [],
      };
    }
    return normalInvoke(command, args);
  });
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: { readText: vi.fn(async () => '11\tx\n22\ty'), writeText: vi.fn(async () => {}) },
  });

  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const cell = await screen.findByRole('gridcell', { name: /record 1 weight:/ });
  fireEvent.mouseDown(cell, { button: 0 });
  fireEvent.keyDown(cell, { key: 'v', ctrlKey: true });

  await waitFor(() => expect(batchArgs).toBeDefined());
  expect(batchArgs.request.targets).toEqual([
    { recordIndex: 0, field: 'weight' },
    { recordIndex: 0, field: 'note' },
    { recordIndex: 1, field: 'weight' },
    { recordIndex: 1, field: 'note' },
  ]);
  expect(batchArgs.request.clipboardText).toBe('11\tx\n22\ty');
  expect(batchArgs.request.fill).toBe(false);
});

function requestDesktopClose() {
  const event = { preventDefault: vi.fn() };
  act(() => desktopWindow.onCloseRequested.mock.calls.at(-1)![0](event));
  return event;
}

test('desktop close allows clean state and Cancel preserves dirty input', async () => {
  const input = await open();
  expect(requestDesktopClose().preventDefault).not.toHaveBeenCalled();
  fireEvent.change(input, { target: { value: '20' } });
  commit(input);
  expect(requestDesktopClose().preventDefault).toHaveBeenCalledOnce();
  fireEvent.click(await screen.findByRole('button', { name: 'Cancel', exact: true }));
  expect(screen.getByRole('gridcell',{name:/record 1 weight: 20/})).toBeTruthy();
  expect(desktopWindow.destroy).not.toHaveBeenCalled();
});

test("desktop close Don't Save destroys the window without writing source", async () => {
  const input = await open();
  fireEvent.change(input, { target: { value: '20' } });
  commit(input);
  requestDesktopClose();
  fireEvent.click(await screen.findByRole('button', { name: "Don't Save", exact: true }));
  await waitFor(() => expect(desktopWindow.destroy).toHaveBeenCalledOnce());
  expect(invoke.mock.calls.some(([command]) => command === 'save_data_file')).toBe(false);
});

test.each(['success', 'failure'])('desktop close Save All respects source save %s', async (status) => {
  const normalInvoke = invoke.getMockImplementation()!;
  let finishSave!: (result: unknown) => void;
  installWorkspaceFixture(invoke, (command, args) => command === 'save_data_file'
    ? new Promise(resolve => { finishSave = resolve; }) : normalInvoke(command, args));
  const input = await open();
  fireEvent.change(input, { target: { value: '20' } });
  commit(input);
  requestDesktopClose();
  fireEvent.click(await screen.findByRole('button', { name: 'Save All', exact: true }));
  await waitFor(() => expect(finishSave).toBeTypeOf('function'));
  expect(desktopWindow.destroy).not.toHaveBeenCalled();
  await act(async () => finishSave(status === 'success'
    ? { status, snapshot: snapshot() }
    : { status, diagnostic: { code: 'E-IO', message: 'Cannot write source' } }));
  if (status === 'success') {
    await waitFor(() => expect(desktopWindow.destroy).toHaveBeenCalledOnce());
  } else {
    expect(desktopWindow.destroy).not.toHaveBeenCalled();
    expect(screen.getByRole('dialog')).toBeTruthy();
    expect(screen.getByRole('gridcell',{name:/record 1 weight: 20/})).toBeTruthy();
  }
});


test('Create Project Cancel returns to Welcome without creating a Project', async () => {
  invoke.mockRejectedValue({ diagnostic: { code: 'E-PROJECT-NOT-FOUND', message: 'No project here' } });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  await screen.findByRole('heading', { name: 'Start with a Masterdata project' });
  fireEvent.click(screen.getAllByRole('button', { name: 'Create Project', exact: true })[0]);
  fireEvent.change(await screen.findByLabelText('New project name'), { target: { value: 'Draft' } });
  fireEvent.click(screen.getByRole('button', { name: 'Cancel', exact: true }));
  expect(await screen.findByRole('heading', { name: 'Start with a Masterdata project' })).toBeTruthy();
  expect(screen.getByRole('complementary', { name: 'Recent Projects' })).toBeTruthy();
  expect(invoke.mock.calls.some(([command]) => command === 'create_project')).toBe(false);
});
