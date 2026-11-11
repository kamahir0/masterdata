import { test } from "node:test";
import assert from "node:assert/strict";
import {
  appendDisplayDefinition,
  displayDefinitionProblems,
  displayDefinitions,
  displayDocumentProblem,
  editDisplayDefinition,
  isDisplayDefinition,
  moveDisplayDefinition,
  newDisplayDefinition,
  removeDisplayDefinition,
  updateDisplayDefinitions,
  updateRawDisplay,
} from "../src/core/display-definitions";
import { identity, itemIdentities } from "../src/core/identity";
import { Session } from "../src/core/session";
import { checkDisplayInput, displayModel } from "../src/core/display-model";
import { defaultConfig, tables, type Source } from "../src/core/types";
import { parseSource, serialize } from "../src/core/yaml";

function setup(extra?: Record<string, unknown> | string) {
  const raw =
    "# canonical\r\nkind: schema\r\ntable: Item\r\nfields: [{name: id, type: ulong, key: 0}]\r\nrecords: [{id: 18446744073709551615}]\r\nfunctionColumns: keep-as-user-data\r\n";
  const source = parseSource("arbitrary/items/schema.yaml", raw, "schema");
  if (extra !== undefined)
    source.display =
      typeof extra === "string"
        ? parseSource(`${source.path}.view`, extra, `display:${source.id}`)
        : {
            id: `display:${source.id}`,
            path: `${source.path}.view`,
            data: extra,
          };
  const other: Source = {
    id: "other",
    path: "other.yaml",
    data: { kind: "data", table: "Item", records: [{ id: 2n }] },
  };
  const saved = [source, other].map((s) => ({
    id: s.id,
    path: s.path,
    text: s.formatText ?? serialize(s),
  }));
  if (source.display)
    saved.push({
      id: source.display.id,
      path: source.display.path,
      text:
        source.display.formatText ??
        source.display.raw ??
        serialize(source.display),
    });
  const session = new Session();
  session.load({
    root: "/test",
    token: "test",
    configText: "",
    saved,
    snapshot: {
      sources: [source, other],
      folders: [],
      config: defaultConfig("arbitrary"),
    },
  });
  const current = () =>
    session.getState().snapshot.sources.find((s) => s.id === "schema")!;
  return { source: current(), other, session, current, raw };
}
const functions = (s: Source) => displayDefinitions(s, "functionColumns").items;

test("表示のみの編集では未保存のソースプレビューも正本の元のテキストを使う", () => {
  const { session, current, raw } = setup();
  updateDisplayDefinitions(session, current(), "functionColumns", [
    { name: "ID", script: "return {text: String(row.id)};" },
  ], "表示編集");
  assert.equal(session.dirtySource(current()), true);
  assert.equal(session.sourceText(current()), raw);
  assert.equal(session.files()[0].text, raw);
  session.renameField(current().id, 0, "itemId");
  assert.match(session.sourceText(current()), /itemId/);
  session.undo();
  assert.equal(session.sourceText(current()), raw);
});

test("分割ファイルのrecordsが配列に戻るまで不完全な母数で評価しない", () => {
  const { session, other } = setup({
    functionColumns: [{ name: "件数", script: "return {text: String(table.rows.length)};" }],
  });
  const input = () => {
    const snapshot = session.getState().snapshot;
    return displayModel(tables(snapshot)[0], snapshot)!.input;
  };
  assert.equal(input().records.length, 2);
  session.update(other.id, ["records"], null, "未完成");
  assert.throws(() => checkDisplayInput(input()), /other.yaml の records/);
  session.undo();
  assert.doesNotThrow(() => checkDisplayInput(input()));
  assert.equal(input().records.length, 2);
});

test("正本の同名キーを読まず、通常編集でsidecarを作らない", () => {
  const { session, source, current } = setup();
  assert.equal(displayDefinitions(source, "functionColumns").kind, "absent");
  session.renameField(source.id, 0, "itemId");
  assert.equal(current().display, undefined);
  assert.equal(session.files().length, 2);
  const reloaded = parseSource(source.path, session.files()[0].text);
  assert.equal(reloaded.data.functionColumns, "keep-as-user-data");
  assert.equal(reloaded.data.records[0].itemId, 18446744073709551615n);
  for (const value of [null, "bad", 42, {}, true]) {
    const { session, source } = setup({ functionColumns: value });
    assert.equal(displayDefinitions(source, "functionColumns").kind, "invalid");
    assert.throws(
      () =>
        updateDisplayDefinitions(
          session,
          source,
          "functionColumns",
          [],
          "編集",
        ),
      /形式/,
    );
    assert.equal(session.getState().past.length, 0);
  }
});

