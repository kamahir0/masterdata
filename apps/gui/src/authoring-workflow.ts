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
