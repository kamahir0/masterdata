import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { runCli } from "../cli/index";
import { projectFileName } from "../desktop/config";

async function tempDir(t: { after(fn: () => Promise<void>): void }) {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), "masterdata-cli-"));
  t.after(() => fs.rm(dir, { recursive: true, force: true }));
  return dir;
}

test("CLI-001: init, parent project discovery, project file validation and 64-bit ulong precision", async (t) => {
  const root = await tempDir(t);
  const initRes = await runCli(["init", root, "--name", "CLIテスト", "--json"]);
  assert.equal(initRes.exitCode, 0, initRes.stderr);
  const initData = JSON.parse(initRes.stdout);
  assert.equal(initData.ok, true);
  assert.equal(initData.name, "CLIテスト");

  // Re-init on existing project must fail without overwriting
  const duplicateInit = await runCli(["init", root, "--name", "別名"]);
  assert.equal(duplicateInit.exitCode, 1);

  // Reject arbitrary TOML / YAML passed to --project
  const otherToml = path.join(root, "other.toml");
  await fs.writeFile(otherToml, 'name = "wrong"\n');
  const badProj = await runCli(["inspect", "--project", otherToml]);
  assert.equal(badProj.exitCode, 1);
  assert.match(badProj.stderr, /masterdata\.toml/);

  // Create a table with ulong and verify precision > Number.MAX_SAFE_INTEGER from nested cwd
  const nestedCwd = path.join(root, "sources", "sub");
  await fs.mkdir(nestedCwd, { recursive: true });
  const txRes = await runCli(["tx", "--json"], {
    cwd: nestedCwd,
    stdin: JSON.stringify([
      {
        op: "table.create",
        name: "account",
        fields: [
          { key: 0, name: "id", type: "ulong" },
          { key: 1, name: "code", type: "string" },
        ],
        pk: ["id"],
      },
      {
        op: "record.add",
        table: "account",
        records: [{ id: "18446744073709551614", code: "007" }],
      },
    ]),
  });
  assert.equal(txRes.exitCode, 0, txRes.stderr || txRes.stdout);

  // Auto-increment ulong from 18446744073709551614 -> 18446744073709551615
  const addRes = await runCli(
    ["record", "add", "account", "code=008", "--json"],
    { cwd: nestedCwd },
  );
  assert.equal(addRes.exitCode, 0, addRes.stderr || addRes.stdout);
  assert.match(addRes.stdout, /18446744073709551615/);

  const getRes = await runCli(
    ["record", "get", "account", "18446744073709551615", "--json"],
    { cwd: root },
  );
  assert.equal(getRes.exitCode, 0, getRes.stderr || getRes.stdout);
  assert.match(getRes.stdout, /18446744073709551615/);
  assert.match(getRes.stdout, /"code": "008"/);

  const yamlText = await fs.readFile(
    path.join(root, "sources/account.yaml"),
    "utf8",
  );
  assert.match(yamlText, /18446744073709551614/);
  assert.match(yamlText, /18446744073709551615/);
  assert.match(yamlText, /"007"|007/);
});

