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
  inputCapture: boolean;
  evidenceKind: string;
  preferences: { theme: Preference;recentProjects:{root:string;name:string}[] };
}>("boot");
startup.bootReadyMs = performance.now();
desktop.surface = { ...desktop.surface, theme: boot.preferences.theme,recentProjects:boot.preferences.recentProjects };
desktop.evidence = boot.evidence;
desktop.inputCapture = boot.inputCapture;
if (boot.evidence||boot.inputCapture) {
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
  if(boot.inputCapture&&e.altKey&&e.shiftKey){
    const cases:Record<string,string>={Digit1:"firstSource",Digit2:"revisit",Digit3:"sameTable",Digit4:"crossTable",Digit5:"schema",Digit6:"dirtyRevisit",Digit7:"rapid",Digit8:"capacityWarm",Digit0:"ordinary"};
    if(cases[e.code]){desktop.measurementCase=cases[e.code];e.preventDefault();return;}
    if(e.code==="KeyR"){desktop.samples.length=0;desktop.droppedSamples=0;e.preventDefault();return;}
    if(e.code==="KeyP"){
      e.preventDefault();
      const p=desktop.surface.projection;
      void invoke("evidence_write",{report:{format:1,kind:"actual-os-input",startup,samples:desktop.samples,droppedSamples:desktop.droppedSamples,
        clock:"performance.now at DOM input handler; eventTimestamp is recorded separately",probe:"trusted pointer/keyboard accepted after target paint opportunity; rAF is not GPU presentation",
        current:{target:desktop.surface.target,source:p?.source,totalRows:p?.totalRows,columns:p?.columns.length,dirty:desktop.surface.status.dirty,
          mountedRows:desktop.viewport?.querySelectorAll('.grid-row').length,mountedCells:desktop.viewport?.querySelectorAll('[role=gridcell]').length,
          activeEditors:document.querySelectorAll('.active-cell-editor').length,theme:document.documentElement.dataset.theme,reducedMotion:document.documentElement.dataset.reducedMotion}}}).catch(desktop.showError);
      return;
    }
  }
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
if(boot.inputCapture){
  const unavailable=()=>{const sample=desktop.currentSample;if(sample&&!sample.firstAccepted&&!sample.invalid)sample.invalid="measurement unavailable: foreground lost during selection";};
  window.addEventListener("blur",unavailable);
  document.addEventListener("visibilitychange",()=>{if(document.visibilityState!=="visible")unavailable();});
}
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
      : boot.evidenceKind === "settings"
        ? await import("./settings-evidence")
      : boot.evidenceKind === "project"
        ? await import("./project-evidence")
      : boot.evidenceKind === "declaration"
        ? await import("./declaration-evidence")
      : boot.evidenceKind === "focus"
        ? await import("./focus-evidence")
      : boot.evidenceKind === "geometry"
        ? await import("./geometry-evidence")
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
