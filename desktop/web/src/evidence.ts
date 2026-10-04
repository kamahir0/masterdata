import { invoke } from "@tauri-apps/api/core";
type Controls = Pick<
  typeof import("./main"),
  "rpc" | "selectTarget" | "samples" | "acceptedInteraction" | "view"
>;
export async function run({
  rpc,
  selectTarget,
  samples,
  acceptedInteraction,
  view,
}: Controls) {
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
      },
    });
  } catch (e) {
    await invoke("evidence_write", {
      report: { format: 1, error: String(e), samples },
    });
  }
}
