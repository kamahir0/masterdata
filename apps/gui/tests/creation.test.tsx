import React from 'react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import SourceCreation from '../src/SourceCreation';
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
const context = { roots: [{ index: 0, label: '/project/sources', folders: ['', 'data'] }], choices: { fieldTypes: ['int', 'ulong', 'ItemId'], tables: ['item'], valueObjectUnderlyings: ['int', 'string'], enumUnderlyings: ['int', 'ulong'] } };
let result: any;
const onCreated = vi.fn(async () => {});
const onCancel = vi.fn();
beforeEach(() => {
  result = { status: 'success', path: 'sources/new.yaml', folder: false, diagnostic: null };
  onCreated.mockClear(); onCancel.mockClear(); invoke.mockReset();
  invoke.mockImplementation(async (command) => {
    if (command === 'creation_context') return context;
    if (command === 'create_source') return result;
    if (command === 'recheck_creation') return { exists: true, folder: false, source: 'actual source' };
    throw new Error(command);
  });
});
afterEach(cleanup);
async function open(canWrite = true) {
  render(<SourceCreation projectPath="/project" initialRootIndex={0} initialFolder="" canWrite={canWrite} onCreated={onCreated} onCancel={onCancel} />);
  await screen.findByLabelText('Table identity');
}
async function choose(label: string, option: string) {
  fireEvent.mouseDown(screen.getByRole('combobox', { name: label }));
  const options = await screen.findAllByText(option, { selector: '.ant-select-item-option-content' });
  fireEvent.click(options.at(-1)!);
}
const createCalls = () => invoke.mock.calls.filter(([command]) => command === 'create_source');

test('guided Table submit keeps destination independent and sends explicit ordered fields and keys', async () => {
  await open();
  fireEvent.change(screen.getByLabelText('Table identity'), { target: { value: 'weapon' } });
  fireEvent.change(screen.getByLabelText('Filename (.yaml / .yml)'), { target: { value: 'unrelated.yml' } });
  fireEvent.click(screen.getByRole('button', { name: 'Add field', exact: true }));
  fireEvent.change(screen.getByLabelText('Field 2 name'), { target: { value: 'count' } });
  fireEvent.click(screen.getByRole('button', { name: 'Field 2 up', exact: true }));
  fireEvent.click(screen.getByRole('button', { name: 'Create', exact: true }));
  await waitFor(() => expect(onCreated).toHaveBeenCalledOnce());
  const request = createCalls()[0][1].request;
  expect(request.destination).toBe('unrelated.yml');
  expect(request.artifact.table).toBe('weapon');
  expect(request.artifact.fields.map((field: any) => field.name)).toEqual(['count', 'id']);
  expect(request.artifact.primaryKey.fields).toEqual(['id']);
});

test('Enum creation transports the full ulong value as text', async () => {
  await open(); await choose('Artifact type', 'Enum'); await choose('Underlying', 'ulong');
  fireEvent.change(screen.getByLabelText('Type name'), { target: { value: 'Kind' } });
  fireEvent.change(screen.getByLabelText('Member 1 name'), { target: { value: 'Max' } });
  fireEvent.change(screen.getByLabelText('Member 1 value'), { target: { value: '18446744073709551615' } });
  fireEvent.click(screen.getByRole('button', { name: 'Create', exact: true }));
  await waitFor(() => expect(onCreated).toHaveBeenCalledOnce());
  expect(createCalls()[0][1].request.artifact.members[0].value).toBe('18446744073709551615');
});

test('Conflict preserves input, exposes no Overwrite and Cancel does not create again', async () => {
  result = { status: 'conflict', diagnostic: { code: 'E-SOURCE-CREATE-CONFLICT', message: 'exists' } };
  await open(); fireEvent.change(screen.getByLabelText('Table identity'), { target: { value: 'weapon' } });
  fireEvent.click(screen.getByRole('button', { name: 'Create', exact: true }));
  await screen.findByRole('alert');
  expect((screen.getByLabelText('Table identity') as HTMLInputElement).value).toBe('weapon');
  expect(screen.queryByRole('button', { name: 'Overwrite' })).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Cancel', exact: true }));
  expect(onCancel).toHaveBeenCalledOnce(); expect(createCalls()).toHaveLength(1);
});

test('Outcome Unknown disables retry until the submitted destination is rechecked', async () => {
  result = { status: 'outcome_unknown', diagnostic: { code: 'E-UNKNOWN', message: 'unknown' } };
  await open(); fireEvent.click(screen.getByRole('button', { name: 'Create', exact: true }));
  await screen.findByRole('alert');
  expect((screen.getByRole('button', { name: 'Create', exact: true }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.change(screen.getByLabelText('Filename (.yaml / .yml)'), { target: { value: 'different.yaml' } });
  fireEvent.click(screen.getByRole('button', { name: 'Recheck Workspace' }));
  await screen.findByRole('status');
  expect(invoke.mock.calls.find(([command]) => command === 'recheck_creation')?.[1].request.destination).toBe('new.yaml');
  expect(createCalls()).toHaveLength(1);
});

test('Cancel and reopen cannot bypass recheck after an uncertain commit', async () => {
  result = { status: 'outcome_unknown', diagnostic: { code: 'E-UNKNOWN', message: 'unknown' } };
  await open();
  fireEvent.change(screen.getByLabelText('Filename (.yaml / .yml)'), { target: { value: 'original.yaml' } });
  fireEvent.click(screen.getByRole('button', { name: 'Create', exact: true }));
  await screen.findByRole('alert');
  fireEvent.click(screen.getByRole('button', { name: 'Cancel', exact: true }));
  cleanup(); await open();
  expect((screen.getByRole('button', { name: 'Create', exact: true }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole('button', { name: 'Recheck Workspace' }));
  await screen.findByRole('status');
  expect(invoke.mock.calls.find(([command]) => command === 'recheck_creation')?.[1].request.destination).toBe('original.yaml');
  expect(createCalls()).toHaveLength(1);
});

test('missing write capability prevents creation', async () => {
  await open(false);
  expect((screen.getByRole('button', { name: 'Create', exact: true }) as HTMLButtonElement).disabled).toBe(true);
  expect(createCalls()).toHaveLength(0);
});

test('Create prevents double submit while the exclusive operation is pending', async () => {
  let finish!: (value: any) => void;
  invoke.mockImplementation(async command => {
    if (command === 'creation_context') return context;
    if (command === 'create_source') return new Promise(resolve => { finish = resolve; });
  });
  await open();
  const button = screen.getByRole('button', { name: 'Create', exact: true });
  fireEvent.click(button); fireEvent.click(button);
  expect(createCalls()).toHaveLength(1);
  finish(result);
  await waitFor(() => expect(onCreated).toHaveBeenCalledOnce());
});
