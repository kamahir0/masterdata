// JSON IPC preserves editable values (including invalid numeric drafts) without
// reserving an object shape in the user's data.
export function encode(value: unknown): string {
  const integers: [(string | number)[], string, ("number" | "undefined")?][] =
    [];
  function walk(v: any, path: (string | number)[]): any {
    if (typeof v === "bigint") {
      integers.push([path, String(v)]);
      return null;
    }
    if (v === undefined) {
      integers.push([path, "", "undefined"]);
      return null;
    }
    if (typeof v === "number" && (!Number.isFinite(v) || Object.is(v, -0))) {
      integers.push([path, Object.is(v, -0) ? "-0" : String(v), "number"]);
      return null;
    }
    if (Array.isArray(v)) return v.map((x, i) => walk(x, [...path, i]));
    if (v && typeof v === "object")
      return Object.fromEntries(
        Object.entries(v).map(([k, x]) => [k, walk(x, [...path, k])]),
      );
    return v;
  }
  const data = walk(value, []);
  return JSON.stringify([data ?? null, integers]);
}
export function decode<T = any>(text: string): T {
  let [data, integers] = JSON.parse(text);
  for (const [path, value, kind] of integers) {
    const restored =
      kind === "undefined"
        ? undefined
        : kind === "number"
          ? Number(value)
          : BigInt(value);
    if (!path.length) data = restored;
    else {
      let parent = data;
      for (const key of path.slice(0, -1)) parent = parent[key];
      parent[path.at(-1)] = restored;
    }
  }
  return data;
}
