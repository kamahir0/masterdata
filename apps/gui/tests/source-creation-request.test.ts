import { expect, test } from "vitest";
import { buildCreationRequest, move } from "../src/SourceCreation";

const base = {
  root: "/project/sources",
  folder: "",
  filename: "new.yaml",
  name: "",
  table: "",
  csharpName: "",
  underlying: "int",
  fromImplicit: false,
  toImplicit: false,
  fields: [{ key: 0, name: "id", type: "int", nullable: false, array: false }],
  primaryKey: ["id"],
  secondaryKeys: [],
  members: [{ name: "", value: "" }],
  viewColumns: [{ name: "display", expression: "" }],
};

test("Table creation preserves explicit field order while destination stays independent", () => {
  const fields = move([
    { key: 0, name: "id", type: "int", nullable: false, array: false },
    { key: 1, name: "count", type: "int", nullable: false, array: false },
  ], 1, -1);
  const request = buildCreationRequest({
    ...base,
    category: "table",
    filename: "unrelated.yml",
    name: "weapon",
    fields,
  });

  expect(request.destination).toBe("unrelated.yml");
  expect((request.artifact as any).table).toBe("weapon");
  expect((request.artifact as any).fields.map((field: any) => field.name)).toEqual(["count", "id"]);
  expect((request.artifact as any).primaryKey.fields).toEqual(["id"]);
  expect((request.artifact as any).inlineRecords).toBe(true);
  expect((buildCreationRequest({ ...base, category: "table", inlineRecords: false }).artifact as any).inlineRecords).toBe(false);
});

test("Enum creation preserves the full ulong value as text", () => {
  const request = buildCreationRequest({
    ...base,
    category: "enum",
    name: "Kind",
    underlying: "ulong",
    members: [{ name: "Max", value: "18446744073709551615" }],
  });

  expect((request.artifact as any).members[0].value).toBe("18446744073709551615");
});
