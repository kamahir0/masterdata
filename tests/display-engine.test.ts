import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { parse } from "yaml";
import { DisplayEngine } from "../src/core/display-engine";
import type { DisplayInput, DisplayProgram } from "../src/core/display-model";

const guest = readFileSync("src/core/display-guest.js", "utf8");
const view = parse(
  readFileSync("tests/fixtures/display/drop.yaml.view", "utf8"),
);
const fn = (script: string, name = "率"): DisplayProgram => ({
  name,
  kind: "function",
  script,
});
const rule = (script: string): DisplayProgram => ({
  name: "書式",
  kind: "rule",
  script,
});
function input(records: unknown[], programs: DisplayProgram[]): DisplayInput {
  return {
    records,
    programs,
    columns: [
      { name: "poolId", type: "string", kind: "data" },
      { name: "weight", type: "uint", kind: "data" },
      ...programs
        .filter((p) => p.kind === "function")
        .map((p) => ({ name: p.name, kind: "function" as const })),
    ],
  };
}
test("across all rows, exact group arithmetic and property-wise style precedence", async () => {
  const pool = ["A", "A", "B", "A", "Z", "invalid", "invalid"];
  const records = [20n, 30, 2n, 50n, 0n, 2n, -1n].map((weight, i) => ({
    weight,
    poolId: pool[i],
  }));
  const engine = await DisplayEngine.create(
    input(records, [
      fn(view.functionColumns[0].script),
      rule('return {bold:true, italic:true, color:"#CABBAA"};'),
      rule(
        'return column.kind === "function" ? {bold:false, backgroundColor:"#001122"} : null;',
      ),
      rule('return {text:"overwrite"};'),
    ]),
    guest,
  );
  try {
    const result = engine.evaluate([3, 0, 1, 2, 4, 5, 6], () => {});
    assert.deepEqual(
      result.map((row) => row[2].text),
      ["50%", "20%", "30%", "100%", "—", "—", "—"],
    );
    assert.deepEqual(result[0][2].style, {
      bold: false,
      italic: true,
      color: "#CABBAA",
      backgroundColor: "#001122",
    });
    assert.equal(result[0][0].style?.bold, true);
    assert.match(result[0][0].error!, /未対応/);
    assert.equal(result[0][0].text, undefined);
  } finally {
    engine.dispose();
  }
});
test("bounded JavaScript failures, read-only records, separate contexts and exact ulong", async () => {
  const scripts = [
    "return {text:String(row.id)};",
    'return {text:typeof fetch + "/" + typeof postMessage + "/" + typeof process};',
    'row.id=1n; return {text:"mutated"};',
    "return {",
    "while(true){}",
    'return {text:"x".repeat(48*1024*1024).toUpperCase()};',
    'throw new Error("仕掛かり");',
    'return {text:table.rows.length + "/" + table.group("poolId","A").length};',
  ];
  const engine = await DisplayEngine.create(
    input(
      [{ id: 18446744073709551615n, poolId: "A", weight: 1n }],
      scripts.map((s) => fn(s)),
    ),
    guest,
  );
  try {
    const cells = engine.evaluate([0], () => {})[0].slice(2);
    assert.equal(cells[0].text, "18446744073709551615");
    assert.equal(cells[1].text, "undefined/undefined/undefined");
    for (const i of [2, 3, 4, 5, 6]) assert.equal(cells[i].text, "計算エラー");
    assert.match(cells[3].error!, /構文/);
    assert.match(cells[4].error!, /時間/);
    assert.match(cells[6].error!, /仕掛かり/);
    assert.equal(cells[7].text, "1/1");
  } finally {
    engine.dispose();
  }
});
