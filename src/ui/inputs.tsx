import React, { useEffect, useRef, useState } from "react";
import { Button, Input, Select, Space, Switch, Tag, Tooltip } from "antd";
import {
  ArrowRightOutlined,
  CloseOutlined,
  PlusOutlined,
  RollbackOutlined,
} from "@ant-design/icons";
import {
  type Field,
  type Data,
  type Snapshot,
  typeSource,
  typeKind,
  text,
  primitives,
  isObject,
} from "../core/types";
import { defaultValue, flagsChange, primitiveOf } from "../core/schema";
export function Entry({
  value,
  onCommit,
  label,
  numeric = false,
  className = "",
  placeholder = "",
  autoFocus = false,
}: {
  value: Data;
  onCommit: (value: string) => void;
  label: string;
  numeric?: boolean;
  className?: string;
  placeholder?: string;
  autoFocus?: boolean;
}) {
  const [draft, setDraft] = useState(text(value));
  const focused = useRef(false);
  const cancel = useRef(false);
  useEffect(() => {
    if (!focused.current) setDraft(text(value));
  }, [value]);
  return (
    <Input
      aria-label={label}
      autoFocus={autoFocus}
      className={`entry ${className}`}
      value={draft}
      placeholder={placeholder}
      inputMode={numeric ? "decimal" : undefined}
      onFocus={(e) => {
        focused.current = true;
        cancel.current = false;
        if (autoFocus) e.target.select();
      }}
      onChange={(e) => {
        setDraft(e.target.value);
        window.desktop?.dirty(true);
      }}
      onBlur={() => {
        focused.current = false;
        if (!cancel.current && draft !== text(value)) {
          try {
            onCommit(draft);
          } catch (error) {
            window.dispatchEvent(
              new CustomEvent("masterdata:error", { detail: error }),
            );
            setDraft(text(value));
          }
        }
        cancel.current = false;
      }}
      onKeyDown={(e) => {
        if (e.nativeEvent.isComposing) return;
        if (e.key === "Escape") {
          cancel.current = true;
          setDraft(text(value));
          e.currentTarget.blur();
          e.stopPropagation();
        }
        if (e.key === "Enter") {
          const grid = e.currentTarget.closest<HTMLElement>(".grid-editor");
          e.currentTarget.blur();
          grid?.focus();
          e.stopPropagation();
        }
        if (
          (e.metaKey || e.ctrlKey) &&
          e.key.toLowerCase() === "z" &&
          draft !== text(value)
        ) {
          e.stopPropagation();
        }
      }}
    />
  );
}
export function parseInput(value: string, type: string): Data {
  if (["int", "uint", "long", "ulong"].includes(type)) {
    try {
      return /^[-+]?\d+$/.test(value.trim()) ? BigInt(value.trim()) : value;
    } catch {
      return value;
    }
  }
  if (type === "float" || type === "double")
    return value.trim() !== "" && Number.isFinite(Number(value))
      ? Number(value)
      : value;
  if (type === "bool")
    return value.toLowerCase() === "true"
      ? true
      : value.toLowerCase() === "false"
        ? false
        : value;
  return value;
}
export function TypeSelect({
  snapshot,
  value,
  onChange,
}: {
  snapshot: Snapshot;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <Select
      aria-label="型"
      className="type-select"
      showSearch
      optionFilterProp="label"
      value={value}
      onChange={onChange}
      options={[
        {
          label: "基本の型",
          options: primitives.map((t) => ({ value: t, label: t })),
        },
        {
          label: "定義した型",
          options: snapshot.sources
            .filter((s) => s.data?.kind === "type")
            .map((s) => ({
              value: String(s.data.name),
              label: String(s.data.name),
            })),
        },
      ]}
      popupMatchSelectWidth={240}
    />
  );
}
export interface ReferenceOption {
  value: string;
  label: string;
  actual: Data;
  source: string;
  index: number;
}
export function ValueControl({
  snapshot,
  field,
  value,
  onChange,
  onOpen,
  label,
  reference,
  navigate,
}: {
  snapshot: Snapshot;
  field: Field;
  value: Data;
  onChange: (value: Data) => void;
  onOpen?: () => void;
  label: string;
  reference?: ReferenceOption[];
  navigate?: (value: ReferenceOption) => void;
}) {
  const type = typeSource(snapshot, field.type);
  const kind = typeKind(type);
  const primitive = primitiveOf(snapshot, field);
  let content: React.ReactNode;
  if (value === null || value === undefined)
    content = (
      <Button
        className="null-value"
        type="dashed"
        size="small"
        icon={<PlusOutlined />}
        aria-label={`${label}に値を作成`}
        onClick={() => {
          onChange(defaultValue(snapshot, { ...field, nullable: false }));
          if (field.array || kind === "custom") onOpen?.();
        }}
      >
        {value === null ? "null · 値を作成" : "未入力 · 値を作成"}
      </Button>
    );
  else if (field.array || kind === "custom") {
    const valid = field.array ? Array.isArray(value) : isObject(value);
    content = (
      <Button
        className={`complex-value ${valid ? "" : "invalid"}`}
        type="text"
        aria-label={`${label}を開く`}
        onClick={onOpen}
      >
        {field.array
          ? `[ ] ${valid ? `${value.length} 要素` : "修復する"}`
          : `${field.type} ${valid ? "{ … }" : "· 修復する"}`}
        <ArrowRightOutlined />
      </Button>
    );
  } else if (reference) {
    const selected = reference.find((r) => text(r.actual) === text(value));
    content = (
      <Space.Compact className="reference-value">
        <Select
          aria-label={label}
          showSearch
          optionFilterProp="label"
          value={selected?.value ?? text(value)}
          options={reference}
          onChange={(key) => {
            const r = reference.find((r) => r.value === key);
            if (r) onChange(r.actual);
          }}
        />
        <Tooltip title="参照先へ移動">
          <Button
            size="small"
            type="text"
            aria-label="参照先へ移動"
            disabled={!selected}
            icon={<ArrowRightOutlined />}
            onClick={() => selected && navigate?.(selected)}
          />
        </Tooltip>
      </Space.Compact>
    );
  } else if (primitive === "bool")
    content = (
      <div className="boolean-value">
        <Switch
          aria-label={label}
          checked={value === true}
          checkedChildren="True"
          unCheckedChildren="False"
          onChange={onChange}
        />
        {typeof value !== "boolean" && <Tag color="error">{text(value)}</Tag>}
      </div>
    );
  else if (kind === "enum" || kind === "flags") {
    const members = type!.data[kind].members ?? [];
    content = (
      <Select
        aria-label={label}
        className="enum-value"
        showSearch
        optionFilterProp="label"
        mode={kind === "flags" ? "multiple" : undefined}
        maxTagCount="responsive"
        value={
          kind === "flags"
            ? Array.isArray(value)
              ? value
              : [text(value)]
            : text(value)
        }
        options={members.map((m: any) => ({
          value: m.name,
          label: m.name === "None" ? "None · なし" : m.name,
        }))}
        onChange={(v) =>
          onChange(kind === "flags" ? flagsChange(value, v as string[]) : v)
        }
      />
    );
  } else
    content = (
      <Entry
        value={value}
        label={label}
        numeric={primitive !== "string"}
        onCommit={(v) => onChange(parseInput(v, primitive))}
      />
    );
  return (
    <div
      className={`value-control ${field.nullable ? "nullable" : ""}`}
      data-type={field.type}
    >
      {content}
      <span className="nullable-slot">
        {field.nullable && value !== null && value !== undefined && (
          <Tooltip title="null（値なし）にする">
            <Button
              type="text"
              size="small"
              aria-label={`${label}をnullにする`}
              icon={<CloseOutlined />}
              onClick={() => onChange(null)}
            />
          </Tooltip>
        )}
      </span>
    </div>
  );
}
