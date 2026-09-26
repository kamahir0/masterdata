import React from "react";
import { afterEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import ValueEditor from "../src/ValueEditor";
import type { AuthoringValue, ResolvedAuthoringField } from "../src/data-editor-types";

const noInvalidPaths = new Set<string>();
const field = (shape: ResolvedAuthoringField["shape"], modifier: ResolvedAuthoringField["modifier"] = "required"): ResolvedAuthoringField => ({
  name: "value",
  typeName: shape.kind === "primitive" ? shape.primitive : shape.name,
  modifier,
  shape,
});

afterEach(cleanup);

test("Value Object keeps an exact decimal underlying value as text", () => {
  const onChange = vi.fn();
  render(<ValueEditor
    field={field({ kind: "value_object", name: "ItemId", underlying: "ulong" })}
    value={{ kind: "number", value: "18446744073709551615" }}
    label="weight"
    cellKey="0:weight"
    editable
    invalidPaths={noInvalidPaths}
    onChange={onChange}
  />);

  const input = screen.getByRole("textbox", { name: "weight" });
  fireEvent.change(input, { target: { value: "18446744073709551614" } });
  expect(onChange).toHaveBeenLastCalledWith({ kind: "number", value: "18446744073709551614" });
});

test("custom, nullable, and array editors materialize only after an explicit choice", () => {
  const customChange = vi.fn();
  const custom = field({ kind: "custom", name: "Profile", fields: [
    field({ kind: "primitive", primitive: "ulong" }),
    field({ kind: "primitive", primitive: "string" }),
  ].map((nested, index) => ({ ...nested, name: index === 0 ? "credits" : "label" })) });
  render(<ValueEditor
    field={custom}
    value={{ kind: "null" }}
    label="profile"
    cellKey="0:profile"
    editable
    invalidPaths={noInvalidPaths}
    onChange={customChange}
  />);
  expect(screen.getByText("Profile fields are not materialized.")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Materialize profile (Profile) fields" }));
  expect(customChange).toHaveBeenLastCalledWith({ kind: "mapping", entries: [
    { name: "credits", value: { kind: "null" } },
    { name: "label", value: { kind: "null" } },
  ] });

  cleanup();
  const arrayChange = vi.fn();
  render(<ValueEditor
    field={field({ kind: "primitive", primitive: "string" }, "array")}
    value={{ kind: "null" }}
    label="tags"
    cellKey="0:tags"
    editable
    invalidPaths={noInvalidPaths}
    onChange={arrayChange}
  />);
  expect(screen.getByText("Array is not materialized.")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Make empty array for tags" }));
  expect(arrayChange).toHaveBeenLastCalledWith({ kind: "sequence", sourceIdentity: true, items: [] });

  cleanup();
  const nullableChange = vi.fn();
  render(<ValueEditor
    field={field({ kind: "primitive", primitive: "string" }, "nullable")}
    value={{ kind: "string", value: "kept" }}
    label="alias"
    cellKey="0:alias"
    editable
    invalidPaths={noInvalidPaths}
    onChange={nullableChange}
  />);
  fireEvent.click(screen.getByRole("button", { name: "Set alias to null" }));
  expect(nullableChange).toHaveBeenLastCalledWith({ kind: "null" });
});

test("array editing preserves source occurrence identity while reordering", () => {
  const onChange = vi.fn();
  const value: AuthoringValue = { kind: "sequence", sourceIdentity: true, items: [
    { sourceIndex: 0, value: { kind: "string", value: "first" } },
    { sourceIndex: 1, value: { kind: "string", value: "second" } },
  ] };
  render(<ValueEditor
    field={field({ kind: "primitive", primitive: "string" }, "array")}
    value={value}
    label="tags"
    cellKey="0:tags"
    editable
    invalidPaths={noInvalidPaths}
    onChange={onChange}
  />);

  fireEvent.click(screen.getByRole("button", { name: "Actions for tags item 2" }));
  fireEvent.click(screen.getByRole("menuitem", { name: "Move tags item 2 up" }));
  expect(onChange).toHaveBeenLastCalledWith({
    kind: "sequence",
    sourceIdentity: true,
    items: [value.items[1], value.items[0]],
  });
});

test("enum and flags controls emit typed authoring values", async () => {
  const enumChange = vi.fn();
  render(<ValueEditor
    field={field({ kind: "enum", name: "Status", underlying: "int", members: ["Ready", "Paused"] })}
    value={{ kind: "null" }}
    label="status"
    cellKey="0:status"
    editable
    invalidPaths={noInvalidPaths}
    onChange={enumChange}
  />);
  fireEvent.mouseDown(screen.getByRole("combobox", { name: "status" }));
  fireEvent.click((await screen.findAllByText("Paused", { selector: ".ant-select-item-option-content" })).at(-1)!);
  expect(enumChange).toHaveBeenLastCalledWith({ kind: "string", value: "Paused" });

  cleanup();
  const flagsChange = vi.fn();
  render(<ValueEditor
    field={field({ kind: "flags", name: "Permissions", underlying: "int", members: ["None", "Read", "Write"] })}
    value={{ kind: "null" }}
    label="access"
    cellKey="0:access"
    editable
    invalidPaths={noInvalidPaths}
    onChange={flagsChange}
  />);
  expect(screen.getByText("No flags value selected yet.")).toBeTruthy();
  fireEvent.click(screen.getByRole("checkbox", { name: "access Write" }));
  expect(flagsChange).toHaveBeenLastCalledWith({
    kind: "sequence",
    sourceIdentity: true,
    items: [{ sourceIndex: null, value: { kind: "string", value: "Write" } }],
  });
});

test("nested diagnostic paths mark the matching ValueEditor field", () => {
  const profile = field({ kind: "custom", name: "Profile", fields: [
    { ...field({ kind: "primitive", primitive: "ulong" }), name: "credits" },
    { ...field({ kind: "primitive", primitive: "string" }), name: "label" },
  ] });
  render(<ValueEditor
    field={profile}
    value={{ kind: "mapping", entries: [
      { name: "credits", value: { kind: "number", value: "18446744073709551615" } },
      { name: "label", value: { kind: "string", value: "kept" } },
    ] }}
    label="profile"
    cellKey="0:profile"
    editable
    invalidPaths={new Set(["/credits"])}
    onChange={() => {}}
  />);

  const credits = screen.getByRole("textbox", { name: "credits" });
  expect(credits.getAttribute("aria-invalid")).toBe("true");
  expect(credits.getAttribute("data-value-path")).toBe("0:profile/credits");
});