test("shape判定は不正要素を変換せず、編集は未知キーと同一性を保持する", () => {
  const values = [
    null,
    "bad",
    [],
    {},
    { name: 3, script: "" },
    { name: "x", script: [] },
  ];
  values.forEach((v) => assert.equal(isDisplayDefinition(v), false));
  const extra = { nested: [18446744073709551615n, null] };
  const valid = { name: "", script: "return {", extension: extra };
  const { source } = setup({ functionColumns: [...values, valid] });
  const list = displayDefinitions(source, "functionColumns");
  assert.equal(list.kind, "array");
  const next = editDisplayDefinition(
    list.items,
    values.length,
    "計算",
    "const x = 1;\nreturn { text: String(x) };",
  );
  assert.deepEqual(next.slice(0, values.length), values);
  assert.equal((next.at(-1) as any).extension, extra);
  assert.equal(identity(next.at(-1)), identity(valid));
  assert.deepEqual(itemIdentities(next), itemIdentities(list.items));
  assert.throws(() => editDisplayDefinition(list.items, 0, "", ""), /形式/);
});

test("空名・重複・通常列との同名を診断する", () => {
  const { source } = setup({
    functionColumns: [
      { name: "id", script: "return {" },
      { name: "id", script: "" },
      { name: "  ", script: "" },
      null,
    ],
    formatRules: [
      { name: "id", script: "x" },
      { name: "同名", script: "x" },
      { name: "同名", script: "x" },
    ],
  });
  const problems = displayDefinitionProblems(source, "functionColumns");
  assert(problems[0].some((p) => p.includes("通常列")));
  assert(problems[0].some((p) => p.includes("同名")));
  assert(problems[1].some((p) => p.includes("コードが空")));
  assert(problems[2].some((p) => p.includes("名前が空")));
  assert(problems[3].some((p) => p.includes("表示YAML")));
  assert.deepEqual(displayDefinitionProblems(source, "formatRules")[0], []);
  assert(
    displayDefinitionProblems(source, "formatRules")[1].some((p) =>
      p.includes("同名"),
    ),
  );
});

test("表示編集・並替え・削除・Undo/Redoとschema単独Saveは正本bytesを維持する", () => {
  const { session, current, other, raw } = setup();
  const update = (
    key: "functionColumns" | "formatRules",
    next: readonly unknown[],
  ) =>
    updateDisplayDefinitions(session, current(), key, next, "表示定義を編集");
  update("functionColumns", [{ name: "率", script: "return {" }]);
  update("formatRules", [
    { name: "強調", script: "" },
    { name: "背景", script: "" },
  ]);
  const code = "const value = 42;\nreturn { text: String(value) };\n";
  update(
    "functionColumns",
    editDisplayDefinition(functions(current()), 0, "率（表示）", code),
  );
  const rules = displayDefinitions(current(), "formatRules").items;
  const order = itemIdentities(rules);
  update("formatRules", moveDisplayDefinition(rules, 1, 0));
  assert.deepEqual(
    itemIdentities(displayDefinitions(current(), "formatRules").items),
    [order[1], order[0]],
  );
  const beforeDelete = current();
  update(
    "formatRules",
    removeDisplayDefinition(
      displayDefinitions(current(), "formatRules").items,
      0,
    ),
  );
  session.undo();
  assert.equal(current(), beforeDelete);
  session.redo();
  assert.equal(displayDefinitions(current(), "formatRules").items.length, 1);
  session.undo();
  const files = session.files([current().id]);
  assert.equal(files.find((f) => f.id === current().id)!.text, raw);
  const view = files.find((f) => f.id === `display:${current().id}`)!;
  const reloaded = parseSource(view.path, view.text);
  assert.equal(reloaded.data.functionColumns[0].script, code);
  assert.deepEqual(
    reloaded.data.formatRules.map((item: any) => item.name),
    ["背景", "強調"],
  );
  assert.equal(session.getState().snapshot.sources[1], other);
  assert.equal(session.dirtySource(other), false);
  assert.equal(session.dirtySource(current()), true);
  for (let i = 0; i < 4; i++) session.undo();
  assert.equal(current().display, undefined);
  assert.equal(session.dirty, false);
});

test("未知キー・不正shape・未完成コードを保持し正本raw編集でsidecarを落とさない", () => {
  const bad = [null, "bad", { name: 1n, script: null, extra: [1n] }];
  const { session, current, source } = setup({
    functionColumns: bad,
    formatRules: { unsupported: true },
    extension: 2n,
  });
  updateDisplayDefinitions(
    session,
    source,
    "functionColumns",
    appendDisplayDefinition(bad, newDisplayDefinition(bad, "functionColumns")),
    "追加",
  );
  const display = current().display;
  session.raw(source.id, "kind: [broken");
  assert.equal(current().display, display);
  session.raw(source.id, "kind: schema\ntable: Item\nfields: []\n");
  assert.equal(current().display, display);
  const reloaded = parseSource(
    "x.view",
    session.files().find((f) => f.path.endsWith(".view"))!.text,
  );
  assert.deepEqual(reloaded.data.functionColumns.slice(0, bad.length), bad);
  assert.deepEqual(reloaded.data.formatRules, { unsupported: true });
  assert.equal(reloaded.data.extension, 2n);
});

