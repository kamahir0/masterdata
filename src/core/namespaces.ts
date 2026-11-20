export interface Namespaces {
  root: string;
  useFolders: boolean;
  folders: Record<string, boolean>;
}
export const defaultNamespaces: Namespaces = {
  root: "Masterdata.Generated",
  useFolders: false,
  folders: {},
};

const keywords = new Set(
  "abstract as base bool break byte case catch char checked class const continue decimal default delegate do double else enum event explicit extern false finally fixed float for foreach goto if implicit in int interface internal is lock long namespace new null object operator out override params private protected public readonly ref return sbyte sealed short sizeof stackalloc static string struct switch this throw true try typeof uint ulong unchecked unsafe ushort using virtual void volatile while __arglist __makeref __reftype __refvalue".split(
    " ",
  ),
);
export function validSegment(name: string) {
  return (
    /^[_\p{L}\p{Nl}][_\p{L}\p{Nl}\p{Nd}\p{Mn}\p{Mc}\p{Pc}]*$/u.test(name) &&
    !keywords.has(name)
  );
}
export function folderChain(path: string): string[] {
  const chain: string[] = [];
  for (const part of path.split("/").filter(Boolean))
    chain.push(chain.length ? chain[chain.length - 1] + "/" + part : part);
  return chain;
}
export function included(ns: Namespaces, path: string) {
  return Object.hasOwn(ns.folders, path) ? ns.folders[path] : ns.useFolders;
}
export function effectiveNamespace(ns: Namespaces, folder: string) {
  const parts = [ns.root];
  for (const path of folderChain(folder)) {
    if (!included(ns, path)) break;
    parts.push(path.split("/").pop()!);
  }
  return parts.join(".");
}
export function sourceFolder(file: string) {
  return file.split("/").slice(0, -1).join("/");
}
export function namespaceError(ns: Namespaces, folder: string) {
  if (!ns.root.split(".").every(validSegment))
    return "ルート名前空間に有効なC#識別子を指定してください。";
  for (const path of folderChain(folder)) {
    if (!included(ns, path)) break;
    if (!validSegment(path.split("/").pop()!))
      return `「${path}」をC#名前空間に使えません。名前を変更するか、このフォルダをOFFにしてください。`;
  }
}
