import React from 'react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import App, { boundedHistoryPush } from '../src/App';
import { authoringValuesEqual, type AuthoringValue } from '../src/data-editor-types';
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
const validation = { valid: true, diagnostics: [] };
const numberShape = { name: 'weight', typeName: 'ulong', modifier: 'required', shape: { kind: 'primitive', primitive: 'ulong' } };
const snapshot = () => ({ path: 'data.yaml', table: 'item', baseSource: 'weight: 10', baseContentIdentity: 'base',
  columns: [{ name: 'weight', typeName: 'ulong', editable: true, keyField: false, shape: numberShape, readOnlyReason: null }],
  rows: [{ recordIndex: 0, cells: [{ field: 'weight', text: '10', value: { kind: 'number', value: '10' }, editable: true, readOnlyReason: null }] }], validation });
const mutationSnapshot = (rows = [{ recordIndex: 0, cells: [
  { field: 'id', text: '1', editable: false },
  { field: 'weight', text: '10', editable: true },
  { field: 'note', text: 'first', editable: true },
] }]) => ({
  path: 'data.yaml', table: 'item', baseSource: 'kind: data\ntable: item\nrecords: []\n', baseContentIdentity: 'base',
  columns: [
    { name: 'id', typeName: 'ulong', editable: false, keyField: true, shape: { name: 'id', typeName: 'ulong', modifier: 'required', shape: { kind: 'primitive', primitive: 'ulong' } }, readOnlyReason: null },
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
const complexSnapshot = (rows: any[] = [{
  recordIndex: 0,
  cells: [
    { field: 'id', text: '1', value: { kind: 'number', value: '1' }, editable: false },
    { field: 'profile', text: '', value: { kind: 'mapping', entries: [
      { name: 'credits', value: { kind: 'number', value: '18446744073709551615' } },
      { name: 'label', value: { kind: 'string', value: 'kept' } },
      { name: 'alias', value: { kind: 'string', value: 'current' } },
    ] }, editable: true },
    { field: 'tags', text: '', value: { kind: 'sequence', sourceIdentity: true, items: [
      { sourceIndex: 0, value: { kind: 'string', value: 'first' } },
      { sourceIndex: 1, value: { kind: 'string', value: 'second' } },
    ] }, editable: true },
    { field: 'status', text: 'Ready', value: { kind: 'string', value: 'Ready' }, editable: true },
    { field: 'access', text: '[None]', value: { kind: 'sequence', sourceIdentity: true, items: [
      { sourceIndex: 0, value: { kind: 'string', value: 'None' } },
    ] }, editable: true },
    { field: 'bonus', text: 'null', value: { kind: 'null' }, editable: true },
  ],
}]) => {
  const profile = { kind: 'custom', name: 'Profile', fields: [
    { name: 'credits', typeName: 'ulong', modifier: 'required', shape: { kind: 'primitive', primitive: 'ulong' } },
    { name: 'label', typeName: 'string', modifier: 'required', shape: { kind: 'primitive', primitive: 'string' } },
    { name: 'alias', typeName: 'string', modifier: 'nullable', shape: { kind: 'primitive', primitive: 'string' } },
  ] };
  const fields = [
    { name: 'id', typeName: 'ulong', editable: false, keyField: true, shape: { name: 'id', typeName: 'ulong', modifier: 'required', shape: { kind: 'primitive', primitive: 'ulong' } }, readOnlyReason: null },
    { name: 'profile', typeName: 'Profile', editable: true, keyField: false, shape: { name: 'profile', typeName: 'Profile', modifier: 'required', shape: profile }, readOnlyReason: null },
    { name: 'tags', typeName: 'string', editable: true, keyField: false, shape: { name: 'tags', typeName: 'string', modifier: 'array', shape: { kind: 'primitive', primitive: 'string' } }, readOnlyReason: null },
    { name: 'status', typeName: 'Status', editable: true, keyField: false, shape: { name: 'status', typeName: 'Status', modifier: 'required', shape: { kind: 'enum', name: 'Status', underlying: 'int', members: ['Ready', 'Paused'] } }, readOnlyReason: null },
    { name: 'access', typeName: 'Permissions', editable: true, keyField: false, shape: { name: 'access', typeName: 'Permissions', modifier: 'required', shape: { kind: 'flags', name: 'Permissions', underlying: 'int', members: ['None', 'Read', 'Write', 'Execute'] } }, readOnlyReason: null },
    { name: 'bonus', typeName: 'Profile', editable: true, keyField: false, shape: { name: 'bonus', typeName: 'Profile', modifier: 'nullable', shape: profile }, readOnlyReason: null },
  ];
  return {
    path: 'data.yaml', table: 'item', baseSource: 'kind: data\ntable: item\nrecords: []\n', baseContentIdentity: 'base',
    columns: fields,
    rows: rows.map((row) => ({ ...row, cells: row.cells.map((cell: any) => ({ ...cell, readOnlyReason: null })) })),
    addRow: { supported: true, reason: null }, validation,
  };
};
const workspace = { project: { project_root: '/project', name: 'Demo', project_id: 'demo' }, sourceRoots: ['.'],
  files: [{ path: 'data.yaml', sourceRoot: '.', kind: 'data' }],
  capabilities: { workspaceRead: true, workspaceWrite: true, validate: true, build: true } };
async function chooseOption(label: string, option: string) {
  fireEvent.mouseDown(screen.getByRole('combobox', { name: label }));
  const options = await screen.findAllByText(option, { selector: '.ant-select-item-option-content' });
  fireEvent.click(options.at(-1)!);
}
let openSnapshot: ReturnType<typeof snapshot>;
let preview: (args: any) => Promise<any>;
let pollingTimerSpy: ReturnType<typeof vi.spyOn>;
beforeEach(() => {
  pollingTimerSpy = vi.spyOn(window, 'setInterval').mockImplementation(() => 0 as any);
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
  pollingTimerSpy.mockRestore();
});
async function open(label = 'record 1 weight') { render(<App />); return await screen.findByRole('textbox', { name: label }); }

test('sequence equality treats source occurrence order as an authoring change', () => {
  const items = [
    { sourceIndex: 0, value: { kind: 'string', value: 'same' } },
    { sourceIndex: 1, value: { kind: 'string', value: 'same' } },
  ];
  const original: AuthoringValue = { kind: 'sequence', sourceIdentity: true, items };
  const moved: AuthoringValue = { kind: 'sequence', sourceIdentity: true, items: [items[1], items[0]] };
  expect(authoringValuesEqual(original, moved)).toBe(false);
  expect(authoringValuesEqual(original, structuredClone(original))).toBe(true);
});

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

test('schema-aware controls edit nested exact integers, nullable fields, arrays, Enum, and Flags', async () => {
  openSnapshot = complexSnapshot();
  let previewArgs: any;
  preview = async (args) => {
    previewArgs = args;
    return { candidateSource: 'candidate', changed: true, validation };
  };
  render(<App />);

  const credits = await screen.findByRole('textbox', { name: 'credits' }) as HTMLInputElement;
  expect(credits.value).toBe('18446744073709551615');
  fireEvent.change(screen.getByRole('textbox', { name: 'label' }), { target: { value: 'updated' } });
  fireEvent.click(screen.getByRole('button', { name: 'Move record 1 tags item 2 up' }));
  fireEvent.click(screen.getByRole('button', { name: 'Add record 1 tags item' }));
  fireEvent.change(screen.getByRole('textbox', { name: 'record 1 tags item 3' }), { target: { value: 'third' } });
  fireEvent.click(screen.getByRole('button', { name: 'Set alias to null' }));
  fireEvent.click(screen.getByRole('button', { name: 'Materialize record 1 bonus (Profile) fields' }));
  fireEvent.change(screen.getAllByRole('textbox', { name: 'credits' })[1], { target: { value: '7' } });
  await chooseOption('record 1 status', 'Paused');
  fireEvent.click(screen.getByRole('checkbox', { name: 'record 1 access Write' }));

  await waitFor(() => expect(previewArgs?.edits?.map((edit: any) => edit.field).sort())
    .toEqual(['access', 'bonus', 'profile', 'status', 'tags']));
  const edits = Object.fromEntries(previewArgs.edits.map((edit: any) => [edit.field, edit.value]));
  const profile = edits.profile;
  expect(profile.entries.find((entry: any) => entry.name === 'credits').value)
    .toEqual({ kind: 'number', value: '18446744073709551615' });
  expect(profile.entries.find((entry: any) => entry.name === 'label').value)
    .toEqual({ kind: 'string', value: 'updated' });
  expect(profile.entries.find((entry: any) => entry.name === 'alias').value)
    .toEqual({ kind: 'null' });
  expect(edits.bonus.entries.find((entry: any) => entry.name === 'credits').value)
    .toEqual({ kind: 'number', value: '7' });
  expect(edits.bonus.entries.find((entry: any) => entry.name === 'label').value)
    .toEqual({ kind: 'null' });
  expect(edits.tags.items).toEqual([
    { sourceIndex: 1, value: { kind: 'string', value: 'second' } },
    { sourceIndex: 0, value: { kind: 'string', value: 'first' } },
    { sourceIndex: null, value: { kind: 'string', value: 'third' } },
  ]);
  expect(edits.status).toEqual({ kind: 'string', value: 'Paused' });
  expect(edits.access.items).toEqual([{ sourceIndex: null, value: { kind: 'string', value: 'Write' } }]);
}, 30_000);

test('Value Object editor sends its ulong underlying as exact decimal text', async () => {
  const base = snapshot();
  openSnapshot = {
    ...base,
    columns: [{
      ...base.columns[0],
      typeName: 'ItemId',
      shape: {
        name: 'weight',
        typeName: 'ItemId',
        modifier: 'required',
        shape: { kind: 'value_object', name: 'ItemId', underlying: 'ulong' },
      },
    }],
    rows: [{ ...base.rows[0], cells: [{
      ...base.rows[0].cells[0],
      text: '18446744073709551615',
      value: { kind: 'number', value: '18446744073709551615' },
    }] }],
  } as any;
  let previewArgs: any;
  preview = async (args) => {
    previewArgs = args;
    return { candidateSource: 'candidate', changed: true, validation };
  };
  const input = await open();
  expect((input as HTMLInputElement).value).toBe('18446744073709551615');
  fireEvent.change(input, { target: { value: '18446744073709551614' } });
  await waitFor(() => expect(previewArgs?.edits?.[0]?.value)
    .toEqual({ kind: 'number', value: '18446744073709551614' }));
});

test('Add Row starts with null placeholders and materializes complex values only after explicit choices', async () => {
  openSnapshot = complexSnapshot([]);
  let previewArgs: any;
  preview = async (args) => {
    previewArgs = args;
    return { candidateSource: openSnapshot.baseSource, changed: true, validation };
  };
  render(<App />);
  fireEvent.click(await screen.findByRole('button', { name: 'Add Row', exact: true }));
  await screen.findByRole('textbox', { name: 'new record id' });
  await waitFor(() => expect(previewArgs?.addedRecords).toHaveLength(1));
  expect(previewArgs.addedRecords[0].fields.map((field: any) => field.value.kind))
    .toEqual(['null', 'null', 'null', 'null', 'null', 'null']);
  expect(screen.getByText('No flags value selected yet.')).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: 'Materialize new record profile (Profile) fields' }));
  fireEvent.click(screen.getByRole('button', { name: 'Make empty array for new record tags' }));
  await chooseOption('new record status', 'Paused');
  fireEvent.click(screen.getByRole('checkbox', { name: 'new record access Write' }));
  await waitFor(() => {
    const values = Object.fromEntries(previewArgs?.addedRecords?.[0]?.fields?.map((field: any) => [field.field, field.value]) ?? []);
    expect(values.profile?.kind).toBe('mapping');
    expect(values.status).toEqual({ kind: 'string', value: 'Paused' });
    expect(values.access).toEqual({ kind: 'sequence', sourceIdentity: true, items: [{ sourceIndex: null, value: { kind: 'string', value: 'Write' } }] });
  });

  const fields = Object.fromEntries(previewArgs.addedRecords[0].fields.map((field: any) => [field.field, field.value]));
  expect(fields.profile.entries.map((entry: any) => entry.value.kind)).toEqual(['null', 'null', 'null']);
  expect(fields.tags).toEqual({ kind: 'sequence', sourceIdentity: true, items: [] });
  expect(fields.status).toEqual({ kind: 'string', value: 'Paused' });
  expect(fields.access).toEqual({ kind: 'sequence', sourceIdentity: true, items: [{ sourceIndex: null, value: { kind: 'string', value: 'Write' } }] });
  expect(fields.bonus).toEqual({ kind: 'null' });
}, 15_000);

