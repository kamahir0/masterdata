import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

test("browser WASM executes the shared schema, data, and validation path", async () => {
  const bytes = readFileSync(new URL("../public/masterdata_web.wasm", import.meta.url));
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const wasm = instance.exports;
  const encoder = new TextEncoder();
  const decoder = new TextDecoder();
  function call(request) {
    const input = encoder.encode(JSON.stringify(request));
    const pointer = wasm.md_alloc(input.length);
    new Uint8Array(wasm.memory.buffer, pointer, input.length).set(input);
    const packed = wasm.md_call(pointer, input.length);
    const outputPointer = Number(packed & 0xffff_ffffn);
    const outputLength = Number(packed >> 32n);
    const output = JSON.parse(decoder.decode(new Uint8Array(wasm.memory.buffer, outputPointer, outputLength)));
    wasm.md_free(outputPointer, outputLength);
    return output;
  }
  const files = [
    { path: "schemas/item.yaml", source: "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n" },
    { path: "data/item.yaml", source: "kind: data\ntable: item\nrecords:\n  - id: 1\n" },
    { path: "types/item-id.yaml", source: "kind: type\nname: ItemId\nvalueObject:\n  underlying: int\n" },
  ];
  assert.equal(call({ op: "analyze", files }).validation.valid, true);
  const table = call({ op: "open_table", files, target: "schemas/item.yaml" });
  assert.equal(table.snapshot.schema.table, "item");
  const type = call({ op: "open_type", files, target: "types/item-id.yaml" });
  assert.equal(type.snapshot.category, "Value Object");
  const opened = call({ op: "open_data", files, target: "data/item.yaml" });
  assert.equal(opened.snapshot.rows[0].cells[0].value.value, "1");
  const query = call({ op: "query_data", files, target: "data/item.yaml", mutation: {}, query: { search: "1", filters: [], sort: null } });
  assert.deepEqual(query.result.orderedRecordIndices, [0]);
  const preview = call({ op: "preview_data", files, target: "data/item.yaml", mutation: {
    edits: [{ recordIndex: 0, field: "id", value: { kind: "number", value: "2" } }],
  } });
  assert.match(preview.preview.candidateSource, /id: 2/);
  assert.equal(preview.preview.validation.valid, true);
});
