export type DirtyPathDecision = "cancel" | "discard" | "save";

export async function resolveDirtyPathMutation({
  decision,
  save,
  discard,
  mutate,
}: {
  decision: DirtyPathDecision;
  save: () => Promise<boolean>;
  discard: () => void | Promise<void>;
  mutate: () => void | Promise<void>;
}): Promise<boolean> {
  if (decision === "cancel") return false;
  if (decision === "save" && !(await save())) return false;
  if (decision === "discard") await discard();
  await mutate();
  return true;
}

export function migrationRefreshPlan<T>(
  editors: Record<string, T>,
  affectedPaths: readonly string[],
  isDirty: (editor: T) => boolean,
): { reloadPaths: string[]; retainedEditors: Record<string, T> } {
  const reloadPaths = affectedPaths.filter((path) => editors[path] !== undefined && !isDirty(editors[path]));
  const reloadSet = new Set(reloadPaths);
  const retainedEditors = Object.fromEntries(
    Object.entries(editors).filter(([path]) => !reloadSet.has(path)),
  ) as Record<string, T>;
  return { reloadPaths, retainedEditors };
}

export function migrationBlockedFiles<T extends { path: string }>(
  files: readonly T[],
  dirtyPaths: readonly string[],
): T[] {
  const dirty = new Set(dirtyPaths);
  return files.filter((file) => dirty.has(file.path));
}

export function migrationApplyDisabled({
  canWrite,
  stale,
  blocked,
  destructive,
  confirmed,
  succeeded,
}: {
  canWrite: boolean;
  stale: boolean;
  blocked: boolean;
  destructive: boolean;
  confirmed: boolean;
  succeeded: boolean;
}): boolean {
  return !canWrite || stale || blocked || (destructive && !confirmed) || succeeded;
}

export function migrationDestructiveAuthorization(destructive: boolean, confirmed: boolean): boolean {
  return destructive && confirmed;
}
