export type ThemePreference = "system" | "light" | "dark";
export type EffectiveTheme = "light" | "dark";

export const THEME_PREFERENCE_STORAGE_KEY = "masterdata.theme-preference.v1";

/**
 * Read the user's stored theme preference from user-local storage.
 * Defaults to "system" if no preference is stored or if stored value is invalid (GUI-THEME-001, GUI-THEME-004).
 */
export function readStoredThemePreference(): ThemePreference {
  try {
    const stored = window.localStorage.getItem(THEME_PREFERENCE_STORAGE_KEY);
    if (stored === "system" || stored === "light" || stored === "dark") {
      return stored;
    }
  } catch {
    // If storage is unavailable or throws, fallback to system.
  }
  return "system";
}

/**
 * Store the user's theme preference in user-local storage.
 * Does not mutate project-scoped files or cause project dirty state (GUI-THEME-002).
 */
export function storeThemePreference(preference: ThemePreference): void {
  try {
    window.localStorage.setItem(THEME_PREFERENCE_STORAGE_KEY, preference);
  } catch {
    // Storage write failure is handled gracefully without crashing.
  }
}

/**
 * Detect OS appearance preference.
 * Fallback to Light (false) if OS preference cannot be determined (GUI-THEME-001).
 */
export function getOsPrefersDark(): boolean {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return false;
  }
  try {
    return window.matchMedia("(prefers-color-scheme: dark)").matches;
  } catch {
    return false;
  }
}

/**
 * Resolve the effective theme from the preference and OS dark state (GUI-THEME-001, GUI-THEME-003).
 */
export function resolveEffectiveTheme(
  preference: ThemePreference,
  osPrefersDark: boolean,
): EffectiveTheme {
  if (preference === "light") return "light";
  if (preference === "dark") return "dark";
  return osPrefersDark ? "dark" : "light";
}

/**
 * Apply the effective theme to document root attribute (GUI-THEME-003, GUI-THEME-005).
 */
export function applyThemeToDom(effective: EffectiveTheme): void {
  if (typeof document !== "undefined" && document.documentElement) {
    document.documentElement.setAttribute("data-theme", effective);
  }
}
