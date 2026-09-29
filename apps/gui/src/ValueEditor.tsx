import { useEffect, useRef, useState } from "react";
import { Button, Checkbox, Dropdown, Input, Select } from "antd";
import { GripVertical, MoreHorizontal } from "lucide-react";
import type { AuthoringMember, AuthoringSequenceItem, AuthoringValue, ResolvedAuthoringField, ResolvedAuthoringType } from "./data-editor-types";
import { authoringValueSummary, nullAuthoringValue } from "./data-editor-types";

export type ValueEditorProps = {
  field: ResolvedAuthoringField;
  value: AuthoringValue;
  label: string;
  cellKey: string;
  editable: boolean;
  invalidPaths: ReadonlySet<string>;
  onChange: (value: AuthoringValue) => void;
  onOperation?: (value: AuthoringValue) => void;
  bufferedText?: boolean;
};

export default function ValueEditor(props: ValueEditorProps) {
  return <FieldValueEditor {...props} path="" />;
}

function FieldValueEditor({ field, value, label, cellKey, editable, invalidPaths, onChange, onOperation, bufferedText = false, path, primary = true }: ValueEditorProps & { path: string; primary?: boolean }) {
  const draggingItem = useRef<number | null>(null);
  const pointerCleanup = useRef<(() => void) | null>(null);
  const [dropPosition, setDropPosition] = useState<number | null>(null);
  useEffect(() => () => pointerCleanup.current?.(), []);
  const invalid = invalidPaths.has(path);
  const operation = onOperation ?? onChange;
  const reorderArrayItem = (target: number) => {
    const from = draggingItem.current;
    draggingItem.current = null;
    setDropPosition(null);
    if (from === null || value.kind !== "sequence") return;
    const destination = from < target ? target - 1 : target;
    if (destination === from) return;
    const items = [...value.items];
    const [moved] = items.splice(from, 1);
    items.splice(destination, 0, moved);
    operation({ kind: "sequence", sourceIdentity: true, items });
    focusValuePath(cellKey, joinPath(path, String(destination)), true);
  };
  if (field.modifier === "array") {
    return (
      <fieldset className={`value-editor array-editor ${invalid ? "invalid" : ""}`} aria-invalid={invalid || undefined}>
        <legend>{label} <span className="value-editor-type">Array of {typeName(field.shape)}</span></legend>
        {value.kind === "sequence" ? (
          <div className="array-value-list">
            {value.items.map((item, index) => (
              <div data-array-index={index} className={`array-value-item ${dropPosition === index ? "drop-before" : ""} ${dropPosition === index + 1 ? "drop-after" : ""}`} key={sequenceItemKey(item)}>
                <button className="array-item-grab" type="button" disabled={!editable}
                  aria-label={`Drag ${label} item ${index + 1} to reorder`}
                  onPointerDown={event => {
                    if (!editable || (event.button != null && event.button !== 0)) return;
                    const list = event.currentTarget.closest<HTMLElement>(".array-value-list");
                    if (!list) return;
                    const startY = event.clientY;
                    const pointerId = event.pointerId;
                    let target: number | null = null;
                    let moved = false;
                    let pointerX = event.clientX;
                    let pointerY = event.clientY;
                    draggingItem.current = index;
                    const locate = () => {
                      const item = document.elementFromPoint(pointerX, pointerY)?.closest<HTMLElement>(".array-value-item");
                      if (item && list.contains(item)) {
                        const rect = item.getBoundingClientRect();
                        target = Number(item.dataset.arrayIndex) + (pointerY >= rect.top + rect.height / 2 ? 1 : 0);
                        setDropPosition(target);
                      } else { target = null; setDropPosition(null); }
                    };
                    const edgeScroll = () => {
                      if (!moved) return;
                      const scroll = list.closest<HTMLElement>(".ant-popover-inner");
                      if (scroll) {
                        const rect = scroll.getBoundingClientRect();
                        if (pointerY < rect.top + 28) scroll.scrollTop -= 12;
                        else if (pointerY > rect.bottom - 28) scroll.scrollTop += 12;
                      }
                      locate();
                    };
                    const timer = window.setInterval(edgeScroll, 30);
                    const move = (pointer: PointerEvent) => {
                      if (pointer.pointerId !== pointerId) return;
                      pointerX = pointer.clientX;
                      pointerY = pointer.clientY;
                      if (Math.abs(pointerY - startY) < 5 && !moved) return;
                      moved = true;
                      locate();
                    };
                    const cleanup = () => {
                      window.clearInterval(timer);
                      window.removeEventListener("pointermove", move);
                      window.removeEventListener("pointerup", up);
                      window.removeEventListener("pointercancel", cancel);
                      pointerCleanup.current = null;
                    };
                    const cancel = (pointer: PointerEvent) => { if (pointer.pointerId !== pointerId) return; cleanup(); draggingItem.current = null; setDropPosition(null); };
                    const up = (pointer: PointerEvent) => { if (pointer.pointerId !== pointerId) return; cleanup(); if (moved && target !== null) reorderArrayItem(target); else { draggingItem.current = null; setDropPosition(null); } };
                    pointerCleanup.current?.();
                    pointerCleanup.current = cleanup;
                    window.addEventListener("pointermove", move);
                    window.addEventListener("pointerup", up);
                    window.addEventListener("pointercancel", cancel);
                  }}><GripVertical size={14} /></button>
                <span className="array-item-label">Item {index + 1}</span>
                <TypeValueEditor
                  shape={field.shape}
                  value={item.value}
                  label={`${label} item ${index + 1}`}
                  cellKey={cellKey}
                  dataPath={joinPath(path, String(index))}
                  primary={primary && index === 0}
                  editable={editable}
                  invalidPaths={invalidPaths}
                  bufferedText={bufferedText}
                  onOperation={(next) => {
                    const changed = { ...item, value: next };
                    carrySequenceItemKey(item, changed);
                    operation({ kind: "sequence", sourceIdentity: true,
                      items: value.items.map((existing, itemIndex) => itemIndex === index ? changed : existing) });
                  }}
                  onChange={(next) => {
                    const changed = { ...item, value: next };
                    carrySequenceItemKey(item, changed);
                    onChange({ kind: "sequence", sourceIdentity: true,
                      items: value.items.map((existing, itemIndex) => itemIndex === index ? changed : existing) });
                  }}
                />
                <Dropdown trigger={["click"]} menu={{ items: [
                  { key: "up", label: `Move ${label} item ${index + 1} up`, disabled: !editable || index === 0, onClick: () => {
                    const items = [...value.items]; [items[index - 1], items[index]] = [items[index], items[index - 1]];
                    operation({ kind: "sequence", sourceIdentity: true, items });
                    focusValuePath(cellKey, joinPath(path, String(index - 1)), true);
                  } },
                  { key: "down", label: `Move ${label} item ${index + 1} down`, disabled: !editable || index === value.items.length - 1, onClick: () => {
                    const items = [...value.items]; [items[index], items[index + 1]] = [items[index + 1], items[index]];
                    operation({ kind: "sequence", sourceIdentity: true, items });
                    focusValuePath(cellKey, joinPath(path, String(index + 1)), true);
                  } },
                  { key: "remove", label: `Remove ${label} item ${index + 1}`, danger: true, disabled: !editable, onClick: () => {
                    const items = value.items.filter((_, itemIndex) => itemIndex !== index);
                    operation({ kind: "sequence", sourceIdentity: true, items });
                    focusValuePath(cellKey, items.length ? joinPath(path, String(Math.min(index, items.length - 1))) : path, items.length > 0);
                  } },
                ] }}><Button className="array-item-menu" type="text" size="small" htmlType="button" data-value-path={valuePathKey(cellKey, joinPath(path, String(index)))}
                  aria-label={`Actions for ${label} item ${index + 1}`} disabled={!editable} icon={<MoreHorizontal size={15} />} /></Dropdown>
              </div>
            ))}
          </div>
        ) : (
          <div className="unmaterialized-value">
            {value.kind !== "null" && <output>Current source value: {authoringValueSummary(value)}</output>}
            {value.kind === "null" && <span>Array is not materialized.</span>}
            <div className="value-editor-actions">
              <Button
                htmlType="button"
                data-cell={cellKey}
                data-value-path={valuePathKey(cellKey, path)}
                aria-label={`Make empty array for ${label}`}
                disabled={!editable}
                onClick={() => { operation({ kind: "sequence", sourceIdentity: true, items: [] }); focusValuePath(cellKey, path); }}
              >Make empty array</Button>
              <Button
                htmlType="button"
                data-value-path={valuePathKey(cellKey, path)}
                aria-label={`Add first ${label} array item`}
                disabled={!editable}
                onClick={() => { operation({ kind: "sequence", sourceIdentity: true, items: [newSequenceItem(nullAuthoringValue())] }); focusValuePath(cellKey, joinPath(path, "0")); }}
              >Add first item</Button>
            </div>
          </div>
        )}
        {value.kind === "sequence" && (
          <Button
            className="array-add-item"
            htmlType="button"
            data-cell={cellKey}
            data-value-path={valuePathKey(cellKey, path)}
            aria-label={`Add ${label} item`}
            disabled={!editable}
            onClick={() => { operation({ kind: "sequence", sourceIdentity: true, items: [...value.items, newSequenceItem(nullAuthoringValue())] }); focusValuePath(cellKey, joinPath(path, String(value.items.length))); }}
          >Add item</Button>
        )}
        {invalid && <span className="nested-value-error" role="alert">This array value is invalid.</span>}
      </fieldset>
    );
  }

  return (
    <fieldset className={`value-editor field-value-editor ${invalid ? "invalid" : ""}`} aria-invalid={invalid || undefined}>
      <legend>{label} <span className="value-editor-type">{field.typeName}{field.modifier === "nullable" ? " · Nullable" : ""}</span></legend>
      <TypeValueEditor
        shape={field.shape}
        value={value}
        label={label}
        cellKey={cellKey}
        dataPath={path}
        primary={primary}
        editable={editable}
        invalidPaths={invalidPaths}
        bufferedText={bufferedText}
        onChange={onChange}
        onOperation={operation}
      />
      {field.modifier === "nullable" && value.kind !== "null" && (
        <Button
          size="small"
          htmlType="button"
          data-value-path={valuePathKey(cellKey, path)}
          aria-label={`Set ${label} to null`}
          disabled={!editable}
          onClick={() => operation(nullAuthoringValue())}
        >Set null</Button>
      )}
      {invalid && <span className="nested-value-error" role="alert">This value is invalid.</span>}
    </fieldset>
  );
}

