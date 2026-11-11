// This function is loaded as source into a fresh QuickJS context per definition.
// It receives copied data and strings. It has no callbacks into the host.
(wire, columnsJSON, source, kind) => {
  "use strict";
  const [rows, special] = JSON.parse(wire);
  for (const [path, value, tag] of special) {
    let parent = rows;
    for (const key of path.slice(0, -1)) parent = parent[key];
    parent[path.at(-1)] =
      tag === "undefined"
        ? undefined
        : tag === "number"
          ? Number(value)
          : BigInt(value);
  }
  const columns = JSON.parse(columnsJSON);
  const bounds = {
    int: [-2147483648n, 2147483647n],
    uint: [0n, 4294967295n],
    long: [-(2n ** 63n), 2n ** 63n - 1n],
    ulong: [0n, 2n ** 64n - 1n],
  };
  const fieldTypes = new Map(
    columns.filter((c) => c.kind === "data").map((c) => [c.name, c.type]),
  );
  for (const row of rows) {
    if (row === null || typeof row !== "object" || Array.isArray(row)) continue;
    for (const [field, type] of fieldTypes) {
      const value = row[field];
      if (Object.hasOwn(bounds, type) && Number.isSafeInteger(value))
        row[field] = BigInt(value);
      else if (
        (type === "float" || type === "double") &&
        typeof value === "bigint"
      )
        row[field] = Number(value);
    }
  }
  function freeze(object) {
    if (object && typeof object === "object") {
      for (const item of Object.values(object)) freeze(item);
      Object.freeze(object);
    }
    return object;
  }
  freeze(rows);
  freeze(columns);
  const empty = Object.freeze([]);
  const groups = new Map();
  const totals = new Map();
  function group(field, value) {
    if (!fieldTypes.has(field) || (value != null && typeof value === "object"))
      return empty;
    if (!groups.has(field)) {
      const index = new Map();
      for (const row of rows) {
        const key = row?.[field];
        if (!index.has(key)) index.set(key, []);
        index.get(key).push(row);
      }
      for (const list of index.values()) Object.freeze(list);
      groups.set(field, index);
    }
    return groups.get(field).get(value) ?? empty;
  }
  function sumBy(field, value, measure) {
    const type = fieldTypes.get(measure);
    const integer = Object.hasOwn(bounds, type);
    if (!integer && type !== "float" && type !== "double") return null;
    const list = group(field, value);
    if (!list.length) return null;
    if (!totals.has(measure)) totals.set(measure, new Map());
    const cache = totals.get(measure);
    if (cache.has(list)) return cache.get(list);
    let sum = integer ? 0n : 0;
    for (const row of list) {
      const n = row?.[measure];
      if (
        integer
          ? typeof n !== "bigint" || n < bounds[type][0] || n > bounds[type][1]
          : !Number.isFinite(n)
      ) {
        sum = null;
        break;
      }
      sum += n;
    }
    if (typeof sum === "number" && !Number.isFinite(sum)) sum = null;
    cache.set(list, sum);
    return sum;
  }
  const table = Object.freeze({ rows, group, sumBy });
  const stringify = JSON.stringify;
  const keys = Object.keys;
  const has = Object.hasOwn;
  const array = Array.isArray;
  const proto = Object.getPrototypeOf;
  const plain = (value) =>
    value !== null &&
    typeof value === "object" &&
    !array(value) &&
    (proto(value) === Object.prototype || proto(value) === null);
  let evaluate;
  try {
    // Function belongs to the guest interpreter. The browser never compiles this source.
    evaluate = new Function(
      "row",
      "table",
      "column",
      '"use strict";\n' + source,
    );
  } catch (e) {
    return "構文エラー: " + String(e.message).slice(0, 240);
  }
  return (rowIndex, columnIndex) => {
    try {
      const value = evaluate(rows[rowIndex], table, columns[columnIndex]);
      const out = Object.create(null);
      let style;
      if (kind === "function") {
        if (
          !plain(value) ||
          !has(value, "text") ||
          typeof value.text !== "string"
        )
          throw "戻り値には文字列のtextが必要です";
        const text = value.text;
        if (text.length > 4096) throw "表示テキストは4096文字以内です";
        if (keys(value).some((key) => key !== "text" && key !== "style"))
          throw "戻り値に未対応の項目があります";
        out.text = text;
        style = value.style;
      } else {
        if (value === null) return "{}";
        style = value;
      }
      if (style !== undefined || kind === "rule") {
        if (!plain(style) || typeof style.then === "function")
          throw "書式オブジェクトを返してください";
        const clean = Object.create(null);
        for (const key of keys(style)) {
          const attr = style[key];
          if (key === "bold" || key === "italic") {
            if (typeof attr !== "boolean")
              throw key + "にはbooleanを指定してください";
          } else if (key === "color" || key === "backgroundColor") {
            if (typeof attr !== "string" || !/^#[0-9a-f]{6}$/i.test(attr))
              throw key + "には#RRGGBBを指定してください";
          } else throw "未対応の書式プロパティ: " + key.slice(0, 64);
          clean[key] = attr;
        }
        out.style = clean;
      }
      return stringify(out);
    } catch (error) {
      const out = Object.create(null);
      out.error = String(error).slice(0, 240);
      if (
        error?.name === "InternalError" &&
        /out of memory|stack overflow/i.test(out.error)
      )
        out.resourceLimit = true;
      return stringify(out);
    }
  };
};
