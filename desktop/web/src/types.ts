export type WireValue =
  | { kind: "text" | "literal"; value: string }
  | { kind: "null" }
  | { kind: "sequence"; value: WireValue[] }
  | { kind: "mapping"; value: [string, WireValue][] };
export interface Field {
  key: number;
  name: string;
  typeName: string;
  nullable: boolean;
  array: boolean;
}
export interface Shape {
  category: string;
  typeName: string;
  nullable: boolean;
  array: boolean;
  underlying?: string;
  members?: string[];
  fields?: [Field, Shape][];
}
export interface Cell {
  value: WireValue | null;
  display: string;
  valid: boolean;
  editable: boolean;
  reason: string | null;
  problem: { code: string; message: string; path: string[] } | null;
}
export interface Row {
  id: string;
  occurrence: number;
  viewIndex: number;
  pendingDelete: boolean;
  added: boolean;
  cells: Cell[];
}
export interface Work {
  projectDiscovery: number;
  projectEnumeration: number;
  projectYamlParse: number;
  projectValidation: number;
  localParse: number;
  bytesRead: number;
}
export interface Measurement {
  elapsedMs: number;
  work: Work;
  stagesMs: Record<string, number>;
}
export interface Projection {
  sessionEpoch: number;
  clicked: string;
  source: string | null;
  table: { name: string; csharpName: string; source: string; fields: Field[] };
  sources: string[];
  columns: { field: Field; shape: Shape | null; reason: string | null }[];
  rows: Row[];
  totalRows: number;
  sourceTotalRows: number;
  rowStart: number;
  revision: number;
  schemaRevision: number;
  generation: number;
  dirty: boolean;
  schemaDirty: boolean;
  canUndo: boolean;
  canRedo: boolean;
  schemaCanUndo: boolean;
  schemaCanRedo: boolean;
  conflict: boolean;
  writeStates: { source: string; outcome: string }[];
  canAdd: boolean;
  addReason: string | null;
  viewState: {
    search: string;
    selectedRow: string | null;
    selectedField: string | null;
    scrollTop: number;
    scrollLeft: number;
  };
  measurement: Measurement;
}
export type TypeDeclaration =
  | {category:"valueObject";underlying:string;from_implicit:boolean;to_implicit:boolean}
  | {category:"enum";underlying:string;flags:boolean;members:[string,string][]}
  | {category:"custom";fields:Field[]};
export interface TypeProjection {
  kind:"type";
  sessionEpoch:number;
  clicked:string;
  source:string;
  name:string;
  identity:string;
  declaration:TypeDeclaration;
  protectedMembers:string[];
  generation:number;
  measurement:Measurement;
}
export interface DeclarationInput {key:string;name:string;typeName:string;nullable:boolean;array:boolean;}
export type ConstantInput = {kind:"unset"} | {kind:"null"} | {kind:"scalar";text:string} | {kind:"sequence";value:ConstantInput[]} | {kind:"mapping";value:[string,ConstantInput][]};
export type TypeOperation =
  | {operation:"setValueObjectConversions";typeName:string;fromImplicit:boolean;toImplicit:boolean}
  | {operation:"addEnumMember";typeName:string;name:string;value:string}
  | {operation:"renameEnumMember";typeName:string;member:string;newName:string}
  | {operation:"dropEnumMember";typeName:string;member:string}
  | {operation:"addCustomField";typeName:string;declaration:DeclarationInput;initializer:null}
  | {operation:"renameCustomField";typeName:string;field:string;newName:string}
  | {operation:"dropCustomField";typeName:string;field:string};
export interface Inventory {
  project: { id: string; name: string; version: string };
  root: string;
  roots: string[];
  folders: string[];
  sources: {
    path: string;
    kind: string | null;
    binding: string | null;
    error: string | null;
  }[];
  types: string[];
  dirty: string[];
  uncertain: string[];
  recoveryRequired: boolean;
  recovery: RecoveryInfo[];
  generation: number;
  externalVersion: number;
  environmentError: string | null;
}
export interface RecoveryInfo {
  id: string;
  directory: string;
  message: string;
  files: { source: string; state: string; oldCopy: string; newCopy: string }[];
}
export type FieldOperation =
  | { kind: "rename"; field: string; newName: string }
  | { kind: "add"; neighbor: string | null; after: boolean }
  | { kind: "drop"; field: string };
export interface MigrationReview {
  token: string;
  command: { operation: string; table?: string; declaration?: { name: string }; newName?: string };
  destructive: boolean;
  affectedRecords: number;
  files: { source: string; beforeIdentity: string; afterIdentity: string; beforeBytes: number; afterBytes: number }[];
  dirtySources:string[];
}
export interface SetResult {
  outcome: string;
  message: string;
  files: { source: string; state: string; commit: { outcome: string; message: string } | null; rollback: { outcome: string; message: string } | null }[];
  recovery: RecoveryInfo | null;
}
export interface Status {
  open: boolean;
  epoch: number;
  generation: number;
  dirty: string[];
  recoveryRequired: boolean;
  diagnosticsPending: boolean;
  problemCount: number;
  uncertain: string[];
  externalVersion: number;
  environmentError: string | null;
}
export interface Diagnostic {
  code: string;
  kind: string;
  message: string;
  source: string;
  line: number;
  column: number;
  table: string | null;
  occurrence: number | null;
  fieldPath: string[];
  generation: number;
}
export interface HostTiming {
  epoch: number;
  queuedMs: number;
  backendMs: number;
  serializationMs: number;
  bytes: number;
  token: number | null;
  work: Work;
  stagesMs: Record<string, number>;
  nativeCompleteMs: number;
}
export interface Reply<T> {
  data: T;
  host: HostTiming;
}
export interface SelectionSample {
  target: string;
  token: number;
  caseName: string;
  input: number;
  selectionPublication: number;
  ipcReturn?: number;
  statePublication?: number;
  domCommit?: number;
  reactCommit?: number;
  layout?: number;
  paintOpportunity?: number;
  firstAccepted?: number;
  mountedRows?: number;
  host?: HostTiming;
  engine?: Measurement;
  invalid?: string;
}
