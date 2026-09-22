import { Checkbox } from "antd";
import ValueEditor from "./ValueEditor";
import type { AuthoringValue, FieldModifier, ResolvedAuthoringField, ResolvedAuthoringType } from "./data-editor-types";
import { nullAuthoringValue } from "./data-editor-types";

export function initializerJson(value: AuthoringValue): string {
  switch (value.kind) {
    case "null":
      return "null";
    case "invalid":
      return "null";
    case "bool":
      return value.value ? "true" : "false";
    case "number":
      return value.value;
    case "string":
      return JSON.stringify(value.value);
    case "sequence":
      return `[${value.items.map((item) => initializerJson(item.value)).join(",")}]`;
    case "mapping":
      return `{${value.entries.map((entry) => `${JSON.stringify(entry.name)}:${initializerJson(entry.value)}`).join(",")}}`;
  }
}

export function TypedInitializer({
  typeName,
  modifier,
  shape,
  enabled,
  value,
  disabled,
  onEnabledChange,
  onChange,
}: {
  typeName: string;
  modifier: FieldModifier;
  shape: ResolvedAuthoringType | null;
  enabled: boolean;
  value: AuthoringValue;
  disabled: boolean;
  onEnabledChange: (enabled: boolean) => void;
  onChange: (value: AuthoringValue) => void;
}) {
  const field: ResolvedAuthoringField | null = shape
    ? { name: "initializer", typeName, modifier, shape }
    : null;

  return (
    <section className="typed-initializer" aria-label="Typed initializer">
      <Checkbox
        checked={enabled}
        disabled={disabled || !field}
        onChange={(event) => onEnabledChange(event.target.checked)}
      >
        Explicit constant initializer
      </Checkbox>
      {!field && <p>Initializer controls are unavailable because this field type could not be resolved.</p>}
      {enabled && field && (
        <div className="typed-initializer-value">
          <ValueEditor
            field={field}
            value={value}
            label="Initializer"
            cellKey="migration-initializer"
            editable={!disabled}
            invalidPaths={new Set()}
            onChange={onChange}
          />
          <p>Initializer is represented by the shared resolved type model. 64-bit integer digits remain text until Rust parses the reviewed value.</p>
          {modifier === "nullable" && value.kind === "null" && <p>Explicit null is selected. Unchecking the initializer removes the initializer entirely.</p>}
        </div>
      )}
    </section>
  );
}

export function resetInitializer(): AuthoringValue {
  return nullAuthoringValue();
}