function TypeValueEditor({
  shape,
  value,
  label,
  cellKey,
  dataPath,
  primary,
  editable,
  invalidPaths,
  bufferedText,
  onChange,
  onOperation,
}: {
  shape: ResolvedAuthoringType;
  value: AuthoringValue;
  label: string;
  cellKey: string;
  dataPath: string;
  primary: boolean;
  editable: boolean;
  invalidPaths: ReadonlySet<string>;
  bufferedText: boolean;
  onChange: (value: AuthoringValue) => void;
  onOperation: (value: AuthoringValue) => void;
}) {
  const invalid = invalidPaths.has(dataPath);
  const sharedControlProps = {
    "data-cell": primary ? cellKey : undefined,
    "data-value-path": valuePathKey(cellKey, dataPath),
    "aria-invalid": invalid || undefined,
  };
  switch (shape.kind) {
    case "primitive":
      if (shape.primitive === "bool") {
        const selected = value.kind === "bool" ? String(value.value) : "__invalid";
        const options = [
          ...(value.kind !== "null" && value.kind !== "bool"
            ? [{ label: `Current source value: ${authoringValueSummary(value)}`, value: "__invalid", disabled: true }]
            : []),
          { label: "True", value: "true" },
          { label: "False", value: "false" },
        ];
        return (
          <Select
            {...sharedControlProps}
            aria-label={label}
            placeholder="Choose true or false"
            value={value.kind === "bool" ? selected : value.kind === "null" ? undefined : "__invalid"}
            options={options}
            disabled={!editable}
            onChange={(next: string) => onOperation({ kind: "bool", value: next === "true" })}
          />
        );
      }
      if (shape.primitive === "string") {
        return (
          <ScalarTextInput controlProps={sharedControlProps} label={label} value={scalarText(value)}
            editable={editable} buffered={bufferedText} onCommit={(text) => onChange({ kind: "string", value: text })} />
        );
      }
      return (
        <ScalarTextInput controlProps={sharedControlProps} label={label} value={scalarText(value)}
          inputMode={shape.primitive === "float" || shape.primitive === "double" ? "decimal" : "numeric"}
          editable={editable} buffered={bufferedText} onCommit={(text) => onChange({ kind: "number", value: text })} />
      );
    case "value_object":
      return (
        <div className="value-object-editor" role="group" aria-label={`${label} (${shape.name})`}>
          <span className="value-editor-type">{shape.name} · {shape.underlying}</span>
          <TypeValueEditor
            shape={{ kind: "primitive", primitive: shape.underlying }}
            value={value}
            label={label}
            cellKey={cellKey}
            dataPath={dataPath}
            primary={primary}
            editable={editable}
            invalidPaths={invalidPaths}
            bufferedText={bufferedText}
            onChange={onChange}
            onOperation={onOperation}
          />
        </div>
      );
    case "enum": {
      const current = value.kind === "string" ? value.value : undefined;
      const unknownCurrent = current != null && !shape.members.includes(current);
      const options = [
        ...(unknownCurrent ? [{ label: `${current} (unknown member)`, value: current, disabled: true }] : []),
        ...shape.members.map((member) => ({ label: member, value: member })),
        ...(value.kind !== "null" && value.kind !== "string"
          ? [{ label: `Current source value: ${authoringValueSummary(value)}`, value: "__invalid", disabled: true }]
          : []),
      ];
      return (
        <div className="enum-editor">
          <Select
            {...sharedControlProps}
            aria-label={label}
            placeholder={`Choose ${shape.name}`}
            value={current ?? (value.kind === "null" ? undefined : "__invalid")}
            options={options}
            disabled={!editable}
            onChange={(member: string) => onOperation({ kind: "string", value: member })}
          />
          <span className="value-editor-type">{shape.name}</span>
        </div>
      );
    }
    case "flags":
      return (
        <FlagsEditor
          shape={shape}
          value={value}
          label={label}
          cellKey={cellKey}
          dataPath={dataPath}
          primary={primary}
          editable={editable}
          invalidPaths={invalidPaths}
          onChange={onOperation}
        />
      );
    case "custom": {
      if (value.kind !== "mapping") {
        return (
          <div className="custom-editor unmaterialized-value">
            {value.kind !== "null" && <output>Current source value: {authoringValueSummary(value)}</output>}
            {value.kind === "null" && <span>{shape.name} fields are not materialized.</span>}
            <Button
              htmlType="button"
              data-cell={primary ? cellKey : undefined}
              data-value-path={valuePathKey(cellKey, dataPath)}
              aria-label={`${value.kind === "null" ? "Materialize" : "Replace with"} ${label} (${shape.name}) fields`}
              disabled={!editable}
              onClick={() => {
                onOperation({ kind: "mapping", entries: shape.fields.map((field) => ({ name: field.name, value: nullAuthoringValue() })) });
                focusValuePath(cellKey, joinPath(dataPath, shape.fields[0]?.name ?? ""));
              }}
            >{value.kind === "null" ? "Materialize fields" : "Replace with fields"}</Button>
          </div>
        );
      }
      const knownNames = new Set(shape.fields.map((field) => field.name));
      const unknown = value.entries.filter((entry) => !knownNames.has(entry.name));
      const updateMember = (name: string, next: AuthoringValue) => {
        const index = value.entries.findIndex((entry) => entry.name === name);
        const entries = [...value.entries];
        if (index < 0) entries.push({ name, value: next });
        else entries[index] = { ...entries[index], value: next };
        onChange({ kind: "mapping", entries });
      };
      return (
        <div className="custom-editor" role="group" aria-label={`${label} (${shape.name})`}>
          <div className="value-editor-type">{shape.name}</div>
          <div className="custom-field-list">
            {shape.fields.map((nestedField, index) => {
              const current = value.entries.find((entry) => entry.name === nestedField.name)?.value ?? nullAuthoringValue();
              return (
                <FieldValueEditor
                  key={nestedField.name}
                  field={nestedField}
                  value={current}
                  label={nestedField.name}
                  cellKey={cellKey}
                  path={joinPath(dataPath, nestedField.name)}
                  editable={editable}
                  invalidPaths={invalidPaths}
                  bufferedText={bufferedText}
                  onChange={(next) => updateMember(nestedField.name, next)}
                  onOperation={(next) => {
                    const index = value.entries.findIndex((entry) => entry.name === nestedField.name);
                    const entries = [...value.entries];
                    if (index < 0) entries.push({ name: nestedField.name, value: next });
                    else entries[index] = { ...entries[index], value: next };
                    onOperation({ kind: "mapping", entries });
                  }}
                  primary={primary && index === 0}
                />
              );
            })}
          </div>
          {unknown.length > 0 && (
            <div className="unknown-custom-members" aria-label="Unknown Custom Type members">
              <strong>Unknown source members</strong>
              {unknown.map((entry) => (
                <div className="unknown-custom-member" key={entry.name}>
                  <output data-value-path={valuePathKey(cellKey, joinPath(dataPath, entry.name))} tabIndex={0}>
                    {entry.name}: {authoringValueSummary(entry.value)}
                  </output>
                </div>
              ))}
            </div>
          )}
        </div>
      );
    }
  }
}