test("表示文書の置換・並替え・Undo後に古い入力を誤適用しない", () => {
  for (const mutate of [
    (s: Session, source: Source) =>
      updateDisplayDefinitions(
        s,
        source,
        "functionColumns",
        moveDisplayDefinition(functions(source), 0, 1),
        "移動",
      ),
    (s: Session, source: Source) =>
      updateRawDisplay(s, source, "functionColumns: []\n"),
    (s: Session) => s.undo(),
  ]) {
    const { session, current, source } = setup();
    updateDisplayDefinitions(
      session,
      source,
      "functionColumns",
      [
        { name: "A", script: "" },
        { name: "B", script: "" },
      ],
      "追加",
    );
    const captured = current();
    const edited = editDisplayDefinition(
      functions(captured),
      0,
      "未適用",
      "pending",
    );
    mutate(session, captured);
    const before = current();
    assert.throws(
      () =>
        updateDisplayDefinitions(
          session,
          captured,
          "functionColumns",
          edited,
          "適用",
        ),
      /対象の定義/,
    );
    assert.equal(current(), before);
  }
});

test("正本の通常編集と表示入力が併存でき、保存中の新しい入力とUndoを維持する", () => {
  const { session, source, current, raw } = setup({
    functionColumns: [{ name: "A", script: "", extra: true }],
  });
  session.renameField(source.id, 0, "itemId");
  updateDisplayDefinitions(
    session,
    source,
    "functionColumns",
    editDisplayDefinition(functions(source), 0, "B", "return 1;"),
    "適用",
  );
  assert.equal(current().data.fields[0].name, "itemId");
  const captured = session.state.snapshot;
  const files = session.files([source.id]);
  updateRawDisplay(session, current(), "functionColumns: [unfinished\n");
  session.acceptSave({ files, configText: "" }, captured, [source.id]);
  assert(session.dirty);
  session.undo();
  assert.equal(session.dirtySource(current()), false);
  assert.equal((functions(current())[0] as any).extra, true);
  session.undo();
  assert(session.dirty);
  assert.notEqual(session.files()[0].text, raw);
});

test("不正YAML・不正root・未対応versionを保護しraw編集で修復できる", () => {
  for (const raw of [
    "[invalid",
    "null\n",
    "[1, 2]\n",
    "version: 999\nfunctionColumns: []\n",
  ]) {
    const { session, source, current } = setup(raw);
    assert(displayDocumentProblem(source));
    assert.throws(() =>
      updateDisplayDefinitions(session, source, "functionColumns", [], "編集"),
    );
    assert.equal(
      session.files().find((f) => f.path.endsWith(".view"))!.text,
      raw,
    );
    const fixed =
      '# preserve\r\nfunctionColumns: [{name: "", script: "return {"}]\r\nunknown: 42\r\n';
    updateRawDisplay(session, source, fixed);
    assert.equal(
      session.files().find((f) => f.path.endsWith(".view"))!.text,
      fixed,
    );
    assert.equal((functions(current())[0] as any).script, "return {");
    session.undo();
    assert.equal(session.dirty, false);
  }
});

test("schemaの移動・削除・破棄・保存後Undoをsidecarと同じ単位で扱う", () => {
  const { session, source, current } = setup({
    functionColumns: [{ name: "A", script: "" }],
  });
  const original = session.files();
  session.move(source.path, "new/schema.yml");
  assert.equal(
    session.files().find((f) => f.id === `display:${source.id}`)!.path,
    "new/schema.yml.view",
  );
  session.discardSource(source.id);
  assert.equal(session.dirty, false);
  session.move(source.path, "new/schema.yml");
  const captured = session.state.snapshot;
  const files = session.files([source.id]);
  session.acceptSave({ files, configText: "" }, captured, [source.id]);
  assert.equal(session.dirty, false);
  session.undo();
  assert.equal(current().path, source.path);
  assert.deepEqual(
    session.files().map((f) => [f.path, f.text]),
    original.map((f) => [f.path, f.text]),
  );
  session.remove([source.path]);
  assert(
    !session
      .files()
      .some((f) => [source.id, `display:${source.id}`].includes(f.id)),
  );
  session.discardSource(source.id);
  assert.equal(current().path, "new/schema.yml");
  assert.equal(session.dirty, false);
});

test("新規schemaで表示編集を取り消しても空のsidecarを保存しない", () => {
  const { session } = setup();
  const id = session.add("table");
  let source = session.state.snapshot.sources.find((s) => s.id === id)!;
  updateDisplayDefinitions(session, source, "formatRules", [], "編集");
  session.undo();
  assert(!session.files().some((f) => f.id === `display:${id}`));
  source = session.state.snapshot.sources.find((s) => s.id === id)!;
  session.discardSource(id);
  assert(!session.files().some((f) => f.id === id));
  const items = [
    { name: "関数列1", script: "" },
    null,
    { name: "関数列3", script: "" },
  ];
  assert.equal(newDisplayDefinition(items, "functionColumns").name, "関数列2");
  for (const index of [-1, 3, 0.5, NaN]) {
    assert.throws(() => removeDisplayDefinition(items, index));
    assert.throws(() => moveDisplayDefinition(items, 0, index));
  }
});
