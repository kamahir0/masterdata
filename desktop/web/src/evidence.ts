import { invoke } from "@tauri-apps/api/core";
import { desktop } from "./workspace";
type Controls = Pick<
  typeof import("./main"),
  | "rpc"
  | "selectTarget"
  | "samples"
  | "acceptedInteraction"
  | "view"
  | "startup"
>;
export async function run({
  rpc,
  selectTarget,
  samples,
  acceptedInteraction,
  view,
  startup,
}: Controls) {
  const presentation: Record<string, unknown>[] = [];
  const interaction: Record<string, unknown> = {};
  if (!document.hasFocus())
    await new Promise<void>((resolve) =>
      window.addEventListener("focus", () => resolve(), { once: true }),
    );
  try {
    const paths = [
      "sources/a-1.yaml",
      "sources/a-2.yaml",
      "sources/b-schema.yaml",
      "sources/c-schema.yaml",
    ];
    const first = await selectTarget(paths[0], "firstSource", false);
    acceptedInteraction(first);
    desktop.viewport!.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
    );
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => resolve()),
    );
    const input = document.querySelector<HTMLInputElement>(
      ".active-cell-editor input",
    );
    Object.assign(interaction, {
      singleEditor: document.querySelectorAll(".active-cell-editor input")
        .length,
      focusedEditor: input === document.activeElement,
      initialValue: input?.value,
      selection: desktop.interaction.selection,
      busy: desktop.surface.busy,
      shape: view()?.columns[desktop.interaction.selection.column].shape,
      cell: view()?.rows[0].cells[desktop.interaction.selection.column],
    });
    if (!input || input !== document.activeElement)
      throw new Error("keyboard edit did not publish a focused active editor");
    input.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
    );
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => resolve()),
    );
    interaction.cancelRestoredGrid =
      document.activeElement === desktop.viewport &&
      !document.querySelector(".active-cell-editor") &&
      !view()?.dirty;
    if (!interaction.cancelRestoredGrid)
      throw new Error(
        "editor cancel did not preserve the clean source and grid focus",
      );
    const original = desktop.surface.theme;
    for (const appearance of ["dark", "light"] as const) {
      await desktop.setTheme(appearance);
      await new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      );
      const shell = document.getElementById("titlebar")!,
        button = shell.querySelector("button")!,
        explorer = document.querySelector(".ant-tree")!;
      presentation.push({
        appearance,
        shellBackground: getComputedStyle(shell).backgroundColor,
        buttonColor: getComputedStyle(button).color,
        explorerColor: getComputedStyle(explorer).color,
        rootClass: document.querySelector(".desktop-app")?.className,
        buttonClass: button.className,
        colorToken:
          getComputedStyle(button).getPropertyValue("--ant-color-text"),
        reducedMotion: matchMedia("(prefers-reduced-motion: reduce)").matches,
      });
    }
    await desktop.setTheme(original);
    for (let run = 0; run < 3; run++)
      for (let n = 0; n < 100; n++)
        for (const [caseName, path] of [
          ["revisit", paths[0]],
          ["sameTable", paths[1]],
          ["crossTable", paths[2]],
          ["schema", paths[3]],
        ]) {
          const s = await selectTarget(path, caseName, false);
          acceptedInteraction(s);
        }
    await selectTarget(paths[0], "dirtySetup", false);
    const p = view()!;
    await rpc({
      kind: "editText",
      source: p.source,
      revision: p.revision,
      row: p.rows[0].id,
      field: "id",
      text: "999999",
    });
    for (let n = 0; n < 100; n++) {
      await selectTarget(paths[1], "dirtyAway", false);
      const s = await selectTarget(paths[0], "dirtyRevisit", false);
      acceptedInteraction(s);
    }
    for (let n = 0; n < 100; n++) {
      const results = await Promise.all(
        [...paths.slice(0, 3), "sources/c-2.yaml"].map((p) =>
          selectTarget(p, "rapid", false),
        ),
      );
      acceptedInteraction(results.at(-1)!);
    }
    await invoke("evidence_write", {
      report: {
        format: 1,
        platform: navigator.platform,
        userAgent: navigator.userAgent,
        visibility: document.visibilityState,
        focused: document.hasFocus(),
        clock:
          "performance.now; rAF is target-filtered paint opportunity, not GPU presentation",
        samples,
        startup,
        presentation,
        interaction,
      },
    });
  } catch (e) {
    await invoke("evidence_write", {
      report: {
        format: 1,
        error: String(e),
        samples,
        startup,
        presentation,
        interaction,
        visibility: document.visibilityState,
        focused: document.hasFocus(),
      },
    });
  }
}
