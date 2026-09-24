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
  files: [{ path: 'data.yaml', sourceRoot: '.', kind: 'data' }] };
let openSnapshot: ReturnType<typeof snapshot>;
let preview: (args: any) => Promise<any>;
beforeEach(() => {
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
  invoke.mockImplementation(async (command, args) => {
    if (command === 'migration_recovery_status') return null;
    if (command === 'authoring_workspace') return workspace;
    if (command === 'open_data_file') return structuredClone(openSnapshot);
    if (command === 'preview_data_file') return preview(args);
    if (command === 'save_data_file') return { status: 'success', snapshot: snapshot() };
    if (command === 'source_content') return { contentIdentity: 'base', source: 'weight: 10' };
    if (command === 'build') return { generatedFiles: [] };
    throw new Error(`Unexpected command: ${command}`);
  });
});
afterEach(() => {
  cleanup();
  window.localStorage.clear();
});
async function open(label = 'record 1 weight') { render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />); const cell = await screen.findByRole('gridcell', { name: new RegExp(`^${label}:`) }); fireEvent.keyDown(cell, { key: 'Enter' }); return await screen.findByRole('textbox', { name: label }); }
function commit(input: HTMLElement) { fireEvent.keyDown(input, { key: 'Enter' }); }
async function edit(label: string) { const cell = await screen.findByRole('gridcell', { name: new RegExp(`^${label}:`) }); fireEvent.keyDown(cell, { key: 'Enter' }); return await screen.findByRole('textbox', { name: label }); }
function navigation() { return within(screen.getByRole('complementary', { name: 'Explorer' })); }
function openProjectCommands() {
  fireEvent.click(within(document.querySelector('.titlebar')!).getByRole('button', { name: 'Project menu' }));
}
function openSourceFiles() { /* Explorer is the persistent source tree. */ }
function reloadProject() {
  fireEvent.click(within(document.querySelector('.titlebar')!).getByRole('button', { name: 'Project menu' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Reload Project' }));
}

test('initial Project-not-found is a Welcome state without an Explorer error', async () => {
  invoke.mockImplementation(async (command) => {
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
  invoke.mockImplementation(async (command, args) => {
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
  expect(JSON.parse(window.localStorage.getItem('masterdata.recent-projects.v1') ?? '[]')).toEqual([
    { root: '/project', name: 'Demo' },
  ]);
});

test('cancelling the native Project picker leaves the Welcome state unchanged', async () => {
  invoke.mockImplementation(async (command) => {
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
  invoke.mockImplementation(async (command, args) => {
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
  window.localStorage.setItem('masterdata.recent-projects.v1', JSON.stringify([{ root: '/recent', name: 'Recent Demo' }]));
  invoke.mockImplementation(async (command) => {
    if (command === 'authoring_workspace') throw { diagnostic: { code: 'E-PROJECT-NOT-FOUND', message: 'No project here' } };
    throw new Error(`Unexpected command: ${command}`);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  expect(await screen.findByText('Recent Demo')).toBeTruthy();
  const initialCalls = invoke.mock.calls.length;
  fireEvent.click(screen.getByRole('button', { name: 'Remove Recent Demo from Recent Projects' }));
  expect(screen.queryByText('Recent Demo')).toBeNull();
  expect(window.localStorage.getItem('masterdata.recent-projects.v1')).toBe('[]');
  expect(invoke.mock.calls).toHaveLength(initialCalls);
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
  expect(screen.getByRole('gridcell', { name: /record 1 weight: 20/ })).toBeTruthy();
  expect(screen.getByText('1 dirty')).toBeTruthy();
});

test('project-wide diagnostics mark only the source file that owns the cell', async () => {
  openSnapshot.validation = { valid: false, diagnostics: [{ code: 'E-TABLE-INVALID-RECORD-VALUE', source: '/project/other/data.yaml', record_identity: 'record[0]', message: 'field `weight` is invalid' }] } as any;
  const input = await open();
  expect(input.closest('td')?.classList.contains('invalid')).toBe(false);
  expect(screen.getByText('field `weight` is invalid')).toBeTruthy();
});

test('diagnostic belonging to the selected source marks its cell', async () => {
  openSnapshot.validation = { valid: false, diagnostics: [{ code: 'E-TABLE-INVALID-RECORD-VALUE', source: '/project/data.yaml', record_identity: 'record[0]', message: 'field `weight` is invalid' }] } as any;
  const input = await open();
  expect(input.closest('td')?.classList.contains('invalid')).toBe(true);
});

test('Explorer exposes only source roots and F2 opens source move', async () => {
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const tree = await screen.findByRole('tree', { name: 'Project source files' });
  expect(within(tree).getByRole('treeitem', { name: '.', exact: true })).toBeTruthy();
  expect(navigation().queryByRole('button', { name: 'Tables' })).toBeNull();
  fireEvent.keyDown(within(tree).getByRole('treeitem', { name: 'data.yaml', exact: true }), { key: 'F2' });
  expect(await screen.findByRole('dialog')).toBeTruthy();
  expect(screen.getByText('Rename or move source')).toBeTruthy();
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

test('filter options stay out of the default grid and retain their draft when reopened', async () => {
  await open();
  expect(screen.queryByRole('textbox', { name: 'Data filter value' })).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Filter & sort' }));
  const value = screen.getByRole('textbox', { name: 'Data filter value' }) as HTMLInputElement;
  fireEvent.change(value, { target: { value: 'rare' } });
  fireEvent.click(screen.getByRole('button', { name: 'Filter & sort' }));
  expect(screen.queryByRole('textbox', { name: 'Data filter value' })).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Filter & sort' }));
  expect((screen.getByRole('textbox', { name: 'Data filter value' }) as HTMLInputElement).value).toBe('rare');
});

test('Data query draft survives visiting a Project area and returning to the same source', async () => {
  await open();
  fireEvent.click(screen.getByRole('button', { name: 'Filter & sort' }));
  fireEvent.change(screen.getByRole('textbox', { name: 'Data filter value' }), { target: { value: 'rare' } });
  openProjectCommands();
  fireEvent.click(screen.getByRole('menuitem', { name: 'Build & Publish' }));
  expect(await screen.findByRole('heading', { name: 'Build / Publish' })).toBeTruthy();
  openSourceFiles();
  fireEvent.click(navigation().getByRole('treeitem', { name: 'data.yaml', exact: true }));
  expect((screen.getByRole('textbox', { name: 'Data filter value' }) as HTMLInputElement).value).toBe('rare');
});

test('Data query drafts remain file-local when switching between Data documents', async () => {
  const multiFileWorkspace = { ...workspace, files: [
    { ...workspace.files[0], table: 'item', typeName: null },
    { path: 'other.yaml', sourceRoot: '.', kind: 'data', table: 'item', typeName: null },
  ] };
  const normalInvoke = invoke.getMockImplementation()!;
  invoke.mockImplementation(async (command, args) => {
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
  expect((screen.getByRole('textbox', { name: 'Data filter value' }) as HTMLInputElement).value).toBe('rare');
});

test('Table context opens Data creation with its Table already selected', async () => {
  const contextWorkspace = { ...workspace, files: [{ ...workspace.files[0], table: 'item', typeName: null }] };
  const normalInvoke = invoke.getMockImplementation()!;
  invoke.mockImplementation(async (command, args) => {
    if (command === 'authoring_workspace') return contextWorkspace;
    if (command === 'creation_context') return { roots: [{ index: 0, label: '/project', folders: [''] }], choices: { fieldTypes: ['int'], tables: ['item'], valueObjectUnderlyings: ['int'], enumUnderlyings: ['int'] } };
    return normalInvoke(command, args);
  });
  await open();
  fireEvent.click(screen.getByRole('button', { name: 'New data file' }));
  expect((await screen.findByRole('combobox', { name: 'Existing Table' })).closest('.ant-select')?.textContent).toContain('item');
  expect(screen.getByRole('combobox', { name: 'Artifact type' }).closest('.ant-select')?.textContent).toContain('Data');
});

test('Table Overview follows the selected logical Table rather than the previously active file', async () => {
  const multiTableWorkspace = { ...workspace, files: [
    { ...workspace.files[0], table: 'item', typeName: null },
    { path: 'enemy-data.yaml', sourceRoot: '.', kind: 'data', table: 'enemy', typeName: null },
  ] };
  const normalInvoke = invoke.getMockImplementation()!;
  invoke.mockImplementation(async (command, args) => {
    if (command === 'authoring_workspace') return multiTableWorkspace;
    if (command === 'table_overview') return { status: 'complete', table: args.request.table, columns: [], rows: [], totalCount: 0, selectedCount: 0, displayedCount: 0, configContentIdentity: 'config', sources: [], selection: { profile: null, includeTags: [], excludeTags: [], available: true }, diagnostics: [] };
    return normalInvoke(command, args);
  });
  await open();
  fireEvent.click(navigation().getByRole('treeitem', { name: 'enemy-data.yaml', exact: true }));
  fireEvent.click(await screen.findByRole('button', { name: 'Table Overview' }));
  await waitFor(() => expect(invoke.mock.calls.some(([command, args]) => command === 'table_overview' && args.request.table === 'enemy')).toBe(true));
});

test('empty Project offers a contextual Folder action with a folder-safe name', async () => {
  const normalInvoke = invoke.getMockImplementation()!;
  invoke.mockImplementation(async (command, args) => {
    if (command === 'authoring_workspace') return { ...workspace, files: [] };
    if (command === 'creation_context') return { roots: [{ index: 0, label: '/project', folders: [''] }], choices: { fieldTypes: ['int'], tables: [], valueObjectUnderlyings: ['int'], enumUnderlyings: ['int'] } };
    return normalInvoke(command, args);
  });
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  expect(await screen.findByRole('heading', { name: 'Start with a Table' })).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Create Folder' }));
  expect((await screen.findByRole('combobox', { name: 'Artifact type' })).closest('.ant-select')?.textContent).toContain('Folder');
  expect((screen.getByRole('textbox', { name: 'Folder name' }) as HTMLInputElement).value).toBe('new-folder');
});

test('Ant Design unsaved dialog Cancel preserves edits and does not reload', async () => {
  const input = await open();
  fireEvent.change(input, { target: { value: '20' } });
  commit(input);
  reloadProject();
  expect(await screen.findByRole('dialog')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Cancel', exact: true }));
  expect(screen.getByRole('gridcell', { name: /record 1 weight: 20/ })).toBeTruthy();
  expect(invoke.mock.calls.filter(([command]) => command === 'authoring_workspace')).toHaveLength(1);
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
  invoke.mockImplementation(async (command, args) => command === 'save_data_file'
    ? { status: 'failure', snapshot: null, current: null, diagnostic: { code: 'E-IO', message: 'Cannot write source' } }
    : normalInvoke(command, args));
  const input = await open();
  fireEvent.change(input, { target: { value: '20' } });
  commit(input);
  reloadProject();
  fireEvent.click(await screen.findByRole('button', { name: 'Save All', exact: true }));
  await waitFor(() => expect(screen.getByText('Save failed.')).toBeTruthy());
  expect(screen.getByRole('dialog')).toBeTruthy();
  expect(screen.getByRole('gridcell', { name: /record 1 weight: 20/ })).toBeTruthy();
  expect(invoke.mock.calls.filter(([command]) => command === 'authoring_workspace')).toHaveLength(1);
});

test('creation refresh selects the new source without discarding an existing dirty buffer', async () => {
  const normalInvoke = invoke.getMockImplementation()!;
  let created = false;
  invoke.mockImplementation(async (command, args) => {
    if (command === 'creation_context') return { roots: [{ index: 0, label: '/project', folders: [''] }], choices: { fieldTypes: ['int'], tables: ['item'], valueObjectUnderlyings: ['int'], enumUnderlyings: ['int'] } };
    if (command === 'create_source') { created = true; return { status: 'success', path: 'new.yaml', folder: false, diagnostic: null }; }
    if (command === 'authoring_workspace' && created) return { ...workspace, files: [...workspace.files, { path: 'new.yaml', sourceRoot: '.', kind: 'schema' }] };
    return normalInvoke(command, args);
  });
  const input = await open(); fireEvent.change(input, { target: { value: '20' } }); commit(input);
  openSourceFiles();
  fireEvent.click(screen.getByRole('button', { name: 'New source artifact' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Table' }));
  fireEvent.change(await screen.findByLabelText('Table identity'), { target: { value: 'weapon' } });
  fireEvent.click(screen.getByRole('button', { name: 'Create', exact: true }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(screen.getByRole('treeitem', { name: 'new.yaml', exact: true }).getAttribute('aria-selected')).toBe('true');
  fireEvent.click(screen.getByRole('treeitem', { name: 'data.yaml, unsaved changes', exact: true }));
  expect(screen.getByRole('gridcell', { name: /record 1 weight: 20/ })).toBeTruthy();
  expect(invoke.mock.calls.some(([command]) => command === 'save_data_file')).toBe(false);
}, APP_INTEGRATION_TEST_TIMEOUT_MS);

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
  invoke.mockImplementation(async (command, args) => {
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
  expect(invoke.mock.calls.some(([command, args]) => command === 'open_data_file' && args.relativePath === 'moved.yaml')).toBe(true);
});

test('complex table scope disables Add Row with a reason but keeps existing Delete available', async () => {
  openSnapshot = { ...mutationSnapshot(), addRow: { supported: false, reason: 'Nullable fields are outside the initial Add Row scope.' } };
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  const add = await screen.findByRole('button', { name: 'Add Row', exact: true });
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
  invoke.mockImplementation(async (command, args) => {
    if (command === 'save_data_file') return saveResponse;
    if (command === 'source_content') return sourceResponse;
    return normalInvoke(command, args);
  });

  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  fireEvent.click(await screen.findByRole('button', { name: 'Add Row', exact: true }));
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
const tableWorkspace = {...workspace,files:[...workspace.files,{path:'other.yaml',sourceRoot:'.',kind:'data'},{path:'schema.yaml',sourceRoot:'.',kind:'schema'}]};
const tablePlan = {token:'table-plan',table:'item',operation:'RenameField',field:'id',destructive:false,affectedRecordCount:1,files:[{path:'schema.yaml',before:'name: id',after:'name: itemId'},{path:'data.yaml',before:'id: 1',after:'itemId: 1'}],diagnostics:[]};
const recoveryRequired = { state:'recovery_required',files:['schema.yaml'],diagnostic:{code:'E-IO-ACCESS',message:'rollback failed'},recoveryWorkspace:'/recovery' };
async function planFromTable(type = false) {
  openSourceFiles();
  fireEvent.click(screen.getByRole('treeitem',{name:'schema.yaml',exact:true}));
  fireEvent.click(await screen.findByRole('button',{name:'Actions for field id'}));
  fireEvent.click(screen.getByRole('menuitem',{name:'Rename Field'}));
  fireEvent.change(screen.getByLabelText('Field name'),{target:{value:'itemId'}});
  fireEvent.click(screen.getByRole('button',{name:'Plan / Re-plan'}));
  await screen.findByRole('region',{name:type?'Type Migration Plan':'Migration Plan'});
}
test('Migration recovery result blocks Create and Build',async()=>{
  const normal=invoke.getMockImplementation()!;
  invoke.mockImplementation(async(command,args)=>{
    if(command==='authoring_workspace')return tableWorkspace;
    if(command==='open_table')return tableSnapshot;
    if(command==='plan_table_migration')return {...tablePlan,files:tablePlan.files.slice(0,1)};
    if(command==='apply_table_migration')return recoveryRequired;
    return normal(command,args);
  });
  const input=await open();fireEvent.change(input,{target:{value:'20'}});commit(input);
  await planFromTable();fireEvent.click(screen.getByRole('button',{name:'Apply reviewed Plan'}));
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
  invoke.mockImplementation(async(command,args)=>{
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
  expect(screen.getByRole('gridcell',{name:/record 1 weight: 10/})).toBeTruthy();
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
  invoke.mockImplementation(async (command, args) => {
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
  invoke.mockImplementation(async (command, args) => {
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
  invoke.mockImplementation((command, args) => command === 'save_data_file'
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
