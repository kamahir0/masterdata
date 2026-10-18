import {
  parseDocument,
  stringify,
  isMap,
  isSeq,
  isScalar,
  visit,
  type Node,
} from "yaml";
import { type Source, type SavedFile, uid, isObject } from "./types";
export function parseSource(path: string, input: string, id = uid()): Source {
  const doc = parseDocument(input, {
    intAsBigInt: true,
    keepSourceTokens: true,
    uniqueKeys: true,
  });
  try {
    if (doc.errors.length)
      throw new Error(doc.errors.map((e) => e.message).join("\n"));
    return {
      id,
      path,
      data: doc.toJS({ maxAliasCount: 0 }),
      formatText: input,
    };
  } catch (error) {
    return { id, path, data: null, raw: input, syntaxError: String(error) };
  }
}
export function serialize(source: Source, saved?: SavedFile): string {
  if (source.raw !== undefined) return source.raw;
  const original = source.formatText ?? saved?.text;
  if (original === undefined) return stringify(source.data, { lineWidth: 0 });
  const doc = parseDocument(original, {
    intAsBigInt: true,
    keepSourceTokens: true,
  });
  if (doc.errors.length) return stringify(source.data, { lineWidth: 0 });
  // YAML is patched at the persistence boundary; ordinary input never reparses or serializes a file.
  // Keep AST nodes for comments and quoting, not for historical flow/block choices.
  // Every edited document uses the same block layout, including newly appended entries.
  function sync(node: any, next: any): any {
    if (isMap(node) && isObject(next)) {
      for (const pair of [...node.items])
        if (!(String((pair.key as any)?.value) in next))
          node.delete(String((pair.key as any)?.value));
      for (const [key, value] of Object.entries(next)) {
        const previous = node.get(key, true);
        node.set(key, previous ? sync(previous, value) : doc.createNode(value));
      }
      return node;
    }
    if (isSeq(node) && Array.isArray(next)) {
      node.items = next.map((value, i) =>
        node.items[i] ? sync(node.items[i], value) : doc.createNode(value),
      );
      return node;
    }
    if (isScalar(node) && (next === null || typeof next !== "object")) {
      if (node.value !== next) {
        node.value = next;
        node.source = undefined;
      }
      return node;
    }
    return doc.createNode(next);
  }
  doc.contents = sync(doc.contents, source.data) as any;
  visit(doc, (_, node) => {
    if (isMap(node) || isSeq(node)) node.flow = node.items.length === 0;
  });
  const output = doc.toString({ lineWidth: 0 });
  return original.includes("\r\n") ? output.replace(/\r?\n/g, "\r\n") : output;
}
