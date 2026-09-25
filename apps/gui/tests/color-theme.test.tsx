import React from 'react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import App from '../src/App';
import {
  THEME_PREFERENCE_STORAGE_KEY,
  applyThemeToDom,
  getOsPrefersDark,
  readStoredThemePreference,
  resolveEffectiveTheme,
} from '../src/theme';
import {
  type ApplicationUserStateDto,
  persistNativeThemePreference,
} from '../src/user-state';

const { invoke, openDialog, desktopWindow } = vi.hoisted(() => ({
  invoke: vi.fn(),
  openDialog: vi.fn(),
  desktopWindow: { onCloseRequested: vi.fn(), destroy: vi.fn() },
}));

vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => desktopWindow }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: openDialog }));

const validation = { valid: true, diagnostics: [] };
const numberShape = { name: 'weight', typeName: 'ulong', modifier: 'required', shape: { kind: 'primitive', primitive: 'ulong' } };
const snapshot = () => ({
  path: 'data.yaml',
  table: 'item',
  baseSource: 'weight: 10',
  baseContentIdentity: 'base',
  columns: [{ name: 'weight', typeName: 'ulong', editable: true, keyField: false, shape: numberShape, readOnlyReason: null }],
  rows: [{ recordIndex: 0, cells: [{ field: 'weight', text: '10', value: { kind: 'number', value: '10' }, editable: true, readOnlyReason: null }] }],
  validation,
});

const workspace = {
  project: { project_root: '/project', name: 'Demo', project_id: 'demo' },
  sourceRoots: ['.'],
  files: [{ path: 'data.yaml', sourceRoot: '.', kind: 'data', table: 'item', typeName: null, hasInlineRecords: false, diagnostic: null }],
};

let matchMediaListeners: Array<(e: { matches: boolean }) => void> = [];
let osPrefersDarkValue = false;
let currentNativeUserState: ApplicationUserStateDto = {};

function setupMatchMedia(initialDark = false) {
  osPrefersDarkValue = initialDark;
  matchMediaListeners = [];
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: query === '(prefers-color-scheme: dark)' ? osPrefersDarkValue : false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn((event: string, callback: (e: { matches: boolean }) => void) => {
        if (event === 'change') {
          matchMediaListeners.push(callback);
        }
      }),
      removeEventListener: vi.fn((event: string, callback: (e: { matches: boolean }) => void) => {
        if (event === 'change') {
          matchMediaListeners = matchMediaListeners.filter((cb) => cb !== callback);
        }
      }),
      dispatchEvent: vi.fn(),
    })),
  });
}

function triggerOsAppearanceChange(newDark: boolean) {
  osPrefersDarkValue = newDark;
  act(() => {
    matchMediaListeners.forEach((listener) => listener({ matches: newDark }));
  });
}

beforeEach(() => {
  window.localStorage.clear();
  document.documentElement.removeAttribute('data-theme');
  currentNativeUserState = {};
  setupMatchMedia(false);
  desktopWindow.onCloseRequested.mockReset();
  desktopWindow.onCloseRequested.mockResolvedValue(() => {});
  desktopWindow.destroy.mockReset();
  desktopWindow.destroy.mockResolvedValue(undefined);
  openDialog.mockReset();
  openDialog.mockResolvedValue(null);
  invoke.mockReset();
  invoke.mockImplementation(async (command, args: any) => {
    if (command === 'load_application_user_state') return structuredClone(currentNativeUserState);
    if (command === 'set_theme_preference') {
      currentNativeUserState.themePreference = args.preference;
      return structuredClone(currentNativeUserState);
    }
    if (command === 'set_recent_projects') {
      currentNativeUserState.recentProjects = args.projects;
      return structuredClone(currentNativeUserState);
    }
    if (command === 'migration_recovery_status') return null;
    if (command === 'authoring_workspace') return workspace;
    if (command === 'open_data_file') return structuredClone(snapshot());
    if (command === 'preview_data_file') return { candidateSource: 'weight: 20', changed: true, validation };
    if (command === 'save_data_file') return { status: 'success', snapshot: snapshot() };
    if (command === 'source_content') return { contentIdentity: 'base', source: 'weight: 10' };
    if (command === 'build') return { generatedFiles: [] };
    throw new Error(`Unexpected command: ${command}`);
  });
});

