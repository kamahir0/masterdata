import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import ComputedViewEditor from '../src/ComputedViewEditor';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

const baseSnapshot = {
  path: 'sources/item-view.yaml',
  baseSource: 'kind: view\n',
  baseContentIdentity: 'sha256:base',
  name: 'display',
  table: 'item',
  columns: [{ name: 'label', expression: 'name + "!"' }],
  diagnostics: [],
};

beforeEach(() => {
  invoke.mockReset();
  invoke.mockImplementation(async (command: string, args: any) => {
    if (command === 'open_computed_view') return structuredClone(baseSnapshot);
    if (command === 'preview_computed_view') {
      return {
        candidateSource: 'kind: view\n',
        candidateContentIdentity: 'sha256:candidate',
        changed: true,
        validation: { valid: true, diagnostics: [] },
      };
    }
    if (command === 'save_computed_view') {
      return {
        status: 'success',
        snapshot: { ...structuredClone(baseSnapshot), columns: args.request.columns },
        current: null,
        diagnostic: null,
      };
    }
    throw new Error(`unexpected command: ${command}`);
  });
});

afterEach(cleanup);

test('Computed View editor delegates preview and save without frontend YAML semantics', async () => {
  const onSaved = vi.fn();
  render(<ComputedViewEditor projectPath="/project" path={baseSnapshot.path} canWrite onSaved={onSaved} />);

  await screen.findByRole('heading', { name: 'display' });
  fireEvent.change(screen.getByLabelText('Computed column 1 expression'), { target: { value: 'name + "?"' } });
  fireEvent.click(screen.getByRole('button', { name: 'Preview' }));
  await screen.findByRole('region', { name: 'Computed View preview' });

  const previewCall = invoke.mock.calls.find(([command]) => command === 'preview_computed_view');
  expect(previewCall?.[1].request).toEqual({
    name: 'display',
    table: 'item',
    columns: [{ name: 'label', expression: 'name + "?"' }],
  });

  await waitFor(() => expect(screen.getByRole('button', { name: 'Save' }).className).not.toContain('ant-btn-loading'));
  fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  await waitFor(() => expect(onSaved).toHaveBeenCalledOnce());
  const saveCall = invoke.mock.calls.find(([command]) => command === 'save_computed_view');
  expect(saveCall?.[1].baseContentIdentity).toBe('sha256:base');
  expect(saveCall?.[1].request.columns[0].expression).toBe('name + "?"');
});
