import { Button, Checkbox, Input, Select } from "antd";
import type { AuthoringMember, AuthoringSequenceItem, AuthoringValue, ResolvedAuthoringField, ResolvedAuthoringType } from "./data-editor-types";
import { authoringValueSummary, nullAuthoringValue } from "./data-editor-types";

type ValueEditorProps = {
  field: ResolvedAuthoringField;
  value: AuthoringValue;
  label: string;
  cellKey: string;
  editable: boolean;
  invalidPaths: ReadonlySet<string>;
  onChange: (value: AuthoringValue) => void;
};

export default function ValueEditor(props: ValueEditorProps) {
  return <FieldValueEditor {...props} path="" />;
}

function FieldValueEditor({ field, value, label, cellKey, editable, invalidPaths, onChange, path, primary = true }: ValueEditorProps & { path: string; primary?: boolean }) {
  const invalid = invalidPaths.has(path);
  if (field.modifier === "array") {
    return (
      <fieldset className={`value-editor array-editor ${invalid ? "invalid" : ""}`} aria-invalid={invalid || undefined}>
        <legend>{label} <span className="value-editor-type">Array of {typeName(field.shape)}</span></legend>
        {value.kind === "sequence" ? (
          <div className="array-value-list">
            {value.items.map((item, index) => (
              <div className="array-value-item" key={index}>
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
                  onChange={(next) => onChange({
                    kind: "sequence",
                    sourceIdentity: true,
                    items: value.items.map((existing, itemIndex) => itemIndex === index
                      ? { ...existing, value: next }
                      : existing),
                  })}
                />
                <div className="array-item-actions">
                  <Button
                    size="small"
                    htmlType="button"
                    aria-label={`Move ${label} item ${index + 1} up`}
                    disabled={!editable || index === 0}
                    onClick={() => {
                      const items = [...value.items];
                      [items[index - 1], items[index]] = [items[index], items[index - 1]];
                      onChange({ kind: "sequence", sourceIdentity: true, items });
                    }}
                  >Move up</Button>
                  <Button
                    size="small"
                    htmlType="button"
                    aria-label={`Move ${label} item ${index + 1} down`}
                    disabled={!editable || index === value.items.length - 1}
                    onClick={() => {
                      const items = [...value.items];
                      [items[index], items[index + 1]] = [items[index + 1], items[index]];
                      onChange({ kind: "sequence", sourceIdentity: true, items });
                    }}
                  >Move down</Button>
                  <Button
                    size="small"
                    danger
                    htmlType="button"
                    aria-label={`Remove ${label} item ${index + 1}`}
                    disabled={!editable}
                    onClick={() => onChange({
                      kind: "sequence",
                      sourceIdentity: true,
                      items: value.items.filter((_, itemIndex) => itemIndex !== index),
                    })}
                  >Remove</Button>
                </div>
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
                onClick={() => onChange({ kind: "sequence", sourceIdentity: true, items: [] })}
              >Make empty array</Button>
              <Button
                htmlType="button"
                data-value-path={valuePathKey(cellKey, path)}
                aria-label={`Add first ${label} array item`}
                disabled={!editable}
                onClick={() => onChange({ kind: "sequence", sourceIdentity: true, items: [newSequenceItem(nullAuthoringValue())] })}
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
            onClick={() => onChange({ kind: "sequence", sourceIdentity: true, items: [...value.items, newSequenceItem(nullAuthoringValue())] })}
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
        onChange={onChange}
      />
      {field.modifier === "nullable" && value.kind !== "null" && (
        <Button
          size="small"
          htmlType="button"
          data-value-path={valuePathKey(cellKey, path)}
          aria-label={`Set ${label} to null`}
          disabled={!editable}
          onClick={() => onChange(nullAuthoringValue())}
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
  onChange,
}: {
  shape: ResolvedAuthoringType;
  value: AuthoringValue;
  label: string;
  cellKey: string;
  dataPath: string;
  primary: boolean;
  editable: boolean;
  invalidPaths: ReadonlySet<string>;
  onChange: (value: AuthoringValue) => void;
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
            onChange={(next: string) => onChange({ kind: "bool", value: next === "true" })}
          />
        );
      }
      if (shape.primitive === "string") {
        return (
          <Input
            {...sharedControlProps}
            aria-label={label}
            value={scalarText(value)}
            readOnly={!editable}
            onChange={(event) => onChange({ kind: "string", value: event.target.value })}
          />
        );
      }
      return (
        <Input
          {...sharedControlProps}
          aria-label={label}
          inputMode={shape.primitive === "float" || shape.primitive === "double" ? "decimal" : "numeric"}
          value={scalarText(value)}
          readOnly={!editable}
          onChange={(event) => onChange({ kind: "number", value: event.target.value })}
        />
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
            onChange={onChange}
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
            onChange={(member: string) => onChange({ kind: "string", value: member })}
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
          onChange={onChange}
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
              onClick={() => onChange({
                kind: "mapping",
                entries: shape.fields.map((field) => ({ name: field.name, value: nullAuthoringValue() })),
              })}
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
                  onChange={(next) => updateMember(nestedField.name, next)}
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
