import React, { useState } from "react";
import {
  Button,
  Card,
  Dropdown,
  Select,
  Space,
  Table,
  Tag,
  Typography,
} from "antd";
import {
  DeleteOutlined,
  HolderOutlined,
  MoreOutlined,
  PlusOutlined,
  UpOutlined,
  DownOutlined,
} from "@ant-design/icons";
import { type Source, type Snapshot, typeKind } from "../core/types";
import { session } from "../core/session";
import { Entry, parseInput } from "./inputs";
import { FieldEditor } from "./fields";
export function TypeEditor({
  source,
  snapshot,
}: {
  source: Source;
  snapshot: Snapshot;
}) {
  const [focusIndex, setFocusIndex] = useState(-1);
  const kind = typeKind(source);
  if (!kind) return null;
  const definition = source.data[kind];
  const members = Array.isArray(definition.members) ? definition.members : [];
  const setMembers = (next: any[], label = "候補を編集") =>
    session.update(source.id, [kind, "members"], next, label);
  const add = () => {
    setFocusIndex(members.length);
    let index = 1;
    while (members.some((m: any) => m.name === `Member${index}`)) index++;
    let value = kind === "flags" ? 1n : 0n;
    while (members.some((m: any) => String(m.value) === String(value)))
      value = kind === "flags" ? value * 2n : value + 1n;
    setMembers([...members, { name: `Member${index}`, value }], "候補を追加");
  };
  const move = (from: number, to: number) => {
    if (to < 0 || to >= members.length || from === to) return;
    const next = [...members];
    next.splice(to, 0, next.splice(from, 1)[0]);
    setMembers(next, "候補を並べ替え");
  };
  return (
    <div className="type-editor editor-scroll" key={source.id}>
      <div className="section-intro">
        <div>
          <Tag color="purple">
            {kind === "custom"
              ? "Custom Type"
              : kind === "valueObject"
                ? "Value Object"
                : kind === "flags"
                  ? "Flags"
                  : "Enum"}
          </Tag>
          <Typography.Title level={3}>型を定義する</Typography.Title>
          <Typography.Text type="secondary">
            項目を追加して、その場で名前や型を編集できます。
          </Typography.Text>
        </div>
      </div>
      {kind === "custom" ? (
        <FieldEditor source={source} snapshot={snapshot} />
      ) : (
        <>
          <div className="underlying">
            <span>基になる型</span>
            <Select
              aria-label="基になる型"
              value={definition.underlying}
              options={(kind === "valueObject"
                ? ["int", "uint", "long", "ulong", "string"]
                : ["int", "uint", "long", "ulong"]
              ).map((value) => ({ value, label: value }))}
              onChange={(v) =>
                session.update(source.id, [kind, "underlying"], v)
              }
            />
          </div>
          {kind === "valueObject" ? (
            <Card className="value-object-info">
              <Typography.Paragraph>
                同じ数値や文字列でも、用途の異なるIDを別の型として扱えます。
              </Typography.Paragraph>
              <Typography.Text type="secondary">
                例: ItemId と QuestId を取り違えないための型です。
              </Typography.Text>
            </Card>
          ) : (
            <div className="definition-table">
              <Table
                size="small"
                pagination={false}
                dataSource={members}
                rowKey={(_, i) => String(i)}
                onRow={(_, index) => ({
                  onDragOver: (e) => e.preventDefault(),
                  onDrop: (e) => {
                    e.preventDefault();
                    if (e.dataTransfer.types.includes("member-index"))
                      move(
                        Number(e.dataTransfer.getData("member-index")),
                        index!,
                      );
                  },
                })}
                columns={[
                  {
                    title: "",
                    width: 40,
                    render: (_, _r, i) => (
                      <span
                        className="drag-handle"
                        draggable
                        aria-label="候補を移動"
                        onDragStart={(e) =>
                          e.dataTransfer.setData("member-index", String(i))
                        }
                      >
                        <HolderOutlined />
                      </span>
                    ),
                  },
                  {
                    title: "名前",
                    render: (_, member, i) => (
                      <Entry
                        autoFocus={i === focusIndex}
                        value={member.name}
                        label={`候補${i + 1}の名前`}
                        onCommit={(v) => session.renameMember(source.id, i, v)}
                      />
                    ),
                  },
                  {
                    title: "値",
                    width: 220,
                    render: (_, member, i) => (
                      <Entry
                        value={member.value}
                        label={`候補${i + 1}の値`}
                        numeric
                        onCommit={(v) =>
                          session.update(
                            source.id,
                            [kind, "members", i, "value"],
                            parseInput(v, definition.underlying),
                          )
                        }
                      />
                    ),
                  },
                  {
                    title: "",
                    width: 50,
                    render: (_, _m, i) => (
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
                              disabled: i === members.length - 1,
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
                              ? setMembers(
                                  members.filter(
                                    (_: any, j: number) => i !== j,
                                  ),
                                  "候補を削除",
                                )
                              : move(i, key === "up" ? i - 1 : i + 1),
                        }}
                      >
                        <Button
                          type="text"
                          aria-label={`候補${i + 1}の操作`}
                          icon={<MoreOutlined />}
                        />
                      </Dropdown>
                    ),
                  },
                ]}
              />
              <div className="definition-add">
                <Button type="dashed" icon={<PlusOutlined />} onClick={add}>
                  候補を追加
                </Button>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
