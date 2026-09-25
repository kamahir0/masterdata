import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { applyThemeToDom, getOsPrefersDark, readStoredThemePreference, resolveEffectiveTheme } from "./theme";
import "./styles.css";

// Immediate pre-render theme application to avoid any visual flash (GUI-THEME-004)
applyThemeToDom(resolveEffectiveTheme(readStoredThemePreference(), getOsPrefersDark()));

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);


