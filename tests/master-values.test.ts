import { test } from "node:test";
import assert from "node:assert/strict";
import { Session } from "../src/core/session";
import { defaultConfig } from "../src/core/types";
import { resolveMasterValues } from "../src/core/master-values";
import { validate } from "../src/core/validation";
import { parseSource } from "../src/core/yaml";

test("キーと定義名の変更・削除は参照を含めて戻せ、型変更後も元の値を保持する", () => {
  const master = parseSource(
    "settings.yaml",
    `kind: masterValue
table: BattleSettings
keyMode: number
fields: [{name: Limit, key: 1, valueKey: 1, type: int}]
records: [{Limit: 4}]`,
  );
  const quest = parseSource(
    "quest.yaml",
    `kind: schema
table: Quest
fields: [{name: id, key: 0, type: int}, {name: limit, key: 1, type: int}]
primaryKey: {fields: [id]}
records: [{id: 1, limit: {$masterValue: {definition: BattleSettings, key: 1}}}, {id: 2, limit: 3}]`,
  );
  const session = new Session();
  const snapshot = {
    sources: [master, quest],
    folders: [],
    config: defaultConfig(),
  };
  session.load({ root: "", token: "", snapshot, saved: [], configText: "" });
  const current = () => session.state.snapshot;
  const link = () => current().sources[1].data.records[0].limit.$masterValue;
  session.renameMaster(master.id, "BattleSettings", 0, 1001n);
  session.renameDeclaration(master.id, "Battle");
  session.renameField(master.id, 0, "MaxPartySize");
  session.move("settings.yaml", "battle.yaml");
  assert.deepEqual(link(), { definition: "Battle", key: 1001n });
  session.update(master.id, ["records", 0, "MaxPartySize"], 6n);
  assert.deepEqual(validate(current()), []);
  assert.deepEqual(
    resolveMasterValues(current()).snapshot.sources[1].data.records.map(
      (r: { limit: bigint }) => r.limit,
    ),
    [6n, 3n],
  );
  session.update(master.id, ["fields", 0, "type"], "string");
  assert.equal(current().sources[0].data.records[0].MaxPartySize, 6n);
  assert.ok(
    validate(current()).some(
      (p) => p.source === quest.id && p.code === "MASTER_VALUE",
    ),
  );
  session.undo();
  session.removeField(master.id, 0);
  assert.equal(link().key, 1001n);
  assert.ok(
    validate(current()).some(
      (p) => p.source === quest.id && p.code === "MASTER_VALUE",
    ),
  );
  session.undo();
  assert.deepEqual(validate(current()), []);
  session.redo();
  assert.equal(current().sources[0].data.fields.length, 0);
});
