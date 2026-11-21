import { Button, Popover, Select, Space, Table } from "antd";
import {
  LinkOutlined,
  DeleteOutlined,
  UpOutlined,
  DownOutlined,
} from "@ant-design/icons";
import { useMemo, useState, type ComponentProps } from "react";
import {
  fieldsOf,
  list,
  tables,
  text,
  type Field,
  type Snapshot,
  type Source,
} from "../core/types";
import { identity, moveItem } from "../core/identity";
import { session } from "../core/session";
import { defaultValue } from "../core/schema";
import { isMasterRef, masterItems, sameKey } from "../core/master-values";
import { Entry, parseInput, TypeSelect, ValueControl } from "./inputs";
import { ModifierButtons } from "./fields";
import type { Cursor } from "./nested";

export function recordOptions(
  snapshot: Snapshot,
  source: Source,
  field: Field,
) {
  const ref = list(source.data.references).find(
    (r) =>
      r?.fields?.length === 1 &&
      r.fields[0] === field.name &&
      r.target?.fields?.length === 1,
  );
  const target =
    ref && tables(snapshot).find((t) => t.name === ref.target.table);
  return target?.sources.flatMap((s: Source) =>
    list(s.data.records).map((row, index) => {
      const actual = row?.[ref.target.fields[0]];
      const displayName = text(row?.name ?? row?.title ?? "");
      return {
        actual,
        displayName,
        label: `${text(actual)} ${displayName}`,
        value: `${s.id}:${index}`,
        source: s.id,
        index,
      };
    }),
  );
}

export function MasterValueEditor({
  source,
  snapshot,
  onOpen,
  navigate,
}: {
  source: Source;
  snapshot: Snapshot;
  navigate: (id: string) => void;
  onOpen: (cursor: Cursor) => void;
}) {
  const fields = fieldsOf(source);
  const numeric = source.data.keyMode === "number";
  const referenced = useMemo(
    () =>
      snapshot.sources.some((s) =>
        list(s.data?.records).some((row) =>
          Object.values(row ?? {}).some(
            (cell) =>
              isMasterRef(cell) &&
              (cell as any).$masterValue?.definition === source.data.table,
          ),
        ),
      ),
    [snapshot, source.data.table],
  );
  return (
    <div className="editor-scroll master-values-editor">
      <Space>
        <label>
          キー方式　
          <Select
            aria-label="キー方式"
            value={source.data.keyMode}
            disabled={!!fields.length || referenced}
            onChange={(value) => session.update(source.id, ["keyMode"], value)}
            options={[
              { value: "number", label: "数値" },
              { value: "string", label: "文字列" },
            ]}
          />
        </label>
        <Button onClick={() => session.addField(source.id)}>項目を追加</Button>
      </Space>
      <Table<Field>
        size="small"
        pagination={false}
        rowKey={identity}
        dataSource={fields}
        columns={[
          {
            title: "キー",
            render: (_, f, i) => (
              <Entry
                label="項目キー"
                value={f.valueKey}
                numeric={numeric}
                onCommit={(v) =>
                  session.renameMaster(
                    source.id,
                    source.data.table,
                    i,
                    numeric ? parseInput(v, "int") : v,
                  )
                }
              />
            ),
          },
          {
            title: "プロパティ名",
            render: (_, f, i) => (
              <Entry
                label="プロパティ名"
                value={f.name}
                onCommit={(v) => session.renameField(source.id, i, v)}
              />
            ),
          },
          {
            title: "型",
            render: (_, f, i) => (
              <Space>
                <TypeSelect
                  snapshot={snapshot}
                  value={f.type}
                  onChange={(v) =>
                    session.update(source.id, ["fields", i, "type"], v)
                  }
                />
                <ModifierButtons
                  field={f}
                  onChange={(v) => session.update(source.id, ["fields", i], v)}
                />
              </Space>
            ),
          },
          {
            title: "値",
            render: (_, f) => (
              <ValueControl
                snapshot={snapshot}
                field={f}
                value={source.data.records?.[0]?.[f.name]}
                label={`${f.name}の値`}
                reference={recordOptions(snapshot, source, f)}
                navigate={(option) => navigate(option.source)}
                onChange={(v) =>
                  session.update(source.id, ["records", 0, f.name], v)
                }
                onOpen={() =>
                  onOpen({
                    source: source.id,
                    field: f,
                    path: ["records", 0, f.name],
                  })
                }
              />
            ),
          },
          {
            title: "操作",
            render: (_, f, i) => (
              <Space size={0}>
                {[-1, 1].map((delta) => (
                  <Button
                    key={delta}
                    type="text"
                    size="small"
                    aria-label={delta < 0 ? "上へ" : "下へ"}
                    disabled={!fields[i + delta]}
                    icon={delta < 0 ? <UpOutlined /> : <DownOutlined />}
                    onClick={() =>
                      session.update(
                        source.id,
                        ["fields"],
                        moveItem(fields, i, i + delta),
                      )
                    }
                  />
                ))}
                <Button
                  type="text"
                  size="small"
                  aria-label={`${f.name}を削除`}
                  icon={<DeleteOutlined />}
                  onClick={() => session.removeField(source.id, i)}
                />
              </Space>
            ),
          },
        ]}
      />
    </div>
  );
}

export function MasterReferenceCell({
  candidates,
  ...props
}: ComponentProps<typeof ValueControl> & {
  candidates: ReturnType<typeof masterItems>;
}) {
  const [open, setOpen] = useState(false);
  const linked = isMasterRef(props.value);
  const ref = linked ? props.value.$masterValue : undefined;
  const matches = linked
    ? candidates.filter(
        (x) =>
          x.source.data.table === ref?.definition && sameKey(x.key, ref?.key),
      )
    : [];
  const current = matches.length === 1 ? matches[0] : undefined;
  if (!linked && !candidates.length) return <ValueControl {...props} />;
  const caption = linked
    ? `${ref?.definition} / ${ref?.key}（${current ? text(current.value) : "参照エラー"}）`
    : "マスター値を参照";
  return (
    <div className="master-reference-cell">
      {!linked && <ValueControl {...props} />}
      <Popover
        trigger="click"
        open={open}
        onOpenChange={setOpen}
        content={
          <Space orientation="vertical">
            <Select
              aria-label="参照するマスター値"
              showSearch
              optionFilterProp="label"
              placeholder="キー・プロパティ名で検索"
              style={{ minWidth: 240 }}
              value={undefined}
              options={candidates.map((x, i) => ({
                value: i,
                label: `${x.source.data.table} / ${x.key} · ${x.field.name}`,
              }))}
              onChange={(i: number) => {
                const item = candidates[i];
                props.onChange({
                  $masterValue: {
                    definition: item.source.data.table,
                    key: item.key,
                  },
                });
                setOpen(false);
              }}
            />
            {linked && (
              <Button
                onClick={() => {
                  props.onChange(
                    current
                      ? structuredClone(current.value)
                      : defaultValue(props.snapshot, props.field),
                  );
                  setOpen(false);
                }}
              >
                直接値にする
              </Button>
            )}
          </Space>
        }
      >
        <Button
          type="text"
          size="small"
          icon={<LinkOutlined />}
          aria-label={caption}
          title={caption}
        >
          {linked && caption}
        </Button>
      </Popover>
    </div>
  );
}