function FlagsEditor({ shape, value, label, cellKey, dataPath, primary, editable, invalidPaths, onChange }: {
  shape: Extract<ResolvedAuthoringType, { kind: "flags" }>;
  value: AuthoringValue;
  label: string;
  cellKey: string;
  dataPath: string;
  primary: boolean;
  editable: boolean;
  invalidPaths: ReadonlySet<string>;
  onChange: (value: AuthoringValue) => void;
}) {
  const items = value.kind === "sequence" ? value.items : [];
  const selectedNames = items
    .map((item) => item.value)
    .filter((item): item is Extract<AuthoringValue, { kind: "string" }> => item.kind === "string");
  const isSelected = (member: string) => selectedNames.some((item) => item.value === member);
  const replaceItems = (member: string, checked: boolean) => {
    const isZero = member === "None";
    let next = [...items];
    if (isZero && checked) {
      next = next.filter((item) => !isKnownFlag(item.value, shape.members));
      if (!next.some((item) => flagName(item) === "None")) {
        next.push(newSequenceItem({ kind: "string", value: "None" }));
      }
    } else if (isZero) {
      next = next.filter((item) => flagName(item) !== "None");
    } else if (checked) {
      next = next.filter((item) => flagName(item) !== "None");
      if (!next.some((item) => flagName(item) === member)) {
        next.push(newSequenceItem({ kind: "string", value: member }));
      }
    } else {
      next = next.filter((item) => flagName(item) !== member);
    }
    onChange({ kind: "sequence", sourceIdentity: true, items: next });
  };
  const invalidSequenceValue = value.kind !== "null" && value.kind !== "sequence";
  return (
    <div className="flags-editor" role="group" aria-label={`${label} (${shape.name})`} aria-invalid={invalidPaths.has(dataPath) || undefined}>
      <span className="value-editor-type">{shape.name} · select members</span>
      {value.kind === "null" && <span className="value-editor-hint">No flags value selected yet.</span>}
      {value.kind === "sequence" && value.items.length === 0 && <span className="value-editor-hint">No flag members selected.</span>}
      {invalidSequenceValue && <output>Current source value: {authoringValueSummary(value)}</output>}
      <div className="flags-member-list">
        {shape.members.map((member, index) => (
          <Checkbox
            key={member}
            data-cell={primary && index === 0 ? cellKey : undefined}
            data-value-path={valuePathKey(cellKey, dataPath)}
            aria-label={`${label} ${member}`}
            checked={isSelected(member)}
            disabled={!editable}
            onChange={(event) => replaceItems(member, event.target.checked)}
          >{member}</Checkbox>
        ))}
      </div>
      {value.kind === "sequence" && value.items.map((item, index) => {
        if (isKnownFlag(item.value, shape.members)) return null;
        return (
          <div className="unknown-flag-member" key={index}>
            <output data-value-path={valuePathKey(cellKey, joinPath(dataPath, String(index)))} tabIndex={0}>
              Unrecognized flag value: {authoringValueSummary(item.value)}
            </output>
            <Button
              size="small"
              danger
              htmlType="button"
              data-value-path={valuePathKey(cellKey, joinPath(dataPath, String(index)))}
              aria-label={`Remove unrecognized ${label} flag ${index + 1}`}
              disabled={!editable}
              onClick={() => onChange({
                kind: "sequence",
                sourceIdentity: true,
                items: items.filter((_, itemIndex) => itemIndex !== index),
              })}
            >Remove</Button>
          </div>
        );
      })}
    </div>
  );
}

