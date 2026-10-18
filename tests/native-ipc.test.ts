import { test } from "node:test";
import assert from "node:assert/strict";
import { ProjectService } from "../desktop/service";
import { encode, decode } from "../src/core/wire";
test("IPC preserves uint64 and arbitrary object shapes without reserved data keys", () => {
  const value = {
    n: 18446744073709551615n,
    min: -9223372036854775808n,
    array: [1n, null],
    __bigint: "user-data",
    nested: { value: 2n },
  };
  assert.deepEqual(decode(encode(value)), value);
  assert.equal(decode(encode(1n)), 1n);
  assert.deepEqual(
    decode(
      encode({
        nan: NaN,
        positive: Infinity,
        negative: -Infinity,
        zero: -0,
        unset: undefined,
      }),
    ),
    {
      nan: NaN,
      positive: Infinity,
      negative: -Infinity,
      zero: -0,
      unset: undefined,
    },
  );
});
test("native service rejects unknown commands and stale tokens", async () => {
  const service = new ProjectService();
  assert.equal(await service.request("initial", []), null);
  await assert.rejects(service.request("arbitrary-file", []), /許可/);
  await assert.rejects(
    service.request("save", [{ token: "stale" }]),
    /切り替わって/,
  );
});