test("CLI-002: error tolerance on broken/invalid files vs quality gates and --strict", async (t) => {
  const root = await tempDir(t);
  await fs.cp(path.resolve("samples/Adventure"), root, { recursive: true });

  // 1. Add an unrelated YAML file with a syntax error
  const brokenPath = path.join(root, "sources/broken.yaml");
  const brokenText = "kind: schema\ntable: broken\nfields: [\n";
  await fs.writeFile(brokenPath, brokenText);

  // inspect succeeds (exitCode 0) and reports syntaxErrorCount: 1
  const inspectRes = await runCli(["inspect", "--json"], { cwd: root });
  assert.equal(inspectRes.exitCode, 0);
  const inspectData = JSON.parse(inspectRes.stdout);
  assert.equal(inspectData.problems.syntaxErrorCount, 1);
  assert.equal(inspectData.problems.syntaxErrors[0].path, "broken.yaml");

  // Editing an unrelated valid table (item) succeeds and leaves broken.yaml untouched
  const editValid = await runCli(
    ["record", "set", "item", "--pk", "1001", "price=999", "--json"],
    { cwd: root },
  );
  assert.equal(editValid.exitCode, 0, editValid.stderr || editValid.stdout);
  assert.equal(await fs.readFile(brokenPath, "utf8"), brokenText);

  // validate fails (exitCode 1) because of broken.yaml
  const valBroken = await runCli(["validate", "--json"], { cwd: root });
  assert.equal(valBroken.exitCode, 1);
  const valBrokenData = JSON.parse(valBroken.stdout);
  assert.ok(valBrokenData.problems.some((p: any) => p.code === "YAML"));

  // Remove broken.yaml and introduce a semantic error (type change bool -> int without converting existing values)
  await fs.unlink(brokenPath);
  const colAdd = await runCli(
    ["table", "column", "add", "item", "sellable", "bool"],
    { cwd: root },
  );
  assert.equal(colAdd.exitCode, 0, colAdd.stderr);
  const colTypeChange = await runCli(
    ["table", "column", "set", "item", "sellable", "--type", "int", "--json"],
    { cwd: root },
  );
  // Changing type succeeds (exitCode 0) and preserves existing bool values, reporting problems > 0
  assert.equal(colTypeChange.exitCode, 0);
  const colTypeData = JSON.parse(colTypeChange.stdout);
  assert.ok(colTypeData.problemCount > 0);

  // --strict rejects further edits while semantic problems remain, without writing to disk
  const strictRes = await runCli(
    ["record", "set", "item", "--pk", "1001", "price=555", "--strict", "--json"],
    { cwd: root },
  );
  assert.equal(strictRes.exitCode, 1);
  const itemAfterStrict = await runCli(
    ["record", "get", "item", "1001", "--json"],
    { cwd: root },
  );
  assert.equal(JSON.parse(itemAfterStrict.stdout).record.price, 999);

  // Broken masterdata.toml rejects all commands except init
  const tomlFile = path.join(root, projectFileName);
  const validToml = await fs.readFile(tomlFile, "utf8");
  await fs.writeFile(tomlFile, 'name = ""\n');
  const badConfigRes = await runCli(["inspect"], { cwd: root });
  assert.equal(badConfigRes.exitCode, 1);
  await fs.writeFile(tomlFile, validToml);
});

