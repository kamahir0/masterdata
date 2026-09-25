import { beforeEach, describe, expect, test, vi } from 'vitest';
import {
  type ApplicationUserStateDto,
  RECENT_PROJECTS_STORAGE_KEY,
  bootstrapApplicationUserState,
  initializeRecentProjects,
  initializeThemePreference,
  loadNativeApplicationUserState,
  persistNativeRecentProjects,
  persistNativeThemePreference,
  sanitizeRecentProjects,
} from '../src/user-state';
import { THEME_PREFERENCE_STORAGE_KEY } from '../src/theme';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

beforeEach(() => {
  window.localStorage.clear();
  invoke.mockReset();
});

describe('Theme migration and persistence', () => {
  test('native value absent + valid legacy value migrates to native and removes legacy key', async () => {
    window.localStorage.setItem(THEME_PREFERENCE_STORAGE_KEY, 'dark');
    invoke.mockResolvedValue({});

    const result = await initializeThemePreference({});

    expect(result).toBe('dark');
    expect(invoke).toHaveBeenCalledWith('set_theme_preference', { preference: 'dark' });
    expect(window.localStorage.getItem(THEME_PREFERENCE_STORAGE_KEY)).toBeNull();
  });

  test('native value present + legacy value present uses native and does not overwrite with legacy', async () => {
    window.localStorage.setItem(THEME_PREFERENCE_STORAGE_KEY, 'light');

    const result = await initializeThemePreference({ themePreference: 'dark' });

    expect(result).toBe('dark');
    expect(invoke).not.toHaveBeenCalled();
    // Legacy value is untouched because migration is not needed
    expect(window.localStorage.getItem(THEME_PREFERENCE_STORAGE_KEY)).toBe('light');
  });

  test('invalid legacy value falls back to system without writing invalid value to native', async () => {
    window.localStorage.setItem(THEME_PREFERENCE_STORAGE_KEY, 'corrupted_theme');

    const result = await initializeThemePreference({});

    expect(result).toBe('system');
    expect(invoke).not.toHaveBeenCalled();
  });

  test('absent native and absent legacy falls back to system', async () => {
    const result = await initializeThemePreference({});
    expect(result).toBe('system');
    expect(invoke).not.toHaveBeenCalled();
  });

  test('native write failure does not destroy legacy localStorage value', async () => {
    window.localStorage.setItem(THEME_PREFERENCE_STORAGE_KEY, 'light');
    invoke.mockRejectedValue(new Error('disk full or IO error'));

    const result = await initializeThemePreference({});

    expect(result).toBe('light');
    expect(invoke).toHaveBeenCalledWith('set_theme_preference', { preference: 'light' });
    // Legacy key must NOT be destroyed if native write failed
    expect(window.localStorage.getItem(THEME_PREFERENCE_STORAGE_KEY)).toBe('light');
  });

  test('normal theme update does not write to localStorage', async () => {
    invoke.mockResolvedValue({});

    await persistNativeThemePreference('dark');

    expect(invoke).toHaveBeenCalledWith('set_theme_preference', { preference: 'dark' });
    expect(window.localStorage.getItem(THEME_PREFERENCE_STORAGE_KEY)).toBeNull();
  });
});

describe('Recent Projects migration, sanitization, and persistence', () => {
  test('native value absent + legacy array migrates sanitized to native and removes legacy key', async () => {
    const legacy = [
      { root: '/p1', name: 'P1' },
      { root: '/p2', name: 'P2' },
      { root: '/p1', name: 'P1 duplicate' },
    ];
    window.localStorage.setItem(RECENT_PROJECTS_STORAGE_KEY, JSON.stringify(legacy));
    invoke.mockResolvedValue({});

    const result = await initializeRecentProjects({});

    expect(result).toEqual([
      { root: '/p1', name: 'P1' },
      { root: '/p2', name: 'P2' },
    ]);
    expect(invoke).toHaveBeenCalledWith('set_recent_projects', {
      projects: [
        { root: '/p1', name: 'P1' },
        { root: '/p2', name: 'P2' },
      ],
    });
    expect(window.localStorage.getItem(RECENT_PROJECTS_STORAGE_KEY)).toBeNull();
  });

  test('native value present does not overwrite with legacy value', async () => {
    window.localStorage.setItem(
      RECENT_PROJECTS_STORAGE_KEY,
      JSON.stringify([{ root: '/legacy', name: 'Legacy' }]),
    );

    const nativeProjects = [{ root: '/native', name: 'Native' }];
    const result = await initializeRecentProjects({ recentProjects: nativeProjects });

    expect(result).toEqual(nativeProjects);
    expect(invoke).not.toHaveBeenCalled();
    expect(window.localStorage.getItem(RECENT_PROJECTS_STORAGE_KEY)).not.toBeNull();
  });

  test('sanitizeRecentProjects caps at 10 items and deduplicates roots preserving most recent order', () => {
    const raw = [
      { root: '/p0', name: 'P0' },
      { root: '/p1', name: 'P1' },
      { root: '/p0', name: 'P0 Duplicate' },
      ...Array.from({ length: 15 }, (_, i) => ({ root: `/extra_${i}`, name: `Extra ${i}` })),
    ];

    const sanitized = sanitizeRecentProjects(raw);

    expect(sanitized).toHaveLength(10);
    expect(sanitized[0]).toEqual({ root: '/p0', name: 'P0' });
    expect(sanitized[1]).toEqual({ root: '/p1', name: 'P1' });
    expect(sanitized.filter((p) => p.root === '/p0')).toHaveLength(1);
  });

  test('normal recent projects update does not write to localStorage', async () => {
    invoke.mockResolvedValue({});

    await persistNativeRecentProjects([{ root: '/p', name: 'P' }]);

    expect(invoke).toHaveBeenCalledWith('set_recent_projects', {
      projects: [{ root: '/p', name: 'P' }],
    });
    expect(window.localStorage.getItem(RECENT_PROJECTS_STORAGE_KEY)).toBeNull();
  });
});

describe('Bootstrap application user state', () => {
  test('loads native state, migrates both theme and recent projects, applies theme to DOM', async () => {
    window.localStorage.setItem(THEME_PREFERENCE_STORAGE_KEY, 'dark');
    window.localStorage.setItem(
      RECENT_PROJECTS_STORAGE_KEY,
      JSON.stringify([{ root: '/p1', name: 'Project 1' }]),
    );
    invoke.mockImplementation(async (command) => {
      if (command === 'load_application_user_state') return {};
      if (command === 'set_theme_preference') return {};
      if (command === 'set_recent_projects') return {};
      throw new Error(`Unexpected command: ${command}`);
    });

    const state = await bootstrapApplicationUserState();

    expect(state.themePreference).toBe('dark');
    expect(state.recentProjects).toEqual([{ root: '/p1', name: 'Project 1' }]);
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    expect(window.localStorage.getItem(THEME_PREFERENCE_STORAGE_KEY)).toBeNull();
    expect(window.localStorage.getItem(RECENT_PROJECTS_STORAGE_KEY)).toBeNull();
  });
});