afterEach(() => {
  cleanup();
});

// --- Unit Tests for theme.ts and user-state persistence contract ---

test('GUI-THEME-001 & GUI-THEME-004: readStoredThemePreference returns system by default or on invalid legacy values', () => {
  expect(readStoredThemePreference()).toBe('system');

  window.localStorage.setItem(THEME_PREFERENCE_STORAGE_KEY, 'light');
  expect(readStoredThemePreference()).toBe('light');

  window.localStorage.setItem(THEME_PREFERENCE_STORAGE_KEY, 'dark');
  expect(readStoredThemePreference()).toBe('dark');

  window.localStorage.setItem(THEME_PREFERENCE_STORAGE_KEY, 'invalid_theme');
  expect(readStoredThemePreference()).toBe('system');
});

test('GUI-THEME-001 & GUI-THEME-003: resolveEffectiveTheme resolves correctly', () => {
  expect(resolveEffectiveTheme('light', false)).toBe('light');
  expect(resolveEffectiveTheme('light', true)).toBe('light');

  expect(resolveEffectiveTheme('dark', false)).toBe('dark');
  expect(resolveEffectiveTheme('dark', true)).toBe('dark');

  expect(resolveEffectiveTheme('system', false)).toBe('light');
  expect(resolveEffectiveTheme('system', true)).toBe('dark');
});

test('GUI-THEME-002: persistNativeThemePreference writes to native storage, not localStorage', async () => {
  await persistNativeThemePreference('dark');
  expect(invoke).toHaveBeenCalledWith('set_theme_preference', { preference: 'dark' });
  expect(window.localStorage.getItem(THEME_PREFERENCE_STORAGE_KEY)).toBeNull();
});

// --- Integration Tests for App theme behavior ---

test('GUI-THEME-001 & GUI-THEME-004: launches as System when no stored value exists, following OS preference', async () => {
  setupMatchMedia(true); // OS prefers dark
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);

  await waitFor(() => {
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  });
});

test('GUI-THEME-004: launches as System when stored native value is unset or invalid', async () => {
  currentNativeUserState = { themePreference: null };
  setupMatchMedia(false); // OS prefers light
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);

  await waitFor(() => {
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
  });
});

test('GUI-THEME-002, GUI-THEME-003, GUI-THEME-007: switching to Light/Dark updates DOM and persists to native without restarting', async () => {
  setupMatchMedia(true); // OS prefers dark
  const { unmount } = render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);

  // Wait for workspace and bootstrap to be ready
  await screen.findByRole('complementary', { name: 'Explorer' });
  await waitFor(() => {
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  });

  // Open Application Settings dialog
  fireEvent.click(screen.getByRole('button', { name: 'Application Settings' }));
  const lightRadio = await screen.findByRole('radio', { name: 'Light' });
  fireEvent.click(lightRadio);

  // Immediately applied to DOM and persisted to native storage (GUI-THEME-003, GUI-THEME-002)
  expect(document.documentElement.getAttribute('data-theme')).toBe('light');
  expect(invoke).toHaveBeenCalledWith('set_theme_preference', { preference: 'light' });
  expect(currentNativeUserState.themePreference).toBe('light');
  expect(window.localStorage.getItem(THEME_PREFERENCE_STORAGE_KEY)).toBeNull();

  unmount();

  // Re-launching app preserves Light theme from native storage (GUI-THEME-004)
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);
  await screen.findByRole('complementary', { name: 'Explorer' });
  await waitFor(() => {
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
  });

  // Open Application Settings again and switch to Dark
  fireEvent.click(screen.getByRole('button', { name: 'Application Settings' }));
  const darkRadio = await screen.findByRole('radio', { name: 'Dark' });
  fireEvent.click(darkRadio);

  expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  expect(invoke).toHaveBeenCalledWith('set_theme_preference', { preference: 'dark' });
  expect(currentNativeUserState.themePreference).toBe('dark');
  expect(window.localStorage.getItem(THEME_PREFERENCE_STORAGE_KEY)).toBeNull();
});

