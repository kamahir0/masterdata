import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { loadWorkspace, saveWorkspace } from "../desktop/files";
import { Session } from "../src/core/session";
import { validate } from "../src/core/validation";
import { tables, fieldsOf } from "../src/core/types";
import { defaultValue, flagsChange } from "../src/core/schema";
import { serialize, parseSource } from "../src/core/yaml";
async function sample() {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "masterdata-test-"));
  await fs.cp(path.resolve("samples/Adventure"), root, { recursive: true });
  return { root, ...(await loadWorkspace(root)) };
}
test("sample includes nested nullable, full ulong, references and profiles without errors", async () => {
  const { project, root } = await sample();
  try {
    assert.deepEqual(validate(project.snapshot), []);
    assert.deepEqual(validate(project.snapshot, "production"), []);
    const item = project.snapshot.sources.find(
      (s) => s.path === "items/common.yaml",
    )!;
    assert.equal(item.data.records[0].maxPower, 18446744073709551615n);
  } finally {
    await fs.rm(root, { recursive: true });
  }
});
test("invalid type edits preserve values, allow unrelated operations, and Undo restores exact state", async () => {
  const { project, root } = await sample();
  try {
    const s = new Session();
    s.load(project);
    const table = tables(s.state.snapshot).find((t) => t.name === "item")!;
    const i = table.fields.findIndex((f) => f.name === "enabled");
    const original = s.state.snapshot;
    s.update(table.source.id, ["fields", i, "type"], "int");
    assert.equal(
      s.state.snapshot.sources.find((f) => f.path === "items/common.yaml")!.data
        .records[0].enabled,
      true,
    );
    assert(validate(s.state.snapshot).some((p) => p.path.includes("enabled")));
    s.addField(table.source.id);
    s.undo();
    s.undo();
    assert.equal(s.state.snapshot, original);
    assert.equal(s.dirty, false);
  } finally {
    await fs.rm(root, { recursive: true });
  }
});
test("Save does not clear history; Undo after Save becomes dirty and can be saved", async () => {
  const { project, workspace, root } = await sample();
  try {
    const s = new Session();
    s.load(project);
    const data = project.snapshot.sources.find(
      (s) => s.path === "items/common.yaml",
    )!;
    s.update(data.id, ["records", 0, "name"], "編集後");
    const captured = s.state.snapshot;
    const result = await saveWorkspace(workspace, {
      token: project.token,
      files: s.files(),
      folders: captured.folders,
      config: captured.config,
    });
    s.acceptSave(result, captured);
    assert.equal(s.dirty, false);
    assert(s.state.past.length);
    s.undo();
    assert(s.dirty);
    assert.equal(
      s.state.snapshot.sources.find((f) => f.id === data.id)!.data.records[0]
        .name,
      "銅の剣",
    );
    const next = s.state.snapshot;
    s.acceptSave(
      await saveWorkspace(workspace, {
        token: project.token,
        files: s.files(),
        folders: next.folders,
        config: next.config,
      }),
      next,
    );
    assert.equal(s.dirty, false);
  } finally {
    await fs.rm(root, { recursive: true });
  }
});
test("file moves and folder deletes are undoable drafts; Save moves files without losing values", async () => {
  const { project, workspace, root } = await sample();
  try {
    const s = new Session();
    s.load(project);
    s.move("types", "definitions");
    assert(
      s.state.snapshot.sources.some((f) => f.path.startsWith("definitions/")),
    );
    assert(await fs.stat(path.join(root, "sources/types")));
    s.remove(["definitions"]);
    assert(!s.state.snapshot.sources.some((f) => f.data?.kind === "type"));
    s.undo();
    const captured = s.state.snapshot;
    const result = await saveWorkspace(workspace, {
      token: project.token,
      files: s.files(),
      folders: captured.folders,
      config: captured.config,
    });
    s.acceptSave(result, captured);
    assert.equal(
      (await loadWorkspace(root)).project.snapshot.sources.filter((f) =>
        f.path.startsWith("definitions/"),
      ).length,
      5,
    );
    s.undo();
    assert(s.dirty);
  } finally {
    await fs.rm(root, { recursive: true });
  }
});
test("external conflict preflight prevents every file write", async () => {
  const { project, workspace, root } = await sample();
  try {
    const s = new Session();
    s.load(project);
    const a = project.snapshot.sources.find(
      (s) => s.path === "items/common.yaml",
    )!;
    const b = project.snapshot.sources.find(
      (s) => s.path === "items/schema.yaml",
    )!;
    s.update(a.id, ["records", 0, "price"], 500n);
    s.update(b.id, ["csharpName"], "AnotherItem");
    await fs.appendFile(
      path.join(root, "sources", b.path),
      "\n# external change\n",
    );
    await assert.rejects(
      saveWorkspace(workspace, {
        token: project.token,
        files: s.files(),
        folders: s.state.snapshot.folders,
        config: s.state.snapshot.config,
      }),
      /外部で変更/,
    );
    assert.equal(
      await fs.readFile(path.join(root, "sources", a.path), "utf8"),
      project.saved.find((f) => f.id === a.id)!.text,
    );
  } finally {
    await fs.rm(root, { recursive: true });
  }
});
test("nullable defaults, nested bool and flags None share one semantic rule", async () => {
  const { project, root } = await sample();
  try {
    const s = project.snapshot;
    assert.equal(
      defaultValue(s, { name: "a", type: "bool", nullable: true }),
      null,
    );
    assert.equal(defaultValue(s, { name: "a", type: "bool" }), false);
    assert.deepEqual(
      defaultValue(s, { name: "a", type: "Effect", array: true }),
      [],
    );
    assert.equal(
      defaultValue(s, { name: "a", type: "CombatStats" }).enabled,
      false,
    );
    assert.deepEqual(flagsChange(["None"], ["None", "Weapon"]), ["Weapon"]);
    assert.deepEqual(flagsChange(["Weapon"], ["Weapon", "None"]), ["None"]);
    assert.deepEqual(flagsChange(["Weapon"], []), ["None"]);
  } finally {
    await fs.rm(root, { recursive: true });
  }
});
test("YAML serialization preserves untouched comments, quotes, bigint, and malformed text is repairable", () => {
  const input =
    '# retained\nkind: data\ntable: item\nrecords:\n  - {id: 18446744073709551615, name: "quoted", enabled: true}\n';
  const s = parseSource("a.yaml", input);
  const changed = {
    ...s,
    data: { ...s.data, records: [{ ...s.data.records[0], enabled: false }] },
  };
  const output = serialize(changed, { id: s.id, path: s.path, text: input });
  assert(output.includes("# retained"));
  assert(output.includes('"quoted"'));
  assert(output.includes("18446744073709551615"));
  assert.equal(parseSource("a.yaml", output).data.records[0].enabled, false);
  const bad = parseSource("bad.yaml", "records: [broken");
  assert(bad.syntaxError);
  assert.equal(serialize(bad), "records: [broken");
});
test("rename field groups schema, records, keys, and target references into one Undo", async () => {
  const { project, root } = await sample();
  try {
    const s = new Session();
    s.load(project);
    const table = tables(s.state.snapshot).find((t) => t.name === "item")!;
    s.renameField(table.source.id, 0, "itemId");
    assert.deepEqual(
      s.state.snapshot.sources.find((f) => f.path === "quests/quest.yaml")!.data
        .references[0].target.fields,
      ["itemId"],
    );
    assert.equal(
      s.state.snapshot.sources.find((f) => f.path === "items/common.yaml")!.data
        .records[0].itemId,
      1001n,
    );
    assert.deepEqual(validate(s.state.snapshot), []);
    s.undo();
    assert.equal(s.dirty, false);
  } finally {
    await fs.rm(root, { recursive: true });
  }
});
test("adding and removing a custom field initializes nested instances in one Undo group", async () => {
  const { project, root } = await sample();
  try {
    const s = new Session();
    s.load(project);
    const custom = s.state.snapshot.sources.find(
      (f) => f.data?.name === "CombatStats",
    )!;
    s.addField(custom.id);
    let item = s.state.snapshot.sources.find(
      (f) => f.path === "items/common.yaml",
    )!;
    assert.equal(item.data.records[0].stats.field1, "");
    assert.equal(item.data.records[0].effects[0].stats.field1, "");
    s.renameField(custom.id, 4, "luck");
    item = s.state.snapshot.sources.find(
      (f) => f.path === "items/common.yaml",
    )!;
    assert.equal(item.data.records[0].effects[0].stats.luck, "");
    s.removeField(custom.id, 4);
    assert.equal(
      fieldsOf(s.state.snapshot.sources.find((f) => f.id === custom.id)).length,
      4,
    );
    s.undo();
    assert.equal(
      fieldsOf(s.state.snapshot.sources.find((f) => f.id === custom.id)).at(-1)
        ?.name,
      "luck",
    );
  } finally {
    await fs.rm(root, { recursive: true });
  }
});
test("Enum rename updates typed data without rewriting unrelated text", async () => {
  const { project, root } = await sample();
  try {
    const s = new Session();
    s.load(project);
    const type = project.snapshot.sources.find(
      (f) => f.data?.name === "Rarity",
    )!;
    s.renameMember(type.id, 0, "Normal");
    const item = s.state.snapshot.sources.find(
      (f) => f.path === "items/common.yaml",
    )!;
    assert.equal(item.data.records[0].rarity, "Normal");
    assert.equal(item.data.records[0].name, "銅の剣");
    assert.deepEqual(validate(s.state.snapshot), []);
    s.undo();
    assert.equal(s.dirty, false);
  } finally {
    await fs.rm(root, { recursive: true });
  }
});
test("semantic and YAML syntax errors can be saved without implicit validation", async () => {
  const { project, workspace, root } = await sample();
  try {
    const s = new Session();
    s.load(project);
    const source = s.state.snapshot.sources.find(
      (f) => f.path === "types/rarity.yaml",
    )!;
    s.raw(source.id, "kind: [unfinished");
    const captured = s.state.snapshot;
    s.acceptSave(
      await saveWorkspace(workspace, {
        token: project.token,
        files: s.files(),
        folders: captured.folders,
        config: captured.config,
      }),
      captured,
    );
    assert.equal(
      await fs.readFile(path.join(root, "sources/types/rarity.yaml"), "utf8"),
      "kind: [unfinished",
    );
    assert.equal(s.dirty, false);
    s.undo();
    assert(s.dirty);
  } finally {
    await fs.rm(root, { recursive: true });
  }
});
test("a save acknowledgement cannot overwrite newer in-memory edits", async () => {
  const { project, workspace, root } = await sample();
  try {
    const s = new Session();
    s.load(project);
    const source = project.snapshot.sources.find(
      (f) => f.path === "items/common.yaml",
    )!;
    s.update(source.id, ["records", 0, "name"], "保存する値");
    const captured = s.state.snapshot;
    const promise = saveWorkspace(workspace, {
      token: project.token,
      files: s.files(),
      folders: captured.folders,
      config: captured.config,
    });
    s.update(source.id, ["records", 0, "name"], "保存中に変更");
    s.acceptSave(await promise, captured);
    assert.equal(
      s.state.snapshot.sources.find((f) => f.id === source.id)!.data.records[0]
        .name,
      "保存中に変更",
    );
    assert(s.dirty);
  } finally {
    await fs.rm(root, { recursive: true });
  }
});
test("comments added in text mode survive later GUI edits and CRLF stays CRLF", () => {
  const source = parseSource(
    "a.yaml",
    '# new comment\r\nkind: data\r\ntable: item\r\nrecords:\r\n  - {id: 1, name: "quoted"}\r\n',
  );
  const changed = {
    ...source,
    data: { ...source.data, records: [{ id: 1n, name: "changed" }] },
  };
  const output = serialize(changed);
  assert(output.includes("# new comment\r\n"));
  assert(output.includes('"changed"'));
  assert(!/(?<!\r)\n/.test(output));
});
test("session identities survive row and column edits but duplicates have independent identities", async () => {
  const { identity } = await import("../src/core/identity");
  const { project, root } = await sample();
  try {
    const s = new Session();
    s.load(project);
    const table = tables(s.state.snapshot).find((t) => t.name === "item")!;
    const data = s.state.snapshot.sources.find(
      (f) => f.path === "items/common.yaml",
    )!;
    const rowId = identity(data.data.records[0]);
    const fieldId = identity(table.fields[0]);
    s.update(data.id, ["records", 0, "price"], 777n);
    assert.equal(
      identity(
        s.state.snapshot.sources.find((f) => f.id === data.id)!.data.records[0],
      ),
      rowId,
    );
    s.update(table.source.id, ["fields", 0, "type"], "long");
    assert.equal(
      identity(
        tables(s.state.snapshot).find((t) => t.name === "item")!.fields[0],
      ),
      fieldId,
    );
    s.addRow(table.source.id, data.id, data.data.records[0]);
    assert.notEqual(
      identity(
        s.state.snapshot.sources
          .find((f) => f.id === data.id)!
          .data.records.at(-1),
      ),
      rowId,
    );
  } finally {
    await fs.rm(root, { recursive: true });
  }
});