test("CLI-003 & CLI-004: preserve array order, YAML comments/CRLF/$tags, cascading rename and fix", async (t) => {
  const root = await tempDir(t);
  await runCli(["init", root, "--name", "OrderAndAst"]);

  // Write a schema file with CRLF, comments, custom key numbers (not 0,1,2 order), and $tags
  const itemFile = path.join(root, "sources/item.yaml");
  const initialYaml = [
    "# アイテム定義コメント",
    "kind: schema",
    "table: item",
    "fields:",
    "  - key: 10",
    "    name: code",
    "    type: string",
    "  - key: 0",
    "    name: id",
    "    type: int",
    "  - key: 5",
    "    name: price",
    "    type: int",
    "primaryKey:",
    "  fields:",
    "    - id",
    "records:",
    "  # 先頭レコードのコメント",
    "  - $tags:",
    "      - event_2026",
    "    code: sword_b",
    "    id: 20",
    "    price: 500",
    "  - code: sword_a",
    "    id: 10",
    "    price: 300",
    "",
  ].join("\r\n");
  await fs.writeFile(itemFile, initialYaml);

  // Update one cell via CLI
  const setRes = await runCli(
    ["record", "set", "item", "--pk", "20", "price=600"],
    { cwd: root },
  );
  assert.equal(setRes.exitCode, 0, setRes.stderr);

  const afterYaml = await fs.readFile(itemFile, "utf8");
  // CRLF preserved
  assert.ok(afterYaml.includes("\r\n"));
  // Comments preserved
  assert.ok(afterYaml.includes("# アイテム定義コメント"));
  assert.ok(afterYaml.includes("# 先頭レコードのコメント"));
  // $tags preserved
  assert.ok(afterYaml.includes("event_2026"));
  // Field order (code -> id -> price) and keys (10, 0, 5) preserved
  const inspectRes = await runCli(["inspect", "item", "--json"], { cwd: root });
  const detail = JSON.parse(inspectRes.stdout).detail;
  assert.deepEqual(
    detail.fields.map((f: any) => [f.name, f.key]),
    [
      ["code", 10],
      ["id", 0],
      ["price", 5],
    ],
  );
  // Record order (id 20 first, id 10 second) preserved
  assert.deepEqual(detail.primaryKeyValues, [20, 10]);

  // Column move changes array order without changing MessagePack keys
  const moveRes = await runCli(
    ["table", "column", "move", "item", "id", "--to", "0"],
    { cwd: root },
  );
  assert.equal(moveRes.exitCode, 0, moveRes.stderr);
  const afterMove = JSON.parse(
    (await runCli(["inspect", "item", "--json"], { cwd: root })).stdout,
  ).detail;
  assert.deepEqual(
    afterMove.fields.map((f: any) => [f.name, f.key]),
    [
      ["id", 0],
      ["code", 10],
      ["price", 5],
    ],
  );

  // Create Enum, Flags, Custom, and referencing Quest table, then test cascading renames
  const setupTx = await runCli(["tx", "--json"], {
    cwd: root,
    stdin: JSON.stringify([
      {
        op: "type.create",
        kind: "enum",
        name: "Rarity",
        members: ["Normal", "Rare"],
      },
      {
        op: "type.create",
        kind: "flags",
        name: "ElementFlags",
        members: ["Fire", "Ice"],
      },
      {
        op: "type.create",
        kind: "custom",
        name: "RewardEntry",
        fields: [
          { key: 0, name: "itemId", type: "int" },
          { key: 1, name: "rarity", type: "Rarity" },
        ],
      },
      {
        op: "table.column.add",
        table: "item",
        name: "elements",
        type: "ElementFlags",
      },
      {
        op: "table.create",
        name: "quest",
        fields: [
          { key: 0, name: "id", type: "int" },
          { key: 1, name: "mainItemId", type: "int" },
          { key: 2, name: "reward", type: "RewardEntry" },
        ],
        pk: ["id"],
      },
      {
        op: "table.ref.add",
        table: "quest",
        name: "mainItem",
        fields: ["mainItemId"],
        targetTable: "item",
        targetFields: ["id"],
      },
      {
        op: "record.add",
        table: "quest",
        records: [
          {
            id: 1,
            mainItemId: 20,
            reward: { itemId: 20, rarity: "Normal" },
          },
        ],
      },
      {
        op: "record.set",
        table: "item",
        pk: 20,
        set: { elements: ["Fire", "Ice"] },
      },
    ]),
  });
  assert.equal(setupTx.exitCode, 0, setupTx.stderr || setupTx.stdout);

  // Flags None=0 protection
  const removeNone = await runCli(
    ["type", "member", "remove", "ElementFlags", "None"],
    { cwd: root },
  );
  assert.equal(removeNone.exitCode, 1);

  // Cascading renames: enum member, flags member, custom field, table column, and table name
  for (const args of [
    ["rename", "member", "Rarity", "Normal", "Common"],
    ["rename", "member", "ElementFlags", "Fire", "Flame"],
    ["rename", "field", "RewardEntry", "itemId", "rewardItemId"],
    ["rename", "field", "item", "id", "itemKey"],
    ["rename", "table", "item", "game_item"],
  ]) {
    const res = await runCli(args, { cwd: root });
    assert.equal(res.exitCode, 0, `${args.join(" ")}: ${res.stderr}`);
  }

  // Validate entire project after cascading renames: 0 errors!
  const valAfterRename = await runCli(["validate", "--json"], { cwd: root });
  assert.equal(valAfterRename.exitCode, 0, valAfterRename.stdout);

  const questRec = JSON.parse(
    (await runCli(["record", "get", "quest", "1", "--json"], { cwd: root }))
      .stdout,
  ).record;
  assert.deepEqual(questRec.reward, {
    rewardItemId: 20,
    rarity: "Common",
  });

  const itemRec = JSON.parse(
    (
      await runCli(["record", "get", "game_item", "20", "--json"], {
        cwd: root,
      })
    ).stdout,
  ).record;
  assert.deepEqual(itemRec.elements, ["Flame", "Ice"]);

  // Test `fix` backfilling missing fields on hand-edited YAML without disturbing order
  await fs.writeFile(
    path.join(root, "sources/quest.yaml"),
    `kind: schema
table: quest
fields:
  - {name: id, type: int}
  - {name: mainItemId, type: int}
  - {name: reward, type: RewardEntry}
primaryKey: {fields: [id]}
records:
  - {id: 1, mainItemId: 20}
`,
  );
  const fixRes = await runCli(["fix", "--table", "quest", "--json"], {
    cwd: root,
  });
  assert.equal(fixRes.exitCode, 0, fixRes.stderr);
  const fixedQuest = JSON.parse(
    (await runCli(["record", "get", "quest", "1", "--json"], { cwd: root }))
      .stdout,
  ).record;
  assert.deepEqual(fixedQuest.reward, {
    rewardItemId: 0,
    rarity: "Common",
  });
});

