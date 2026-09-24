import React from "react";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { WorkspaceNavigation, groupNavigationFiles, type NavigationFile } from "../src/WorkspaceNavigation";

afterEach(cleanup);

const files: NavigationFile[] = [
  { path: "sources/part-b.yaml", sourceRoot: "sources", kind: "data", table: "Item", typeName: null },
  { path: "sources/item.yaml", sourceRoot: "sources", kind: "schema", table: "Item", typeName: null },
  { path: "extra/part-a.yaml", sourceRoot: "extra", kind: "data", table: "Item", typeName: null },
  { path: "sources/rarity.yaml", sourceRoot: "sources", kind: "type", table: null, typeName: "Rarity" },
  { path: "sources/broken.yaml", sourceRoot: "sources", kind: "unknown", table: null, typeName: null },
];

test("workspace metadata groups one Table across schema and data sources without guessing from paths", () => {
  const grouped = groupNavigationFiles(files);
  expect(grouped.tables).toEqual([{ name: "Item", schema: [files[1]], data: [files[2], files[0]] }]);
  expect(grouped.types).toEqual([files[3]]);
  expect(grouped.tables.flatMap((table) => [...table.schema, ...table.data])).not.toContain(files[4]);
});

test("Table navigation exposes contextual actions and keeps source files available", () => {
  const onSelectFile = vi.fn();
  const onSelectTable = vi.fn();
  const onCreateData = vi.fn();
  const onCreate = vi.fn();
  render(<WorkspaceNavigation
    files={files} activePath="sources/part-b.yaml" selectedTable="Item" revealSourcePath={null} surface="editor"
    dirtyPaths={new Set(["sources/part-b.yaml"])} canWrite buildBlocked={false} validating={false} building={false}
    onSelectFile={onSelectFile} onSelectTable={onSelectTable} onSettings={vi.fn()} onDelivery={vi.fn()}
    onValidate={vi.fn()} onBuild={vi.fn()} onCreate={onCreate} onCreateData={onCreateData}
    sources={<button>broken.yaml</button>}
  />);
  const nav = within(screen.getByRole("navigation", { name: "Project navigation" }));
  fireEvent.click(nav.getByRole("button", { name: "Item Overview" }));
  expect(onSelectTable).toHaveBeenCalledWith("Item");
  fireEvent.click(nav.getByRole("button", { name: /Data · part-b.yaml/ }));
  expect(onSelectFile).toHaveBeenCalledWith("sources/part-b.yaml");
  fireEvent.click(nav.getByRole("button", { name: "+ Data file" }));
  expect(onCreateData).toHaveBeenCalledWith("Item");
  fireEvent.click(nav.getByRole("button", { name: "New Type" }));
  expect(onCreate).toHaveBeenCalledWith("type");
  expect(nav.queryByRole("button", { name: "broken.yaml" })).toBeNull();
  fireEvent.click(nav.getByRole("button", { name: "Source Files" }));
  expect(nav.getByRole("button", { name: "broken.yaml" })).toBeTruthy();
});