function ScalarTextInput({ controlProps, label, value, editable, buffered, inputMode, onCommit }: {
  controlProps: { "data-cell": string | undefined; "data-value-path": string; "aria-invalid": boolean | undefined };
  label: string;
  value: string;
  editable: boolean;
  buffered: boolean;
  inputMode?: "decimal" | "numeric";
  onCommit: (text: string) => void;
}) {
  const [draft, setDraft] = useState(value);
  const touched = useRef(false);
  useEffect(() => { setDraft(value); touched.current = false; }, [value]);
  const commit = () => {
    if (!buffered || !touched.current) return;
    touched.current = false;
    onCommit(draft);
  };
  return <Input {...controlProps} aria-label={label} inputMode={inputMode}
    value={buffered ? draft : value} readOnly={!editable}
    onChange={(event) => {
      if (buffered) { touched.current = true; setDraft(event.target.value); }
      else onCommit(event.target.value);
    }}
    onBlur={commit}
    onKeyDown={(event) => {
      if (!buffered || event.nativeEvent.isComposing) return;
      if (event.key === "Escape" && touched.current) {
        event.preventDefault(); event.stopPropagation();
        touched.current = false;
        setDraft(value);
      } else if (event.key === "Enter") {
        event.preventDefault(); event.stopPropagation(); commit();
      } else if (event.key === "Tab" || ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s")) {
        commit();
      }
    }} />;
}