test("CLI-005: multi-source record add rule, --where filtering, nested path updates and tx atomic rollback", async (t) => {
  const root = await tempDir(t);
  await fs.cp(path.resolve("samples/Adventure"), root, { recursive: true });

  // `item` in samples/Adventure has multiple sources (`items.yaml` and `items-extra.yaml` etc. or let's create a split data file if needed)
  const splitRes = await runCli(
    ["table", "data-file", "create", "quest", "quests-event.yaml"],
    { cwd: root },
  );
  assert.equal(splitRes.exitCode, 0, splitRes.stderr);

  // Adding to a multi-source table without --source must be rejected
  const addNoSource = await runCli(
    ["record", "add", "quest", "name=新クエスト", "rewardItemId=1001", "rewardAmount=1"],
    { cwd: root },
  );
  assert.equal(addNoSource.exitCode, 1);
  assert.match(addNoSource.stderr, /--source/);

  // Adding with explicit --source succeeds and puts the record in quests-event.yaml
  const addWithSource = await runCli(
    [
      "record",
      "add",
      "quest",
      "name=イベント討伐",
      "rewardItemId=1001",
      "rewardAmount=2",
      "--source",
      "quests-event.yaml",
      "--tags",
      "event",
      "--json",
    ],
    { cwd: root },
  );
  assert.equal(addWithSource.exitCode, 0, addWithSource.stderr);
  const addedPk = JSON.parse(addWithSource.stdout).result.records[0].id;
  assert.equal(addedPk, 2);

  // Update nested array element on `item` (id=2001 has effects: []) via dot/bracket path
  const appendEffect = await runCli(
    [
      "record",
      "set",
      "item",
      "--pk",
      "2001",
      'effects[+]={"name":"氷結","enabled":true,"values":[5,10],"stats":{"attack":10,"defense":2,"enabled":true,"critical":false}}',
      "--add-tag",
      "limited",
      "--json",
    ],
    { cwd: root },
  );
  assert.equal(appendEffect.exitCode, 0, appendEffect.stderr);

  const updateNested = await runCli(
    [
      "record",
      "set",
      "item",
      "--pk",
      "2001",
      "effects[0].stats.attack=25",
      "--json",
    ],
    { cwd: root },
  );
  assert.equal(updateNested.exitCode, 0, updateNested.stderr);

  // Filter with --where on nested/tags
  const listEvent = await runCli(
    [
      "record",
      "list",
      "item",
      "--where",
      "$tags=limited",
      "--where",
      "effects[0].stats.attack>=25",
      "--json",
    ],
    { cwd: root },
  );
  assert.equal(listEvent.exitCode, 0);
  const listData = JSON.parse(listEvent.stdout);
  assert.equal(listData.totalMatched, 1);
  assert.equal(listData.records[0].record.id, 2001);

  // Atomic rollback on tx failure: if 2nd op fails, 1st op is not written to disk
  const beforeItem = await fs.readFile(
    path.join(root, "sources/items/common.yaml"),
    "utf8",
  );
  const failedTx = await runCli(["tx", "--json"], {
    cwd: root,
    stdin: JSON.stringify([
      { op: "record.set", table: "item", pk: 1001, set: { price: 77777 } },
      { op: "record.set", table: "non_existent_table", pk: 1, set: { a: 1 } },
    ]),
  });
  assert.equal(failedTx.exitCode, 1);
  assert.equal(
    await fs.readFile(path.join(root, "sources/items/common.yaml"), "utf8"),
    beforeItem,
  );
});

