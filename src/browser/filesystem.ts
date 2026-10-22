// The adapter receives only a user-selected root capability. No absolute paths,
// network calls, recursive deletes, or ordinary-edit filesystem access live here.
export interface Directory extends FileSystemDirectoryHandle {
  queryPermission(options: { mode: "readwrite" }): Promise<PermissionState>;
  requestPermission(options: { mode: "readwrite" }): Promise<PermissionState>;
  getDirectoryHandle(
    name: string,
    options?: { create?: boolean },
  ): Promise<Directory>;
}
export function parts(path: string) {
  const result = path.split("/");
  if (
    result.some((p) => !p || p === "." || p === ".." || /[\\\x00-\x1f]/.test(p))
  )
    throw new Error(`安全でないパスです: ${path}`);
  return result;
}
export const missing = (error: unknown) =>
  error instanceof Error && error.name === "NotFoundError";
export async function directory(
  root: Directory,
  path: string,
  create = false,
): Promise<Directory> {
  for (const part of parts(path))
    root = await root.getDirectoryHandle(part, { create });
  return root;
}
export async function parent(root: Directory, path: string, create = false) {
  const names = parts(path);
  const name = names.pop()!;
  return {
    dir: names.length ? await directory(root, names.join("/"), create) : root,
    name,
  };
}
export async function read(
  root: Directory,
  path: string,
): Promise<string | null> {
  try {
    const { dir, name } = await parent(root, path);
    const file = await (await dir.getFileHandle(name)).getFile();
    // File.text silently replaces invalid UTF-8. Saving that would lose bytes.
    return new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(
      await file.arrayBuffer(),
    );
  } catch (error) {
    if (missing(error)) return null;
    throw new Error(`${path}: ${String(error)}`);
  }
}
export async function write(
  root: Directory,
  path: string,
  text: string | null,
) {
  try {
    const { dir, name } = await parent(root, path);
    if (text === null) {
      await dir.removeEntry(name);
      return;
    }
    const handle = await dir.getFileHandle(name, { create: true });
    const stream = await handle.createWritable();
    try {
      await stream.write(text);
      await stream.close();
    } catch (error) {
      await stream.abort().catch(() => {});
      throw error;
    }
  } catch (error) {
    throw new Error(`${path}: ${String(error)}`);
  }
}
export async function exists(root: Directory, path: string) {
  try {
    await directory(root, path);
    return true;
  } catch (error) {
    if (missing(error)) return false;
    throw error;
  }
}
export async function entries(root: Directory, path: string) {
  const dir = await directory(root, path);
  const result: (FileSystemFileHandle | FileSystemDirectoryHandle)[] = [];
  for await (const entry of dir.values()) result.push(entry);
  return result;
}
export async function permission(root: Directory) {
  const options = { mode: "readwrite" as const };
  if (
    (await root.queryPermission(options)) !== "granted" &&
    (await root.requestPermission(options)) !== "granted"
  )
    throw new Error(
      "フォルダへの読み書きが許可されていません。入力は保持しています。「保存方法 → フォルダのアクセスを再許可」または復旧画面から同じフォルダを選択してください。",
    );
}
