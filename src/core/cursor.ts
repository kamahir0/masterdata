import { identity, itemIdentities } from "./identity";
import type { Snapshot } from "./types";

// Array offsets are coordinates, not targets. Capture every ancestor slot so
// reordering a row or a repeated primitive never retargets an open editor.
export interface EditTarget {
  source: string;
  path: (string | { item: string; object: boolean })[];
}
export function captureTarget(
  snapshot: Snapshot,
  source: string,
  path: (string | number)[],
): EditTarget {
  let value = snapshot.sources.find((s) => s.id === source)?.data;
  return {
    source,
    path: path.map((part) => {
      const object = value?.[part] !== null && typeof value?.[part] === "object";
      let anchor: EditTarget["path"][number] = String(part);
      if (typeof part === "number") {
        const item = !Array.isArray(value) ? ""
          : object ? identity(value[part]) : itemIdentities(value)[part];
        anchor = { item, object };
      }
      value = value?.[part];
      return anchor;
    }),
  };
}
export function resolveTarget(
  snapshot: Snapshot,
  target: EditTarget,
): (string | number)[] | null {
  const source = snapshot.sources.find((s) => s.id === target.source);
  if (!source) return null;
  let value = source.data;
  const path: (string | number)[] = [];
  for (const anchor of target.path) {
    let part: string | number;
    if (typeof anchor === "string") part = anchor;
    else if (!Array.isArray(value)) return null;
    else part = anchor.object
      ? value.findIndex((item) => identity(item) === anchor.item)
      : itemIdentities(value).indexOf(anchor.item);
    if (part === -1) return null;
    path.push(part);
    value = value?.[part];
  }
  return path;
}