test(
  "CLI-006: build and publish enforce freshness and protect unmanaged files",
  { timeout: 180000, skip: !!process.env.SKIP_NATIVE_BUILD },
  async (t) => {
    const root = await tempDir(t);
    await fs.cp(path.resolve("samples/Adventure"), root, { recursive: true });

    // Publish before build fails
    const pubBeforeBuild = await runCli(["publish"], { cwd: root });
    assert.equal(pubBeforeBuild.exitCode, 1);
    assert.match(pubBeforeBuild.stderr, /build/);

    // Build succeeds
    const buildRes = await runCli(["build", "--json"], { cwd: root });
    assert.equal(buildRes.exitCode, 0, buildRes.stderr || buildRes.stdout);
    const buildData = JSON.parse(buildRes.stdout);
    assert.equal(buildData.ok, true);
    assert.ok(buildData.result.records > 0);

    // Modify a source after build -> publish without --build must reject stale build
    await runCli(["record", "set", "item", "--pk", "1001", "price=250"], {
      cwd: root,
    });
    const stalePub = await runCli(["publish"], { cwd: root });
    assert.equal(stalePub.exitCode, 1);
    assert.match(stalePub.stderr, /変更されています/);

    // Publish with --build rebuilds and publishes safely while preserving unmanaged files
    const generatedDir = path.join(root, "output", "Generated");
    await fs.mkdir(generatedDir, { recursive: true });
    await fs.writeFile(path.join(generatedDir, "CustomHelper.cs"), "// keep");
    await fs.writeFile(
      path.join(generatedDir, "Item.g.cs.meta"),
      "guid: unity-meta",
    );

    const freshPub = await runCli(["publish", "--build", "--json"], {
      cwd: root,
    });
    assert.equal(freshPub.exitCode, 0, freshPub.stderr || freshPub.stdout);
    assert.equal(
      await fs.readFile(path.join(generatedDir, "CustomHelper.cs"), "utf8"),
      "// keep",
    );
    assert.equal(
      await fs.readFile(path.join(generatedDir, "Item.g.cs.meta"), "utf8"),
      "guid: unity-meta",
    );
  },
);

