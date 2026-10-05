import { createRoot } from "react-dom/client";
import { flushSync } from "react-dom";
import { invoke } from "@tauri-apps/api/core";
import { Application } from "./app";
import { desktop, type Preference } from "./workspace";
import type { SelectionSample } from "./types";
import "antd/dist/reset.css";
import "./style.css";

export const startup: Record<string, unknown> = {
  webEntryMs: performance.now(),
};
const boot = await invoke<{
  platform: string;
  initialProject: string | null;
  evidence: boolean;
  evidenceKind: string;
  preferences: { theme: Preference };
}>("boot");
startup.bootReadyMs = performance.now();
desktop.surface = { ...desktop.surface, theme: boot.preferences.theme };
desktop.evidence = boot.evidence;
if (boot.evidence) {
  const errors: string[] = [];
  startup.browserErrors = errors;
  window.addEventListener("error", (event) =>
    errors.push(event.error?.stack ?? event.message),
  );
  window.addEventListener("unhandledrejection", (event) =>
    errors.push(String(event.reason?.stack ?? event.reason)),
  );
}
await desktop.start();
flushSync(() =>
  createRoot(document.getElementById("app")!).render(
    <Application platform={boot.platform} />,
  ),
);
startup.shellReactCommitMs = performance.now();
if (boot.initialProject) {
  startup.projectOpenInputMs = performance.now();
  const opened = await desktop
    .openProject(boot.initialProject)
    .catch(desktop.showError);
  startup.projectInventoryPublicationMs = performance.now();
  startup.projectHost = opened?.host;
}
document.addEventListener("keydown", (e) => {
  if (!(e.metaKey || e.ctrlKey) || e.isComposing) return;
  if (e.key.toLowerCase() === "s") {
    e.preventDefault();
    void desktop.save();
  } else if (e.key.toLowerCase() === "f") {
    e.preventDefault();
    const input = document.getElementById("search") as HTMLInputElement | null;
    input?.focus();
    input?.select();
  }
});
export const rpc = desktop.rpc.bind(desktop);
export const selectTarget = desktop.selectTarget.bind(desktop);
export const samples = desktop.samples;
export const view = () => desktop.surface.projection;
export function acceptedInteraction(sample: SelectionSample) {
  const viewport = desktop.viewport;
  viewport?.focus();
  viewport?.dispatchEvent(
    new KeyboardEvent("keydown", {
      key:
        desktop.interaction.selection.column ===
        (view()?.columns.length ?? 1) - 1
          ? "ArrowLeft"
          : "ArrowRight",
      bubbles: true,
    }),
  );
  if (!sample.firstAccepted)
    sample.invalid = sample.invalid ?? "interaction was not accepted";
}
if (boot.evidence) {
  const evidence =
    boot.evidenceKind === "authoring"
      ? await import("./authoring-evidence")
      : boot.evidenceKind === "external"
        ? await import("./external-evidence")
      : boot.evidenceKind === "creation"
        ? await import("./creation-evidence")
      : boot.evidenceKind === "path"
        ? await import("./path-evidence")
      : boot.evidenceKind === "migration"
        ? await import("./migration-evidence")
      : boot.evidenceKind === "type"
        ? await import("./type-evidence")
      : boot.evidenceKind === "delivery"
        ? await import("./delivery-evidence")
      : boot.evidenceKind === "capacity"
        ? await import("./capacity-evidence")
      : boot.evidenceKind === "tags"
        ? await import("./tags-evidence")
      : await import("./evidence");
  await evidence.run({
    rpc,
    selectTarget,
    samples,
    acceptedInteraction,
    view,
    startup,
  });
}
