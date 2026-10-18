// Identity belongs to the edit session, not persisted YAML or the user's primary key.
// Editing or duplicating a key must not remount the active input or confuse row selection.
const identities = new WeakMap<object, string>();
const slots = new WeakMap<readonly unknown[], string[]>();
let sequence = 0;
export function itemIdentities(items: readonly unknown[]): readonly string[] {
  let keys = slots.get(items);
  if (!keys) {
    keys = items.map(() => `item-${++sequence}`);
    slots.set(items, keys);
  }
  return keys;
}
// Structural changes carry slots explicitly. Equality cannot identify repeated
// primitive values; neither the value nor its current index is an editor ID.
export function spliceItems<T>(
  items: readonly T[],
  at: number,
  count: number,
  ...insert: T[]
): T[] {
  const next = [...items],
    keys = [...itemIdentities(items)];
  next.splice(at, count, ...insert);
  keys.splice(at, count, ...insert.map(() => `item-${++sequence}`));
  slots.set(next, keys);
  return next;
}
export function moveItem<T>(
  items: readonly T[],
  from: number,
  to: number,
): T[] {
  const next = [...items],
    keys = [...itemIdentities(items)];
  next.splice(to, 0, next.splice(from, 1)[0]);
  keys.splice(to, 0, keys.splice(from, 1)[0]);
  slots.set(next, keys);
  return next;
}
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
    Array.isArray(previous) &&
    Array.isArray(next) &&
    !slots.has(next) &&
    previous.length === next.length
  )
    slots.set(next, [...itemIdentities(previous)]);
  if (
    previous &&
    next &&
    typeof previous === "object" &&
    typeof next === "object" &&
    !identities.has(next)
  )
    identities.set(next, identity(previous));
}

export function withIdentity<T>(previous: unknown, next: T): T {
  inheritIdentity(previous, next);
  return next;
}