test('nested value diagnostics focus the matching Custom Type field', async () => {
  openSnapshot = complexSnapshot();
  openSnapshot.validation = {
    valid: false,
    diagnostics: [{
      code: 'E-TABLE-INVALID-RECORD-VALUE',
      source: '/project/data.yaml',
      record_identity: 'record[0]',
      value_path: '/credits',
      message: 'field `profile` is invalid',
    }],
  } as any;
  render(<App />);
  const credits = await screen.findByRole('textbox', { name: 'credits' });
  fireEvent.click(screen.getByRole('button', { name: /E-TABLE-INVALID-RECORD-VALUE/ }));
  await waitFor(() => expect(document.activeElement).toBe(credits));
  expect(credits.getAttribute('aria-invalid')).toBe('true');
}, 10_000);

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
}, 10_000);

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

const tableSnapshot = {path:'schema.yaml',schema:{table:'item',fields:[{key:0,name:'id',type:'int',nullable:false,array:false}],primaryKey:{fields:['id']},secondaryKeys:[]},fieldTypes:['int','string']};
const tableWorkspace = {...workspace,files:[...workspace.files,{path:'other.yaml',sourceRoot:'.',kind:'data'},{path:'schema.yaml',sourceRoot:'.',kind:'schema'}]};
const tablePlan = {token:'table-plan',table:'item',operation:'RenameField',field:'id',destructive:false,affectedRecordCount:1,files:[{path:'schema.yaml',before:'name: id',after:'name: itemId'},{path:'data.yaml',before:'id: 1',after:'itemId: 1'}],diagnostics:[]};
async function planFromTable(type = false) {
  fireEvent.click(screen.getByRole('treeitem',{name:'schema.yaml',exact:true}));
  fireEvent.click(await screen.findByRole('button',{name:'Rename Field',exact:true}));
  fireEvent.change(screen.getByLabelText('Field name'),{target:{value:'itemId'}});
  fireEvent.click(screen.getByRole('button',{name:'Plan / Re-plan'}));
  await screen.findByRole('region',{name:type?'Type Migration Plan':'Migration Plan'});
}
test.each(['table','type'])('%s Migration refresh reloads affected clean editors and preserves unrelated dirty buffers',async(kind)=>{
  const normal=invoke.getMockImplementation()!;
  invoke.mockImplementation(async(command,args)=>{
    if(command==='authoring_workspace')return {...tableWorkspace,files:tableWorkspace.files.map(f=>f.kind==='schema'&&kind==='type'?{...f,kind:'type'}:f)};
    if(command==='open_type')return {path:'schema.yaml',name:'Reward',category:'Custom Type',fields:tableSnapshot.schema.fields,members:[],conversions:null,underlying:null,fieldTypes:['int']};
    if(command==='plan_type_migration')return {...tablePlan,target:'Reward',selector:'id',affectedOccurrenceCount:1,files:tablePlan.files};
    if(command==='open_table')return tableSnapshot;
    if(command==='plan_table_migration')return tablePlan;
    if(command==='apply_table_migration')return {state:'success',files:['schema.yaml','data.yaml']};
    if(command==='open_data_file')return {...snapshot(),path:args.relativePath};
    return normal(command,args);
  });
  await open();
  fireEvent.click(screen.getByRole('treeitem',{name:'other.yaml',exact:true}));
  const input=await screen.findByRole('textbox',{name:'record 1 weight'});
  fireEvent.change(input,{target:{value:'20'}});
  await planFromTable(kind==='type');
  fireEvent.click(screen.getByRole('button',{name:'Apply reviewed Plan'}));
  await waitFor(()=>expect(invoke.mock.calls.filter(([command,args])=>command==='open_data_file'&&args.relativePath==='data.yaml')).toHaveLength(2));
  fireEvent.click(screen.getByRole('treeitem',{name:'other.yaml, unsaved changes',exact:true}));
  expect((screen.getByRole('textbox',{name:'record 1 weight'}) as HTMLInputElement).value).toBe('20');
  expect(invoke.mock.calls.filter(([command,args])=>command==='open_data_file'&&args.relativePath==='other.yaml')).toHaveLength(1);
  expect(invoke.mock.calls.some(([command])=>command==='save_data_file')).toBe(false);
}, 20_000);
const recoveryRequired = { state:'recovery_required',files:['schema.yaml'],diagnostic:{code:'E-IO-ACCESS',message:'rollback failed'},recoveryWorkspace:'/recovery' };
test.each(['table','type'])('%s Migration recovery result blocks Create and Build',async(kind)=>{
  const normal=invoke.getMockImplementation()!;
  invoke.mockImplementation(async(command,args)=>{
    if(command==='authoring_workspace')return {...tableWorkspace,files:tableWorkspace.files.map(f=>f.kind==='schema'&&kind==='type'?{...f,kind:'type'}:f)};
    if(command==='open_type')return {path:'schema.yaml',name:'Reward',category:'Custom Type',fields:tableSnapshot.schema.fields,members:[],conversions:null,underlying:null,fieldTypes:['int']};
    if(command==='plan_type_migration')return {...tablePlan,target:'Reward',selector:'id',affectedOccurrenceCount:1,files:tablePlan.files.slice(0,1)};
    if(command==='open_table')return tableSnapshot;
    if(command==='plan_table_migration')return {...tablePlan,files:tablePlan.files.slice(0,1)};
    if(command==='apply_table_migration')return recoveryRequired;
    return normal(command,args);
  });
  const input=await open();fireEvent.change(input,{target:{value:'20'}});
  await planFromTable(kind==='type');fireEvent.click(screen.getByRole('button',{name:'Apply reviewed Plan'}));
  await screen.findByText('Recovery Required — source changes and Build are blocked');
  expect((screen.getByRole('button',{name:'New source artifact'}) as HTMLButtonElement).disabled).toBe(true);
  expect((screen.getByRole('button',{name:'Build',exact:true}) as HTMLButtonElement).disabled).toBe(true);
}, 20_000);
test('Recovery Required blocks Save until host recheck succeeds',async()=>{
  const normal=invoke.getMockImplementation()!;
  let recovery:any=recoveryRequired;
  invoke.mockImplementation(async(command,args)=>{
    if(command==='authoring_workspace')return tableWorkspace;
    if(command==='migration_recovery_status')return recovery;
    if(command==='recheck_migration'){recovery=null;return null;}
    return normal(command,args);
  });
  const input=await open();fireEvent.change(input,{target:{value:'20'}});
  await screen.findByText('Recovery Required — source changes and Build are blocked');
  expect((screen.getByRole('button',{name:'New source artifact'}) as HTMLButtonElement).disabled).toBe(true);
  expect((screen.getByRole('button',{name:'Build',exact:true}) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole('treeitem',{name:'data.yaml, unsaved changes',exact:true}));
  expect(screen.getAllByRole('button',{name:'Save',exact:true}).every(button=>(button as HTMLButtonElement).disabled)).toBe(true);
  fireEvent.keyDown(window,{key:'s',metaKey:true});
  expect(invoke.mock.calls.some(([command])=>command==='save_data_file')).toBe(false);
  fireEvent.click(screen.getByRole('button',{name:'Recheck recovered source'}));
  await waitFor(()=>expect((screen.getByRole('button',{name:'Build',exact:true}) as HTMLButtonElement).disabled).toBe(false));
  expect((screen.getByRole('textbox',{name:'record 1 weight'}) as HTMLInputElement).value).toBe('20');
}, 20_000);


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
  fireEvent.click(screen.getByRole('button', { name: 'Settings', exact: true }));
  await screen.findByRole('heading', { name: 'Project Settings' });
  fireEvent.change(screen.getByLabelText('Include tags'), { target: { value: 'new' } });
  fireEvent.keyDown(window, { key: 's', ctrlKey: true });

  await waitFor(() => expect(invoke.mock.calls.some(([command]) => command === 'save_project_config_edit')).toBe(true));
  expect(invoke.mock.calls.some(([command]) => command === 'save_data_file')).toBe(false);
});