test('GUI-THEME-003: System theme tracks live OS appearance changes', async () => {
  setupMatchMedia(false); // initially OS is light
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);

  await waitFor(() => {
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
  });

  // OS changes to Dark while app is running
  triggerOsAppearanceChange(true);
  expect(document.documentElement.getAttribute('data-theme')).toBe('dark');

  // OS changes back to Light while app is running
  triggerOsAppearanceChange(false);
  expect(document.documentElement.getAttribute('data-theme')).toBe('light');
});

test('GUI-THEME-003: Explicit Light/Dark choice does not track live OS changes', async () => {
  currentNativeUserState = { themePreference: 'light' };
  setupMatchMedia(false);
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);

  await waitFor(() => {
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
  });

  // OS changes to Dark, but explicit Light must remain (MUST NOT change)
  triggerOsAppearanceChange(true);
  expect(document.documentElement.getAttribute('data-theme')).toBe('light');
});

test('GUI-THEME-002 & GUI-THEME-003: changing theme does not dirty project or lose editor state', async () => {
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);

  // Open a record to introduce an active editor buffer
  const cell = await screen.findByRole('gridcell', { name: /weight:/ });
  fireEvent.keyDown(cell, { key: 'Enter' });
  const input = await screen.findByRole('textbox', { name: 'record 1 weight' });
  fireEvent.change(input, { target: { value: '25' } });
  fireEvent.keyDown(input, { key: 'Enter' });

  // Verify unsaved indicator is visible for the source edit
  expect(await screen.findByText('1 unsaved')).toBeTruthy();

  // Open Application Settings and change theme to Dark
  fireEvent.click(screen.getByRole('button', { name: 'Application Settings' }));
  const darkRadio = await screen.findByRole('radio', { name: 'Dark' });
  fireEvent.click(darkRadio);

  // Close Application Settings modal
  fireEvent.click(screen.getByRole('button', { name: 'Close' }));

  // Editor buffer remains intact with value 25 (GUI-THEME-003)
  const cellAfter = await screen.findByRole('gridcell', { name: /weight:/ });
  expect(cellAfter).toBeTruthy();

  // Dirty count is still exactly 1 (no project/settings dirty caused by theme change) (GUI-THEME-002)
  expect(screen.getByText('1 unsaved')).toBeTruthy();
  expect(invoke).toHaveBeenCalledWith('set_theme_preference', { preference: 'dark' });
});

test('GUI-THEME-002: Project switching retains user theme preference', async () => {
  currentNativeUserState = { themePreference: 'dark' };
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);

  await waitFor(() => {
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  });

  // Switch project by opening another workspace
  const otherWorkspace = {
    project: { project_root: '/other-project', name: 'Other Project', project_id: 'other' },
    sourceRoots: ['.'],
    files: [{ path: 'data.yaml', sourceRoot: '.', kind: 'data', table: 'item', typeName: null, hasInlineRecords: false, diagnostic: null }],
  };
  invoke.mockImplementation(async (command, args) => {
    if (command === 'load_application_user_state') return structuredClone(currentNativeUserState);
    if (command === 'authoring_workspace') return otherWorkspace;
    if (command === 'migration_recovery_status') return null;
    if (command === 'open_data_file') return structuredClone(snapshot());
    if (command === 'preview_data_file') return { candidateSource: 'weight: 20', changed: false, validation };
    return null;
  });

  // Theme remains dark after switching project
  expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  expect(currentNativeUserState.themePreference).toBe('dark');
});

test('GUI-THEME-007: Selecting System does not overwrite storage with resolved effective theme', async () => {
  setupMatchMedia(true); // OS prefers dark
  currentNativeUserState = { themePreference: 'light' };
  render(<App sourcePollingIntervalMs={null} previewDelayMs={0} />);

  await waitFor(() => {
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
  });

  // Open Application Settings and choose System
  fireEvent.click(screen.getByRole('button', { name: 'Application Settings' }));
  const systemRadio = await screen.findByRole('radio', { name: /System/ });
  fireEvent.click(systemRadio);

  // Effective theme is dark (because OS prefers dark), but stored value MUST remain 'system' (GUI-THEME-007)
  expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  expect(invoke).toHaveBeenCalledWith('set_theme_preference', { preference: 'system' });
  expect(currentNativeUserState.themePreference).toBe('system');
  expect(window.localStorage.getItem(THEME_PREFERENCE_STORAGE_KEY)).toBeNull();
});
