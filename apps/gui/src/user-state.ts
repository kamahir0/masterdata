import { invoke } from "@tauri-apps/api/core";
import {
  type EffectiveTheme,
  type ThemePreference,
  THEME_PREFERENCE_STORAGE_KEY,
  applyThemeToDom,
  getOsPrefersDark,
  resolveEffectiveTheme,
} from "./theme";

export type RecentProject = {
  root: string;
  name: string;
};

export type ApplicationUserStateDto = {
  themePreference?: ThemePreference | null;
  recentProjects?: RecentProject[] | null;
};

export const RECENT_PROJECTS_STORAGE_KEY = "masterdata.recent-projects.v1";
export const RECENT_PROJECT_LIMIT = 10;

export type InitialApplicationUserState = {
  themePreference: ThemePreference;
  recentProjects: RecentProject[];
};

/**
 * Load user state from native storage.
 * Gracefully fallbacks to empty object on error.
 */
export async function loadNativeApplicationUserState(): Promise<ApplicationUserStateDto> {
  try {
    return await invoke<ApplicationUserStateDto>("load_application_user_state");
  } catch (error) {
    return {};
  }
}

/**
 * Persist theme preference to native storage.
 */
export async function persistNativeThemePreference(preference: ThemePreference): Promise<void> {
  try {
    await invoke("set_theme_preference", { preference });
  } catch (error) {
    console.error("Failed to persist theme preference to native backend:", error);
    throw error;
  }
}

/**
 * Persist recent projects to native storage.
 */
export async function persistNativeRecentProjects(projects: RecentProject[]): Promise<void> {
  try {
    await invoke("set_recent_projects", { projects });
  } catch (error) {
    console.error("Failed to persist recent projects to native backend:", error);
    throw error;
  }
}

/**
 * Sanitize recent projects by deduplicating canonical roots and limiting to max 10 entries (GUI-PROJECT-002).
 */
export function sanitizeRecentProjects(projects: RecentProject[]): RecentProject[] {
  const seen = new Set<string>();
  const result: RecentProject[] = [];
  for (const p of projects) {
    if (!p || typeof p.root !== "string" || typeof p.name !== "string") continue;
    if (!seen.has(p.root)) {
      seen.add(p.root);
      result.push({ root: p.root, name: p.name });
      if (result.length >= RECENT_PROJECT_LIMIT) break;
    }
  }
  return result;
}

/**
 * Initialize theme preference with legacy localStorage migration support.
 */
export async function initializeThemePreference(
  nativeState: ApplicationUserStateDto,
): Promise<ThemePreference> {
  // 1. If native preference is valid, use it
  if (
    nativeState.themePreference === "system" ||
    nativeState.themePreference === "light" ||
    nativeState.themePreference === "dark"
  ) {
    return nativeState.themePreference;
  }

  // 2. Native value not set, check legacy localStorage
  let legacyValue: string | null = null;
  try {
    legacyValue = window.localStorage.getItem(THEME_PREFERENCE_STORAGE_KEY);
  } catch {
    // localStorage inaccessible
  }

  if (legacyValue === "system" || legacyValue === "light" || legacyValue === "dark") {
    // 3. Save to native storage
    try {
      await persistNativeThemePreference(legacyValue);
      // 4. Remove legacy key on success
      try {
        window.localStorage.removeItem(THEME_PREFERENCE_STORAGE_KEY);
      } catch {}
    } catch {
      // 5. If migration write fails, do not destroy legacy value
    }
    return legacyValue;
  }

  // 6. Legacy value absent or invalid -> default to "system"
  return "system";
}

/**
 * Initialize recent projects with legacy localStorage migration support.
 */
export async function initializeRecentProjects(
  nativeState: ApplicationUserStateDto,
): Promise<RecentProject[]> {
  // 1. If native recent projects exist, use them
  if (Array.isArray(nativeState.recentProjects)) {
    return sanitizeRecentProjects(nativeState.recentProjects);
  }

  // 2. Native value not set, check legacy localStorage
  let legacyProjects: RecentProject[] = [];
  try {
    const raw = window.localStorage.getItem(RECENT_PROJECTS_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        legacyProjects = parsed.filter(
          (item): item is RecentProject =>
            typeof item?.root === "string" && typeof item?.name === "string",
        );
      }
    }
  } catch {
    // localStorage inaccessible or malformed
  }

  if (legacyProjects.length > 0) {
    const sanitized = sanitizeRecentProjects(legacyProjects);
    try {
      await persistNativeRecentProjects(sanitized);
      try {
        window.localStorage.removeItem(RECENT_PROJECTS_STORAGE_KEY);
      } catch {}
    } catch {
      // If migration write fails, do not destroy legacy value
    }
    return sanitized;
  }

  return [];
}

/**
 * Bootstrap application user state and immediately apply effective theme to DOM.
 * Prevents theme flash prior to React initial mount (GUI-THEME-004).
 */
export async function bootstrapApplicationUserState(): Promise<InitialApplicationUserState> {
  const nativeState = await loadNativeApplicationUserState();
  const [themePreference, recentProjects] = await Promise.all([
    initializeThemePreference(nativeState),
    initializeRecentProjects(nativeState),
  ]);

  const effectiveTheme: EffectiveTheme = resolveEffectiveTheme(themePreference, getOsPrefersDark());
  applyThemeToDom(effectiveTheme);

  return {
    themePreference,
    recentProjects,
  };
}
