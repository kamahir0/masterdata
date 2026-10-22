import {
  type Snapshot,
  type Problem,
  type Field,
  type Data,
  type Source,
  tables,
  typeSources,
  typeSource,
  typeKind,
  fieldsOf,
  primitives,
  isObject,
  text,
  list,
} from "./types";
import { primitiveOf, inProfile } from "./schema";
const ranges: Record<string, [bigint, bigint]> = {
  int: [-2147483648n, 2147483647n],
  uint: [0n, 4294967295n],
  long: [-9223372036854775808n, 9223372036854775807n],
  ulong: [0n, 18446744073709551615n],
};
export const fingerprint = (value: Data): string =>
  JSON.stringify(value, (_, v) =>
    typeof v === "bigint" ? { $integer: String(v) } : v,
  );
export function validate(input: Snapshot, profile = ""): Problem[] {
  const snapshot = profile ? inProfile(input, profile) : input;
  const problems: Problem[] = [];
  const add = (
    source: Source,
    path: (string | number)[],
    message: string,
    code = "VALUE",
  ) => {
    if (problems.length < 2000)
      problems.push({ source: source.id, path, message, code });
  };
  const validIdentifier = (name: unknown) =>
    typeof name === "string" && /^[A-Za-z_][A-Za-z0-9_]*$/.test(name);
  const allTables = tables(snapshot);
  const allTypes = typeSources(snapshot);
  const names = new Map<string, Source>();
  for (const source of snapshot.sources) {
    if (source.syntaxError) {
      add(source, [], source.syntaxError, "YAML");
      continue;
    }
    if (!isObject(source.data)) {
      add(source, [], "定義は項目と値を持つ形式にしてください。", "SHAPE");
      continue;
    }
    if (!["schema", "data", "type"].includes(source.data.kind))
      add(
        source,
        ["kind"],
        "種類はschema / data / typeを指定してください。",
        "KIND",
      );
    if (
      source.data.kind === "data" &&
      !allTables.some((t) => t.name === source.data.table)
    )
      add(
        source,
        ["table"],
        `表「${source.data.table}」が見つかりません。`,
        "TABLE",
      );
    const name =
      source.data.kind === "schema"
        ? source.data.table
        : source.data.kind === "type"
          ? source.data.name
          : null;
    if (name !== null) {
      if (!name || typeof name !== "string")
        add(source, [], "名前を入力してください。", "NAME");
      else if (names.has(`${source.data.kind}:${name}`))
        add(source, [], `名前「${name}」が重複しています。`, "NAME");
      names.set(`${source.data.kind}:${name}`, source);
      const generatedName =
        source.data.kind === "schema"
          ? source.data.csharpName ||
            String(name).replace(
              /(^|[-_ ])([a-zA-Z])/g,
              (_: string, __: string, c: string) => c.toUpperCase(),
            )
          : name;
      if (!validIdentifier(generatedName))
        add(
          source,
          [],
          "生成する型名は英字・数字・_で指定してください。",
          "NAME",
        );
    }
  }
  function fieldValue(
    source: Source,
    path: (string | number)[],
    field: Field,
    value: Data,
    seen: Set<string>,
  ) {
    if (value === null) {
      if (!field.nullable)
        add(
          source,
          path,
          "値が必要です。「値を作成」から初期値を用意できます。",
        );
      return;
    }
    if (value === undefined) {
      add(source, path, "値がありません。入力するか初期値を作成してください。");
      return;
    }
    if (field.array) {
      if (!Array.isArray(value)) {
        add(source, path, "配列の値が必要です。配列を開いて修復してください。");
        return;
      }
      value.forEach((v, i) =>
        fieldValue(
          source,
          [...path, i],
          { ...field, array: false, nullable: false },
          v,
          seen,
        ),
      );
      return;
    }
    const type = typeSource(snapshot, field.type);
    const kind = typeKind(type);
    const primitive = primitiveOf(snapshot, field);
    if (primitive === "string") {
      if (typeof value !== "string")
        add(source, path, "文字列を入力してください。");
      return;
    }
    if (primitive === "bool") {
      if (typeof value !== "boolean")
        add(source, path, "真偽値を選択してください。");
      return;
    }
    if (primitive in ranges) {
      try {
        if (
          (typeof value !== "bigint" && typeof value !== "number") ||
          (typeof value === "number" && !Number.isSafeInteger(value))
        )
          throw 0;
        const n = BigInt(value);
        const [min, max] = ranges[primitive];
        if (n < min || n > max) throw 0;
      } catch {
        add(
          source,
          path,
          `${primitive}の整数（${ranges[primitive].join(" ～ ")}）を入力してください。`,
        );
      }
      return;
    }
    if (primitive === "float" || primitive === "double") {
      if (
        (typeof value !== "number" && typeof value !== "bigint") ||
        !Number.isFinite(Number(value)) ||
        (primitive === "float" &&
          Math.abs(Number(value)) > 3.4028234663852886e38)
      )
        add(source, path, "範囲内の有限な数値を入力してください。");
      return;
    }
    if (kind === "enum") {
      if (!list(type!.data.enum.members).some((m) => isObject(m) && m.name === value))
        add(source, path, "定義されたEnumの候補を選択してください。");
      return;
    }
    if (kind === "flags") {
      const members = list(type!.data.flags.members).filter(isObject).map((m) => m.name);
      if (
        !Array.isArray(value) ||
        !value.length ||
        value.some((v) => !members.includes(v)) ||
        new Set(value).size !== value.length ||
        (value.length > 1 && value.includes("None"))
      )
        add(
          source,
          path,
          "Flagsの候補を選択してください。「なし」は単独で選択します。",
        );
      return;
    }
    if (kind === "custom") {
      if (!isObject(value)) {
        add(source, path, "構造体の値が必要です。開いて修復してください。");
        return;
      }
      if (seen.has(field.type)) {
        add(source, path, "型の循環があります。");
        return;
      }
      const next = new Set(seen).add(field.type);
      fieldsOf(type!).forEach((f) =>
        fieldValue(source, [...path, f.name], f, value[f.name], next),
      );
      return;
    }
    add(
      source,
      path,
      `型「${field.type || "未選択"}」が見つかりません。`,
      "TYPE",
    );
  }
  for (const source of [...allTables.map((t) => t.source), ...allTypes]) {
    const fields = fieldsOf(source);
    const names = new Set<string>();
    const keys = new Set<number>();
    const container = source.data.custom ? ["custom", "fields"] : ["fields"];
    fields.forEach((field, i) => {
      if (!field.name || names.has(field.name))
        add(
          source,
          [...container, i, "name"],
          "項目名が空か重複しています。",
          "FIELD",
        );
      if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(field.name ?? ""))
        add(
          source,
          [...container, i, "name"],
          "項目名は英字・数字・_で指定してください。",
          "FIELD",
        );
      names.add(field.name);
      const key = Number(field.key ?? i);
      if (!Number.isSafeInteger(key) || key < 0 || keys.has(key))
        add(
          source,
          [...container, i, "key"],
          "シリアライズ番号は重複しない0以上の整数にしてください。",
          "KEY",
        );
      keys.add(key);
      if (!primitives.includes(field.type) && !typeSource(snapshot, field.type))
        add(
          source,
          [...container, i, "type"],
          `型「${field.type}」が見つかりません。`,
          "TYPE",
        );
    });
    const kind = typeKind(source);
    if (
      source.data.kind === "type" &&
      ["enum", "flags", "custom", "valueObject"].filter((k) => source.data[k])
        .length !== 1
    )
      add(
        source,
        [],
        "型の種類を一つ定義してください。ソースから修正できます。",
        "TYPE",
      );
    if (
      (source.data.kind === "schema" && !Array.isArray(source.data.fields)) ||
      (kind === "custom" && !Array.isArray(source.data.custom.fields))
    )
      add(source, [], "フィールドの一覧は配列にしてください。", "FIELD");
    for (const k of ["secondaryKeys", "references"])
      if (source.data[k] !== undefined && !Array.isArray(source.data[k]))
        add(
          source,
          [k],
          "キー・参照の一覧は配列にしてください。ソースから修正できます。",
          "SHAPE",
        );
    if (kind === "enum" || kind === "flags") {
      const def = source.data[kind];
      const members = Array.isArray(def.members) ? def.members : [];
      const memberNames = new Set();
      const memberValues = new Set();
      if (!members.length)
        add(source, [kind, "members"], "候補を追加してください。", "ENUM");
      for (let i = 0; i < members.length; i++) {
        const m = members[i];
        if (!isObject(m)) {
          add(source, [kind, "members", i], "候補を名前と数値を持つ形式にしてください。", "SHAPE");
          continue;
        }
        if (
          !/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(m.name ?? "") ||
          memberNames.has(m.name)
        )
          add(
            source,
            [kind, "members", i],
            "候補名が空・不正・重複しています。",
            "ENUM",
          );
        memberNames.add(m.name);
        if (memberValues.has(String(m.value)))
          add(
            source,
            [kind, "members", i],
            "候補の数値が重複しています。",
            "ENUM",
          );
        memberValues.add(String(m.value));
        const range = ranges[def.underlying ?? "int"];
        try {
          const n = BigInt(m.value);
          if (!range || n < range[0] || n > range[1]) throw 0;
          const width =
            def.underlying === "long" || def.underlying === "ulong" ? 64n : 32n;
          const bits = n < 0n ? n + (1n << width) : n;
          if (
            kind === "flags" &&
            (m.name === "None"
              ? n !== 0n
              : bits <= 0n || (bits & (bits - 1n)) !== 0n)
          )
            add(
              source,
              [kind, "members", i],
              "FlagsはNone=0、その他は重複しない単一bitにしてください。",
              "FLAGS",
            );
        } catch {
          add(
            source,
            [kind, "members", i],
            "整数の値と基になる型を確認してください。",
            "ENUM",
          );
        }
      }
      if (kind === "flags" && !memberNames.has("None"))
        add(source, [kind, "members"], "None=0を追加してください。", "FLAGS");
    }
    if (
      kind === "valueObject" &&
      !["int", "uint", "long", "ulong", "string"].includes(
        source.data.valueObject.underlying,
      )
    )
      add(
        source,
        ["valueObject"],
        "基になる型は整数または文字列を選んでください。",
        "TYPE",
      );
  }
  // Type cycles are diagnosed independently of current records, so empty tables cannot hide invalid definitions.
  function visit(name: string, stack: Set<string>, origin: Source) {
    if (stack.has(name)) {
      add(origin, [], `型「${name}」の循環があります。`, "CYCLE");
      return;
    }
    const source = typeSource(snapshot, name);
    if (!source?.data.custom) return;
    const next = new Set(stack).add(name);
    fieldsOf(source).forEach((f) => visit(f.type, next, origin));
  }
  allTypes.forEach((s) => visit(String(s.data.name), new Set(), s));
  for (const table of allTables) {
    const rows: { source: Source; row: Data; index: number }[] = [];
    for (const source of table.sources) {
      if (
        source.data.records !== undefined &&
        !Array.isArray(source.data.records)
      ) {
        add(
          source,
          ["records"],
          "レコード一覧は配列にしてください。",
          "RECORDS",
        );
        continue;
      }
      (source.data.records ?? []).forEach((row: Data, index: number) => {
        rows.push({ source, row, index });
        if (!isObject(row)) {
          add(
            source,
            ["records", index],
            "レコードを項目と値の形式にしてください。",
          );
          return;
        }
        table.fields.forEach((f) =>
          fieldValue(
            source,
            ["records", index, f.name],
            f,
            row[f.name],
            new Set(),
          ),
        );
      });
    }
    const allKeys: { key: any; path: (string | number)[] }[] = [];
    if (table.source.data.primaryKey !== undefined)
      allKeys.push({ key: table.source.data.primaryKey, path: ["primaryKey"] });
    list(table.source.data.secondaryKeys).forEach((key, i) =>
      allKeys.push({ key, path: ["secondaryKeys", i] }));
    if (!table.source.data.primaryKey?.fields?.length)
      add(
        table.source,
        ["primaryKey"],
        "主キーの項目を選択してください。",
        "KEY",
      );
    allKeys.forEach(({ key, path }) => {
      if (!isObject(key)) {
        add(table.source, path, "キーを項目一覧を持つ形式にしてください。", "SHAPE");
        return;
      }
      if (!Array.isArray(key.fields) || !key.fields.length) {
        add(
          table.source,
          path,
          "キーの項目を選択してください。",
          "KEY",
        );
        return;
      }
      for (const name of key.fields) {
        const field = table.fields.find((f) => f.name === name);
        const base = field && primitiveOf(snapshot, field);
        if (
          !field ||
          field.array ||
          field.nullable ||
          (!["int", "uint", "long", "ulong", "string"].includes(base!) &&
            typeKind(typeSource(snapshot, field.type)) !== "enum")
        )
          add(
            table.source,
            path,
            `キー「${name}」は必須の整数・文字列・ID・Enum項目にしてください。`,
            "KEY",
          );
      }
      if (key.nonUnique && path[0] === "secondaryKeys") return;
      const used = new Set<string>();
      for (const { source, row, index } of rows) {
        const signature = fingerprint(key.fields.map((n: string) => row?.[n]));
        if (used.has(signature))
          add(
            source,
            ["records", index],
            `キー（${key.fields.join(", ")}）が重複しています。`,
            "DUPLICATE",
          );
        used.add(signature);
      }
    });
    for (const [referenceIndex, reference] of list(table.source.data.references).entries()) {
      const referencePath = ["references", referenceIndex];
      if (!isObject(reference)) {
        add(table.source, referencePath, "参照を名前・項目・参照先を持つ形式にしてください。", "SHAPE");
        continue;
      }
      if (
        !validIdentifier(reference.name) ||
        (reference.csharpName && !validIdentifier(reference.csharpName))
      )
        add(
          table.source,
          referencePath,
          "参照の名前は英字・数字・_で指定してください。",
          "NAME",
        );
      const target = allTables.find((t) => t.name === reference.target?.table);
      const targetFields = reference.target?.fields;
      const ownFields = reference.fields;
      if (
        !target ||
        !Array.isArray(ownFields) ||
        !Array.isArray(targetFields) ||
        !ownFields.length ||
        ownFields.length !== targetFields.length ||
        ownFields.some((n) => !table.fields.some((f) => f.name === n)) ||
        targetFields.some((n) => !target.fields.some((f) => f.name === n))
      ) {
        add(
          table.source,
          referencePath,
          `参照「${reference.name}」の表と項目を設定してください。`,
          "REFERENCE",
        );
        continue;
      }
      if (
        ownFields.some(
          (n: string, i: number) =>
            table.fields.find((f) => f.name === n)?.type !==
            target.fields.find((f) => f.name === targetFields[i])?.type,
        )
      )
        add(
          table.source,
          referencePath,
          "参照元と参照先の項目の型を揃えてください。",
          "REFERENCE",
        );
      const keys = [
        target.source.data.primaryKey,
        ...list(target.source.data.secondaryKeys),
      ].filter(isObject);
      if (
        !keys.some((k) => fingerprint(k.fields) === fingerprint(targetFields))
      )
        add(
          table.source,
          referencePath,
          `参照先に対応するキーがありません。`,
          "REFERENCE",
        );
      const available = new Set(
        target.sources
          .flatMap((s) => s.data?.records ?? [])
          .map((r: any) =>
            fingerprint(targetFields.map((n: string) => r?.[n])),
          ),
      );
      for (const { source, row, index } of rows) {
        const tuple = ownFields.map((n: string) => row?.[n]);
        if (tuple.every((v: any) => v === null)) continue;
        if (!available.has(fingerprint(tuple)))
          add(
            source,
            ["records", index, ownFields[0]],
            `参照先「${target.name}」に対応するデータがありません。`,
            "REFERENCE",
          );
      }
    }
  }
  return problems;
}
