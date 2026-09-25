export type ThemePreference = "system" | "light" | "dark";
export type EffectiveTheme = "light" | "dark";

/**
 * Legacy storage key used for migration only.
 * localStorage MUST NOT be used as the canonical persistence authority (PROJECT-CONFIG-007, GUI-THEME-002).
 */
export const THEME_PREFERENCE_STORAGE_KEY = "masterdata.theme-preference.v1";

/**
 * Read the legacy stored theme preference from localStorage.
 * Only used as fallback migration source when native storage has no preference.
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
