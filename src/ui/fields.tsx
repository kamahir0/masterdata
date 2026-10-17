import React, { useState } from "react";
import { Button, Dropdown, Space, Table, Tooltip } from "antd";
import {
  DeleteOutlined,
  HolderOutlined,
  PlusOutlined,
  UpOutlined,
  DownOutlined,
  MoreOutlined,
} from "@ant-design/icons";
import {
  type Source,
  type Snapshot,
  type Field,
  fieldsOf,
} from "../core/types";
import { session } from "../core/session";
import { Entry, TypeSelect } from "./inputs";
export function ModifierButtons({
  field,
  onChange,
}: {
  field: Field;
  onChange: (field: Field) => void;
}) {
  return (
    <Space size={3} className="modifiers">
      <Tooltip title={field.array ? "配列を解除" : "配列にする"}>
        <Button
          size="small"
          type={field.array ? "primary" : "text"}
          aria-label="配列"
          aria-pressed={!!field.array}
          onClick={() => onChange({ ...field, array: !field.array })}
        >
          [ ]
        </Button>
      </Tooltip>
      <Tooltip title={field.nullable ? "nullを許可しない" : "nullを許可する"}>
        <Button
          size="small"
          type={field.nullable ? "primary" : "text"}
          aria-label="nullを許可"
          aria-pressed={!!field.nullable}
          onClick={() => onChange({ ...field, nullable: !field.nullable })}
        >
          ?
        </Button>
      </Tooltip>
    </Space>
  );
}
export function FieldEditor({
  source,
  snapshot,
}: {
  source: Source;
  snapshot: Snapshot;
}) {
  const [focusIndex, setFocusIndex] = useState(-1);
  const fields = fieldsOf(source);
  const base = source.data.custom ? ["custom", "fields"] : ["fields"];
  const set = (i: number, f: Field) =>
    session.update(source.id, [...base, i], f, "項目の定義を編集");
  const move = (from: number, to: number) => {
    if (to < 0 || to >= fields.length || from === to) return;
    const next = [...fields];
    next.splice(to, 0, next.splice(from, 1)[0]);
    session.update(source.id, base, next, "項目を並べ替え");
  };
  return (
    <div className="definition-table">
      <Table
        size="small"
        pagination={false}
        rowKey={(_, i) => String(i)}
        dataSource={fields}
        scroll={{ x: 620 }}
        onRow={(_, index) => ({
          onDragOver: (e) => {
            e.preventDefault();
            e.dataTransfer.dropEffect = "move";
          },
          onDrop: (e) => {
            e.preventDefault();
            const from = Number(e.dataTransfer.getData("field-index"));
            if (Number.isInteger(from)) move(from, index!);
          },
        })}
        columns={[
          {
            title: "",
            width: 38,
            render: (_, _f, i) => (
              <span
                className="drag-handle"
                draggable
                aria-label="項目を移動"
                onDragStart={(e) =>
                  e.dataTransfer.setData("field-index", String(i))
                }
              >
                <HolderOutlined />
              </span>
            ),
          },
          {
            title: "名前",
            dataIndex: "name",
            width: 210,
            render: (name, _, i) => (
              <Entry
                autoFocus={i === focusIndex}
                value={name}
                label={`項目${i + 1}の名前`}
                onCommit={(v) => session.renameField(source.id, i, v)}
              />
            ),
          },
          {
            title: "型",
            width: 210,
            render: (_, f, i) => (
              <TypeSelect
                snapshot={snapshot}
                value={f.type}
                onChange={(v) => set(i, { ...f, type: v })}
              />
            ),
          },
          {
            title: "形",
            width: 100,
            render: (_, f, i) => (
              <ModifierButtons field={f} onChange={(v) => set(i, v)} />
            ),
          },
          {
            title: "番号",
            width: 80,
            render: (_, f, i) => (
              <Entry
                value={f.key ?? i}
                label={`項目${i + 1}の番号`}
                numeric
                onCommit={(v) => set(i, { ...f, key: Number(v) })}
              />
            ),
          },
          {
            title: "",
            width: 48,
            render: (_, _f, i) => (
              <Dropdown
                menu={{
                  items: [
                    {
                      key: "up",
                      label: "上へ",
                      icon: <UpOutlined />,
                      disabled: i === 0,
                    },
                    {
                      key: "down",
                      label: "下へ",
                      icon: <DownOutlined />,
                      disabled: i === fields.length - 1,
                    },
                    {
                      key: "delete",
                      label: "削除",
                      icon: <DeleteOutlined />,
                      danger: true,
                    },
                  ],
                  onClick: ({ key }) =>
                    key === "delete"
                      ? session.removeField(source.id, i)
                      : move(i, key === "up" ? i - 1 : i + 1),
                }}
              >
                <Button
                  type="text"
                  aria-label={`項目${i + 1}の操作`}
                  icon={<MoreOutlined />}
                />
              </Dropdown>
            ),
          },
        ]}
      />
      <div className="definition-add">
        <Button
          type="dashed"
          icon={<PlusOutlined />}
          onClick={() => {
            setFocusIndex(fields.length);
            session.addField(source.id);
          }}
        >
          フィールドを追加
        </Button>
      </div>
    </div>
  );
}
