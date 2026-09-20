import { expect, test, vi } from "vitest";
import { migrationRefreshPlan, resolveDirtyPathMutation } from "../src/authoring-workflow";

test("Cancel and failed Save never start a dirty path mutation", async () => {
  const save = vi.fn(async () => false);
  const discard = vi.fn();
  const mutate = vi.fn();

  expect(await resolveDirtyPathMutation({ decision: "cancel", save, discard, mutate })).toBe(false);
  expect(await resolveDirtyPathMutation({ decision: "save", save, discard, mutate })).toBe(false);
  expect(save).toHaveBeenCalledOnce();
  expect(discard).not.toHaveBeenCalled();
  expect(mutate).not.toHaveBeenCalled();
});

test("Don't Save discards only the selected buffer before starting path mutation", async () => {
  const events: string[] = [];
  const save = vi.fn(async () => { events.push("save"); return true; });
  const discard = vi.fn(() => { events.push("discard"); });
  const mutate = vi.fn(() => { events.push("mutate"); });

  expect(await resolveDirtyPathMutation({ decision: "discard", save, discard, mutate })).toBe(true);
  expect(events).toEqual(["discard", "mutate"]);
  expect(save).not.toHaveBeenCalled();
});

test("Save starts path mutation only after the selected buffer is saved", async () => {
  const events: string[] = [];
  const save = vi.fn(async () => { events.push("save"); return true; });
  const discard = vi.fn(() => { events.push("discard"); });
  const mutate = vi.fn(() => { events.push("mutate"); });

  expect(await resolveDirtyPathMutation({ decision: "save", save, discard, mutate })).toBe(true);
  expect(events).toEqual(["save", "mutate"]);
  expect(discard).not.toHaveBeenCalled();
});

test("migration refresh reloads affected clean editors and retains unrelated dirty buffers", () => {
  const editors = {
    "data.yaml": { dirty: false },
    "other.yaml": { dirty: true },
    "untouched.yaml": { dirty: false },
  };

  const plan = migrationRefreshPlan(editors, ["data.yaml", "other.yaml"], (editor) => editor.dirty);

  expect(plan.reloadPaths).toEqual(["data.yaml"]);
  expect(plan.retainedEditors).toEqual({
    "other.yaml": { dirty: true },
    "untouched.yaml": { dirty: false },
  });
});