test("CLI-004 & CLI-005: config, type/table subcommands, dry-run, target syntax error protection and main.ts entrypoint", async (t) => {
  const root = await tempDir(t);
  await runCli(["init", root, "--name", "Subcommands"]);

  // 1. config set and profile management
  assert.equal(
    (
      await runCli(
        [
          "config",
          "set",
          "--name",
          "変更後プロジェクト",
          "--publish-csharp",
          "out/cs",
          "--publish-binary",
          "out/master.bytes",
        ],
        { cwd: root },
      )
    ).exitCode,
    0,
  );
  assert.equal(
    (
      await runCli(
        [
          "config",
          "profile",
          "set",
          "event_only",
          "--include",
          "event",
          "--exclude",
          "debug",
        ],
        { cwd: root },
      )
    ).exitCode,
    0,
  );
  const cfgShow = JSON.parse(
    (await runCli(["config", "show", "--json"], { cwd: root })).stdout,
  );
  assert.equal(cfgShow.config.name, "変更後プロジェクト");
  assert.deepEqual(cfgShow.config.profiles.event_only, {
    include: ["event"],
    exclude: ["debug"],
  });
  assert.equal(
    (
      await runCli(["config", "profile", "remove", "event_only"], {
        cwd: root,
      })
    ).exitCode,
    0,
  );

  // 2. valueObject, flags bit auto-allocation, custom field operations
  assert.equal(
    (
      await runCli(
        ["type", "create", "value-object", "ItemId", "--underlying", "uint"],
        { cwd: root },
      )
    ).exitCode,
    0,
  );
  assert.equal(
    (
      await runCli(["type", "set-underlying", "ItemId", "int"], { cwd: root })
    ).exitCode,
    0,
  );
  assert.equal(
    (
      await runCli(
        ["type", "create", "flags", "BuffFlags", "--members", "AtkUp,DefUp"],
        { cwd: root },
      )
    ).exitCode,
    0,
  );
  // Add 3rd flag without --value -> auto assigns 4 (1 << 2)
  const addFlagRes = JSON.parse(
    (
      await runCli(
        ["type", "member", "add", "BuffFlags", "SpdUp", "--json"],
        { cwd: root },
      )
    ).stdout,
  );
  assert.equal(addFlagRes.result.member.value, 4);

  // Cannot move None or move another member to index 0 on Flags
  assert.equal(
    (
      await runCli(
        ["type", "member", "move", "BuffFlags", "SpdUp", "--to", "0"],
        { cwd: root },
      )
    ).exitCode,
    1,
  );
  assert.equal(
    (
      await runCli(
        ["type", "member", "move", "BuffFlags", "SpdUp", "--to", "1"],
        { cwd: root },
      )
    ).exitCode,
    0,
  );

  // Custom type and field subcommands
  assert.equal(
    (
      await runCli(["type", "create", "custom", "Bonus"], { cwd: root })
    ).exitCode,
    0,
  );
  assert.equal(
    (
      await runCli(["type", "field", "add", "Bonus", "rate", "float"], {
        cwd: root,
      })
    ).exitCode,
    0,
  );
  assert.equal(
    (
      await runCli(
        ["type", "field", "add", "Bonus", "tags", "string", "--array"],
        { cwd: root },
      )
    ).exitCode,
    0,
  );
  assert.equal(
    (
      await runCli(
        ["type", "field", "move", "Bonus", "tags", "--to", "0"],
        { cwd: root },
      )
    ).exitCode,
    0,
  );
  assert.equal(
    (
      await runCli(["type", "field", "remove", "Bonus", "rate"], { cwd: root })
    ).exitCode,
    0,
  );

  // 3. Table keys, secondary keys, --dry-run, record delete, source move
  assert.equal(
    (
      await runCli(
        [
          "table",
          "create",
          "shop_item",
          "--fields",
          "shopId:int,itemId:ItemId,price:int",
          "--pk",
          "shopId,itemId",
        ],
        { cwd: root },
      )
    ).exitCode,
    0,
  );
  assert.equal(
    (
      await runCli(
        [
          "table",
          "key",
          "add-sk",
          "shop_item",
          "price",
          "--non-unique",
        ],
        { cwd: root },
      )
    ).exitCode,
    0,
  );
  assert.equal(
    (
      await runCli(
        ["record", "add", "shop_item", "shopId=1", "itemId=10", "price=500"],
        { cwd: root },
      )
    ).exitCode,
    0,
  );

  // --dry-run does not modify disk
  const dryRes = await runCli(
    [
      "record",
      "set",
      "shop_item",
      "--pk",
      "shopId=1,itemId=10",
      "price=999",
      "--dry-run",
      "--json",
    ],
    { cwd: root },
  );
  assert.equal(dryRes.exitCode, 0);
  assert.equal(JSON.parse(dryRes.stdout).dryRun, true);
  const recStill500 = JSON.parse(
    (
      await runCli(
        ["record", "get", "shop_item", "shopId=1,itemId=10", "--json"],
        { cwd: root },
      )
    ).stdout,
  ).record;
  assert.equal(recStill500.price, 500);

  // Move source file and delete record
  assert.equal(
    (
      await runCli(
        ["source", "move", "shop_item.yaml", "shops/shop_item.yaml"],
        { cwd: root },
      )
    ).exitCode,
    0,
  );
  assert.equal(
    (
      await runCli(
        ["record", "delete", "shop_item", "--pk", "shopId=1,itemId=10"],
        { cwd: root },
      )
    ).exitCode,
    0,
  );

  // 4. Target syntax error protection: if a split data file of shop_item has a YAML syntax error,
  // table-wide operations on shop_item are refused
  const brokenSplit = path.join(root, "sources/shops/broken_split.yaml");
  await fs.writeFile(
    brokenSplit,
    "kind: data\ntable: shop_item\nrecords: [\n",
  );
  const blockedAddCol = await runCli(
    ["table", "column", "add", "shop_item", "stock", "int"],
    { cwd: root },
  );
  assert.equal(blockedAddCol.exitCode, 1);
  assert.match(blockedAddCol.stderr, /YAML構文エラー/);
});

