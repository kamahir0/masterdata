import { test } from "node:test";
import assert from "node:assert/strict";
import {
  writeTsv,
  readTsv,
  encodeCells,
  decodeCells,
  parseCell,
} from "../src/core/clipboard";
import { recordAllocator } from "../src/core/schema";
import { defaultConfig, tables, type Snapshot } from "../src/core/types";
test("TSV round-trips tab, multiline, quotes, empty cells and CRLF records", () => {
  const rows = [
    ["a\tb", "line\nnext", '"quoted"', ""],
    ["", "last"],
  ];
  assert.deepEqual(readTsv(writeTsv(rows)), rows);
  assert.deepEqual(readTsv("a\tb\r\nc\td\r\n"), [
    ["a", "b"],
    ["c", "d"],
  ]);
});
test("editor clipboard keeps null, literal null text, empty text, bigint and nested shapes distinct", () => {
  const values = [
    [null, "null", "", 18446744073709551615n, { a: [false, 10n] }],
  ];
  assert.deepEqual(decodeCells(encodeCells(values)), values);
  const snapshot: Snapshot = {
    config: defaultConfig(),
    folders: [],
    sources: [],
  };
  assert.equal(
    parseCell(snapshot, { name: "s", type: "string", nullable: true }, "null"),
    "null",
  );
  assert.equal(
    parseCell(snapshot, { name: "n", type: "ulong" }, "18446744073709551615"),
    18446744073709551615n,
  );
});
test("bulk row allocation remains unique for composite keys ending in Enum and multiple unique constraints", () => {
  const snapshot: Snapshot = {
    config: defaultConfig(),
    folders: [],
    sources: [
      {
        id: "t",
        path: "t.yaml",
        data: {
          kind: "schema",
          table: "t",
          fields: [
            { name: "id", type: "int" },
            { name: "locale", type: "Language" },
            { name: "code", type: "string" },
          ],
          primaryKey: { fields: ["id", "locale"] },
          secondaryKeys: [{ fields: ["code"] }],
          records: [{ id: 0n, locale: "Ja", code: "" }],
        },
      },
      {
        id: "e",
        path: "e.yaml",
        data: {
          kind: "type",
          name: "Language",
          enum: { underlying: "int", members: [{ name: "Ja", value: 0n }] },
        },
      },
    ],
  };
  const allocate = recordAllocator(snapshot, tables(snapshot)[0]);
  const records = Array.from({ length: 1000 }, () => allocate());
  assert.equal(new Set(records.map((r) => String(r.id))).size, 1000);
  assert.equal(new Set(records.map((r) => r.code)).size, 1000);
});
