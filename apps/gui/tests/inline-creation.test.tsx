import React from "react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { InlineCreationRow } from "../src/InlineCreation";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const onCommit = vi.fn(async () => {});
const onCancel = vi.fn();

beforeEach(() => {
  invoke.mockReset();
  onCommit.mockClear();
  onCancel.mockClear();
  invoke.mockImplementation(async (command: string) => {
    if (command === "creation_context") return { choices: { tables: ["item"] } };
    if (command === "create_source") return { status: "success", path: "weapon.yaml", folder: false, diagnostic: null };
    if (command === "recheck_creation") return { exists: false };
    throw new Error(`Unexpected command: ${command}`);
  });
});

afterEach(cleanup);

function open(category: "folder" | "table" | "data" = "table", projectPath = "/project") {
  return render(
    <InlineCreationRow
      projectPath={projectPath}
      state={{ root: "sources", folder: "items", category, initialTable: category === "data" ? "item" : undefined }}
      canWrite
      onCommit={onCommit}
      onCancel={onCancel}
    />,
  );
}

test("inline Table commit delegates a starter request without building YAML in the UI", async () => {
  open();
  const filename = screen.getByLabelText("Filename (.yaml / .yml)");
  fireEvent.change(filename, { target: { value: "weapon" } });
  fireEvent.keyDown(filename, { key: "Enter" });
  await waitFor(() => expect(onCommit).toHaveBeenCalledOnce());
  expect(invoke).toHaveBeenCalledWith("create_source", {
    projectPath: "/project",
    request: {
      sourceRoot: "sources",
      destination: "items/weapon.yaml",
      artifact: { category: "starter", kind: "table" },
    },
  });
});

test("Data inline creation requires and transports the selected existing Table", async () => {
  open("data");
  const filename = screen.getByLabelText("Filename (.yaml / .yml)");
  await waitFor(() => expect((screen.getByRole("button", { name: "Create", exact: true }) as HTMLButtonElement).disabled).toBe(false));
  fireEvent.change(filename, { target: { value: "items.yaml" } });
  fireEvent.click(screen.getByRole("button", { name: "Create", exact: true }));
  await waitFor(() => expect(onCommit).toHaveBeenCalledOnce());
  expect(invoke).toHaveBeenCalledWith("create_source", expect.objectContaining({
    request: expect.objectContaining({ artifact: { category: "starter", kind: "data", table: "item" } }),
  }));
});

test("Outcome Unknown persists across cancel and reopen until the submitted destination is rechecked", async () => {
  invoke.mockImplementation(async (command: string) => {
    if (command === "create_source") return { status: "outcome_unknown", path: "", folder: false, diagnostic: { code: "E-UNKNOWN", message: "unknown" } };
    if (command === "recheck_creation") return { exists: false };
    throw new Error(`Unexpected command: ${command}`);
  });
  open("folder", "/uncertain-project");
  fireEvent.click(screen.getByRole("button", { name: "Create", exact: true }));
  await screen.findByRole("alert");
  expect((screen.getByRole("button", { name: "Create", exact: true }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Cancel", exact: true }));
  cleanup();
  open("folder", "/uncertain-project");
  expect((screen.getByRole("button", { name: "Create", exact: true }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Recheck Workspace" }));
  await waitFor(() => expect((screen.getByRole("button", { name: "Create", exact: true }) as HTMLButtonElement).disabled).toBe(false));
  expect(invoke.mock.calls.filter(([command]) => command === "create_source")).toHaveLength(1);
});
