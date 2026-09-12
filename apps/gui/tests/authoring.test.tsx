import React from 'react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import App from '../src/App';
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
const validation = { valid: true, diagnostics: [] };
const snapshot = () => ({ path: 'data.yaml', table: 'item', baseSource: 'weight: 10', baseContentIdentity: 'base',
  columns: [{ name: 'weight', typeName: 'ulong', editable: true, keyField: false }],
  rows: [{ recordIndex: 0, cells: [{ field: 'weight', text: '10', editable: true }] }], validation });
const mutationSnapshot = (rows = [{ recordIndex: 0, cells: [
  { field: 'id', text: '1', editable: false },
  { field: 'weight', text: '10', editable: true },
  { field: 'note', text: 'first', editable: true },
] }]) => ({
  path: 'data.yaml', table: 'item', baseSource: 'kind: data\ntable: item\nrecords: []\n', baseContentIdentity: 'base',
  columns: [
    { name: 'id', typeName: 'ulong', editable: false, keyField: true },
    { name: 'weight', typeName: 'ulong', editable: true, keyField: false },
    { name: 'note', typeName: 'string', editable: true, keyField: false },
  ],
  rows, addRow: { supported: true, reason: null }, validation,
});
const workspace = { project: { project_root: '/project', name: 'Demo', project_id: 'demo' }, sourceRoots: ['.'],
  files: [{ path: 'data.yaml', sourceRoot: '.', kind: 'data' }],
  capabilities: { workspaceRead: true, workspaceWrite: true, validate: true, build: true } };
let openSnapshot: ReturnType<typeof snapshot>;
let preview: (args: any) => Promise<any>;
beforeEach(() => {
  openSnapshot = snapshot();
  preview = async () => ({ candidateSource: 'weight: 20', changed: true, validation });
  invoke.mockReset();
  invoke.mockImplementation(async (command, args) => {
    if (command === 'authoring_workspace') return workspace;
    if (command === 'open_data_file') return structuredClone(openSnapshot);
    if (command === 'preview_data_file') return preview(args);
    if (command === 'save_data_file') return { status: 'success', snapshot: snapshot() };
    if (command === 'source_content') return { contentIdentity: 'base', source: 'weight: 10' };
    if (command === 'build') return { generatedFiles: [] };
    throw new Error(`Unexpected command: ${command}`);
  });
});
afterEach(cleanup);
async function open() { render(<App />); return await screen.findByRole('textbox', { name: 'record 1 weight' }); }

