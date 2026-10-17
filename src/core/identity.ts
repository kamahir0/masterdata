// Identity belongs to the edit session, not persisted YAML or the user's primary key.
// Editing or duplicating a key must not remount the active input or confuse row selection.
const identities = new WeakMap<object, string>();
let sequence = 0;
export function identity(value: unknown): string {
  if (!value || typeof value !== "object")
    return `${typeof value}:${String(value)}`;
  let id = identities.get(value);
  if (!id) {
    id = `value-${++sequence}`;
    identities.set(value, id);
  }
  return id;
}
export function inheritIdentity(previous: unknown, next: unknown) {
  if (
    previous &&
    next &&
    typeof previous === "object" &&
    typeof next === "object" &&
    !identities.has(next)
  )
    identities.set(next, identity(previous));
}