// Presentation-only keys follow controls through reorder; sourceIndex remains the source identity.
const sequenceItemKeys = new WeakMap<AuthoringSequenceItem, string>();
let nextSequenceItemKey = 0;
function sequenceItemKey(item: AuthoringSequenceItem): string {
  let key = sequenceItemKeys.get(item);
  if (!key) { key = `item-${++nextSequenceItemKey}`; sequenceItemKeys.set(item, key); }
  return key;
}
function carrySequenceItemKey(previous: AuthoringSequenceItem, next: AuthoringSequenceItem) {
  sequenceItemKeys.set(next, sequenceItemKey(previous));
}

function focusValuePath(cellKey: string, path: string, itemAction = false) {
  window.requestAnimationFrame(() => {
    const selector = `[data-value-path="${CSS.escape(valuePathKey(cellKey, path))}"]`;
    const controls = [...document.querySelectorAll<HTMLElement>(selector)];
    const target = itemAction
      ? controls.find((control) => control.classList.contains("array-item-menu"))
      : controls.find((control) => control.matches("input, button, [tabindex]"));
    target?.focus();
  });
}

function newSequenceItem(value: AuthoringValue): AuthoringSequenceItem {
  return { sourceIndex: null, value };
}

function flagName(item: AuthoringSequenceItem): string | undefined {
  return item.value.kind === "string" ? item.value.value : undefined;
}

function isKnownFlag(value: AuthoringValue, members: string[]): boolean {
  return value.kind === "string" && members.includes(value.value);
}

function scalarText(value: AuthoringValue): string {
  switch (value.kind) {
    case "null": return "";
    case "invalid": return authoringValueSummary(value);
    case "bool": return value.value ? "true" : "false";
    case "number":
    case "string": return value.value;
    case "sequence":
    case "mapping": return authoringValueSummary(value);
  }
}

function typeName(shape: ResolvedAuthoringType): string {
  switch (shape.kind) {
    case "primitive": return shape.primitive;
    case "value_object":
    case "enum":
    case "flags":
    case "custom": return shape.name;
  }
}

function joinPath(path: string, segment: string): string {
  return `${path}/${segment.replaceAll("~", "~0").replaceAll("/", "~1")}`;
}

function valuePathKey(cellKey: string, path: string): string {
  return `${cellKey}${path}`;
}