test('late pre-save no-op preview cannot discard a new edit after Save resets revision', async () => {
  let resolveOld!: (value: unknown) => void;
  preview = () => new Promise(resolve => { resolveOld = resolve; });
  const input = await open();
  fireEvent.change(input, { target: { value: '10 ' } });
  await waitFor(() => expect(resolveOld).toBeDefined());
  fireEvent.click(screen.getAllByRole('button', { name: 'Save', exact: true })[0]);
  await waitFor(() => expect((screen.getByRole('textbox', { name: 'record 1 weight' }) as HTMLInputElement).value).toBe('10'));
  fireEvent.change(screen.getByRole('textbox', { name: 'record 1 weight' }), { target: { value: '20' } });
  await act(async () => resolveOld({ changed: false, candidateSource: 'weight: 10', validation }));
  expect((screen.getByRole('textbox', { name: 'record 1 weight' }) as HTMLInputElement).value).toBe('20');
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

test('Ant Design unsaved dialog Cancel preserves edits and does not reload', async () => {
  const input = await open();
  fireEvent.change(input, { target: { value: '20' } });
  fireEvent.click(screen.getByRole('button', { name: 'Reload', exact: true }));
  expect(await screen.findByRole('dialog')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Cancel', exact: true }));
  expect((input as HTMLInputElement).value).toBe('20');
  expect(invoke.mock.calls.filter(([command]) => command === 'authoring_workspace')).toHaveLength(1);
});

test('dirty Build invokes only saved-source Build and preserves exact 64-bit input', async () => {
  const input = await open();
  fireEvent.change(input, { target: { value: '18446744073709551615' } });
  fireEvent.click(screen.getByRole('button', { name: 'Build', exact: true }));
  await waitFor(() => expect(invoke.mock.calls.some(([command]) => command === 'build')).toBe(true));
  expect(invoke.mock.calls.some(([command]) => command === 'save_data_file')).toBe(false);
  expect((input as HTMLInputElement).value).toBe('18446744073709551615');
});

test('current preview updates the Diff after old snapshot responses are rejected', async () => {
  const input = await open();
  fireEvent.change(input, { target: { value: '20' } });
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
  fireEvent.click(screen.getByRole('button', { name: 'Reload', exact: true }));
  fireEvent.click(await screen.findByRole('button', { name: 'Save All', exact: true }));
  await waitFor(() => expect(screen.getByText('Save failed.')).toBeTruthy());
  expect(screen.getByRole('dialog')).toBeTruthy();
  expect((input as HTMLInputElement).value).toBe('20');
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
  const input = await open(); fireEvent.change(input, { target: { value: '20' } });
  fireEvent.click(screen.getByRole('button', { name: 'New source artifact' }));
  fireEvent.change(await screen.findByLabelText('Table identity'), { target: { value: 'weapon' } });
  fireEvent.click(screen.getByRole('button', { name: 'Create', exact: true }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(screen.getByRole('treeitem', { name: 'new.yaml', exact: true }).getAttribute('aria-selected')).toBe('true');
  fireEvent.click(screen.getByRole('treeitem', { name: 'data.yaml, unsaved changes', exact: true }));
  expect((screen.getByRole('textbox', { name: 'record 1 weight' }) as HTMLInputElement).value).toBe('20');
  expect(invoke.mock.calls.some(([command]) => command === 'save_data_file')).toBe(false);
}, 10_000);

test('Add Row creates an editable draft, validates it through the shared preview, and makes saved keys read-only', async () => {
  openSnapshot = mutationSnapshot([]);
  let previewArgs: any;
  preview = async (args) => {
    previewArgs = args;
    return {
      candidateSource: 'kind: data\ntable: item\nrecords:\n  - id: 18446744073709551615\n    weight: invalid\n    note: draft\n',
      changed: true,
      validation: {
        valid: false,
        diagnostics: [{ code: 'E-TABLE-INVALID-RECORD-VALUE', source: '/project/data.yaml', record_identity: 'record[0]', message: 'field `weight` is invalid' }],
      },
    };
  };
  const normalInvoke = invoke.getMockImplementation()!;
  invoke.mockImplementation(async (command, args) => command === 'save_data_file'
    ? { status: 'success', snapshot: mutationSnapshot([
        { recordIndex: 0, cells: [
          { field: 'id', text: '1', editable: false },
          { field: 'weight', text: '10', editable: true },
          { field: 'note', text: 'first', editable: true },
        ] },
        { recordIndex: 1, cells: [
          { field: 'id', text: '18446744073709551615', editable: false },
          { field: 'weight', text: 'invalid', editable: true },
          { field: 'note', text: 'draft', editable: true },
        ] },
      ]) }
    : normalInvoke(command, args));

  render(<App />);
  const add = await screen.findByRole('button', { name: 'Add Row', exact: true });
  fireEvent.click(add);
  const id = await screen.findByRole('textbox', { name: 'new record id' }) as HTMLInputElement;
  expect(id.readOnly).toBe(false);
  await waitFor(() => expect(document.activeElement).toBe(id));
  fireEvent.change(id, { target: { value: '18446744073709551615' } });
  const weight = screen.getByRole('textbox', { name: 'new record weight' }) as HTMLInputElement;
  fireEvent.change(weight, { target: { value: 'invalid' } });
  fireEvent.change(screen.getByRole('textbox', { name: 'new record note' }), { target: { value: 'draft' } });
  await waitFor(() => expect(previewArgs?.addedRecords?.[0]?.fields.map((field: any) => field.field)).toEqual(['id', 'weight', 'note']));
  await waitFor(() => expect(weight.closest('td')?.classList.contains('invalid')).toBe(true));
  expect(invoke.mock.calls.some(([command]) => command === 'save_data_file')).toBe(false);

  fireEvent.click(screen.getAllByRole('button', { name: 'Save', exact: true })[0]);
  await waitFor(() => expect(screen.getByRole('textbox', { name: 'record 2 id' })).toBeTruthy());
  expect((screen.getByRole('textbox', { name: 'record 2 id' }) as HTMLInputElement).readOnly).toBe(true);
}, 20_000);

test('deleting a new draft cancels the addition and returns the file to clean', async () => {
  openSnapshot = mutationSnapshot([]);
  preview = async (args) => ({ candidateSource: openSnapshot.baseSource, changed: Boolean(args.addedRecords?.length || args.deletedRecordIndices?.length), validation });
  render(<App />);
  fireEvent.click(await screen.findByRole('button', { name: 'Add Row', exact: true }));
  await screen.findByRole('textbox', { name: 'new record id' });
  fireEvent.click(screen.getByRole('button', { name: 'Delete new row 1', exact: true }));
  await waitFor(() => expect(screen.queryByRole('textbox', { name: 'new record id' })).toBeNull());
  expect(screen.getByText('Saved')).toBeTruthy();
  expect(invoke.mock.calls.some(([command]) => command === 'save_data_file')).toBe(false);
}, 10_000);

test('shared no-op preview normalizes structural mutation state back to clean', async () => {
  openSnapshot = mutationSnapshot();
  preview = async () => ({ candidateSource: openSnapshot.baseSource, changed: false, validation });
  render(<App />);

  fireEvent.click(await screen.findByRole('button', { name: 'Delete record 1', exact: true }));
  expect(screen.getByText('Pending delete')).toBeTruthy();
  await waitFor(() => expect(screen.getByText('Saved')).toBeTruthy());
  expect(screen.queryByText('Pending delete')).toBeNull();
  expect((screen.getByRole('textbox', { name: 'record 1 weight' }) as HTMLInputElement).readOnly).toBe(false);

  fireEvent.click(screen.getByRole('button', { name: 'Add Row', exact: true }));
  expect(await screen.findByRole('textbox', { name: 'new record id' })).toBeTruthy();
  await waitFor(() => expect(screen.getByText('Saved')).toBeTruthy());
  expect(screen.queryByRole('textbox', { name: 'new record id' })).toBeNull();
}, 10_000);

test('deleting an edited existing row preserves the edit while Undo restores editability', async () => {
  openSnapshot = mutationSnapshot([
    { recordIndex: 0, cells: [
      { field: 'id', text: '1', editable: false },
      { field: 'weight', text: '10', editable: true },
      { field: 'note', text: 'first', editable: true },
    ] },
    { recordIndex: 1, cells: [
      { field: 'id', text: '2', editable: false },
      { field: 'weight', text: '20', editable: true },
      { field: 'note', text: 'second', editable: true },
    ] },
  ]);
  render(<App />);
  const weight = await screen.findByRole('textbox', { name: 'record 1 weight' }) as HTMLInputElement;
  fireEvent.change(weight, { target: { value: '11' } });
  fireEvent.click(screen.getByRole('button', { name: 'Delete record 1', exact: true }));
  expect(screen.getByText('Pending delete')).toBeTruthy();
  expect(weight.value).toBe('11');
  expect(weight.readOnly).toBe(true);
  fireEvent.click(screen.getByRole('button', { name: 'Undo Delete record 1', exact: true }));
  expect(weight.value).toBe('11');
  expect(weight.readOnly).toBe(false);
});

test('complex table scope disables Add Row with a reason but keeps existing Delete available', async () => {
  openSnapshot = { ...mutationSnapshot(), addRow: { supported: false, reason: 'Nullable fields are outside the initial Add Row scope.' } };
  render(<App />);
  const add = await screen.findByRole('button', { name: 'Add Row', exact: true });
  expect((add as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByText('Nullable fields are outside the initial Add Row scope.')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Delete record 1', exact: true })).toBeTruthy();
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

  render(<App />);
  fireEvent.click(await screen.findByRole('button', { name: 'Add Row', exact: true }));
  const draftId = await screen.findByRole('textbox', { name: 'new record id' }) as HTMLInputElement;
  fireEvent.change(draftId, { target: { value: '1' } });
  fireEvent.click(screen.getAllByRole('button', { name: 'Save', exact: true })[0]);
  await waitFor(() => expect(screen.getByText('Save failed.')).toBeTruthy());
  expect(screen.getByRole('textbox', { name: 'new record id' })).toBeTruthy();

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
  expect(screen.getByRole('textbox', { name: 'new record id' })).toBeTruthy();

  saveResponse = {
    status: 'outcome_unknown',
    snapshot: null,
    current: null,
    diagnostic: { code: 'E-SOURCE-EDIT-WRITE-VERIFY', message: 'could not verify write' },
  };
  fireEvent.click(screen.getByRole('button', { name: 'Overwrite', exact: true }));
  await waitFor(() => expect(screen.getByText('Previous save outcome is unknown.')).toBeTruthy());
  expect(screen.getByRole('textbox', { name: 'new record id' })).toBeTruthy();
}, 20_000);