test('2x2 Paste derives a 2x2 target rectangle from the active cell instead of flattening the selection', async () => {
  openSnapshot = mutationSnapshot([
    { recordIndex: 0, cells: [
      { field: 'id', text: '1', editable: false },
      { field: 'weight', text: '10', editable: true },
      { field: 'note', text: 'a', editable: true },
    ] },
    { recordIndex: 1, cells: [
      { field: 'id', text: '2', editable: false },
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

  const input = await open('record 1 weight');
  const cell = input.closest('.cell-wrap');
  expect(cell).toBeTruthy();
  fireEvent.mouseDown(cell!, { button: 0 });
  fireEvent.keyDown(input, { key: 'v', ctrlKey: true });

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

test('Undo history warns before discarding the oldest entry and retains the newest 50 states', () => {
  const alert = vi.spyOn(window, 'alert').mockImplementation(() => {});
  const state = (index: number) => ({
    edits: { [`cell-${index}`]: { recordIndex: index, field: 'weight', value: { kind: 'number', value: String(index) } } },
    addedRecords: [],
    pendingDeletes: [],
    tagEdits: {},
  }) as any;
  const history = Array.from({ length: 50 }, (_, index) => state(index));
  const next = boundedHistoryPush(history, state(50));
  expect(alert).toHaveBeenCalledOnce();
  expect(next).toHaveLength(50);
  expect(next[0]).toEqual(state(1));
  expect(next.at(-1)).toEqual(state(50));
});
