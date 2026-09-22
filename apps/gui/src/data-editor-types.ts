export type AuthoringValue =
  | { kind: "null" }
  | { kind: "invalid"; diagnostic: { code: string; message: string } }
  | { kind: "bool"; value: boolean }
  | { kind: "number"; value: string }
  | { kind: "string"; value: string }
  | { kind: "sequence"; sourceIdentity: boolean; items: AuthoringSequenceItem[] }
  | { kind: "mapping"; entries: AuthoringMember[] };

export type AuthoringSequenceItem = { sourceIndex: number | null; value: AuthoringValue };
export type AuthoringMember = { name: string; value: AuthoringValue };

export type PrimitiveType = "bool" | "int" | "uint" | "long" | "ulong" | "float" | "double" | "string";
export type FieldModifier = "required" | "nullable" | "array";

export type ResolvedAuthoringType =
  | { kind: "primitive"; primitive: PrimitiveType }
  | { kind: "value_object"; name: string; underlying: PrimitiveType }
  | { kind: "enum"; name: string; underlying: PrimitiveType; members: string[] }
  | { kind: "flags"; name: string; underlying: PrimitiveType; members: string[] }
  | { kind: "custom"; name: string; fields: ResolvedAuthoringField[] };

export type ResolvedAuthoringField = {
  name: string;
  typeName: string;
  modifier: FieldModifier;
  shape: ResolvedAuthoringType;
};

export const nullAuthoringValue = (): AuthoringValue => ({ kind: "null" });

export function authoringValueSummary(value: AuthoringValue): string {
  switch (value.kind) {
    case "null": return "null";
    case "invalid": return `invalid: ${value.diagnostic.code}`;
    case "bool": return value.value ? "true" : "false";
    case "number": return value.value;
    case "string": return JSON.stringify(value.value);
    case "sequence": return `[${value.items.map((item) => authoringValueSummary(item.value)).join(", ")}]`;
    case "mapping": return `{${value.entries.map(({ name, value: entry }) => `${JSON.stringify(name)}: ${authoringValueSummary(entry)}`).join(", ")}}`;
  }
}

export function authoringValuesEqual(left: AuthoringValue, right: AuthoringValue): boolean {
  if (left.kind !== right.kind) return false;
  switch (left.kind) {
    case "null": return true;
    case "invalid": return right.kind === "invalid" && left.diagnostic.code === right.diagnostic.code && left.diagnostic.message === right.diagnostic.message;
    case "bool": return right.kind === "bool" && left.value === right.value;
    case "number": return right.kind === "number" && left.value === right.value;
    case "string": return right.kind === "string" && left.value === right.value;
    case "sequence": return right.kind === "sequence"
      && left.sourceIdentity === right.sourceIdentity
      && left.items.length === right.items.length
      && left.items.every((item, index) => item.sourceIndex === right.items[index].sourceIndex
        && authoringValuesEqual(item.value, right.items[index].value));
    case "mapping": return right.kind === "mapping"
      && left.entries.length === right.entries.length
      && left.entries.every((entry, index) => entry.name === right.entries[index].name
        && authoringValuesEqual(entry.value, right.entries[index].value));
  }
}
