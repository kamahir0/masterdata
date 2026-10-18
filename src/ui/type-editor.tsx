import React, { useRef } from "react";
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
import { identity } from "../core/identity";
import { useEditing } from "./editing";
import { usePointerSort } from "./sortable";
import { sourceKind } from "./source-icon";
import { FieldEditor } from "./fields";
export function TypeEditor({
  source,
  snapshot,
}: {
  source: Source;
  snapshot: Snapshot;
}) {
  const kind = typeKind(source);
  const definition = kind ? source.data[kind] : {};
  const members = Array.isArray(definition.members) ? definition.members : [];
  const locked = (member: any) => kind === "flags" && member.name === "None";
  const minimum = members.some(locked) ? 1 : 0;
  const setMembers = (next: any[], label = "候補を編集") =>
    session.update(source.id, [kind!, "members"], next, label);
  const add = () => {
    let index = 1;
    while (members.some((m: any) => m.name === `Member${index}`)) index++;
    let value = kind === "flags" ? 1n : 0n;
    while (members.some((m: any) => String(m.value) === String(value)))
      value = kind === "flags" ? value * 2n : value + 1n;
    const member = { name: `Member${index}`, value };
    editing.run(() => {
      setMembers([...members, member], "候補を追加");
      return { focus: identity(member) };
    });
  };
  const move = (from: number, to: number) => {
    if (
      from < 0 ||
      to < minimum ||
      to >= members.length ||
      from === to ||
      locked(members[from])
    )
      return;
    const next = [...members];
    next.splice(to, 0, next.splice(from, 1)[0]);
    setMembers(next, "候補を並べ替え");
  };
  const root = useRef<HTMLDivElement>(null);
  const editing = useEditing(root, members.map(identity), {
    context: source.id,
  });
  usePointerSort(root, (_, from, to) =>
    move(
      members.findIndex((m: any) => identity(m) === from),
      members.findIndex((m: any) => identity(m) === to),
    ),
  );
  if (!kind) return null;
  return (
    <div className="type-editor editor-scroll" key={source.id} ref={root}>
      <div className="section-intro">
        <div>
          <Tag
            className="source-kind-label"
            data-source-kind={sourceKind(source)}
          >
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
              <Table<any>
                size="small"
                pagination={false}
                dataSource={members}
                rowKey={identity}
                onRow={(member) =>
                  locked(member)
                    ? {}
                    : ({
                        "data-sort-group": "members",
                        "data-sort-key": identity(member),
                        "data-sort-anchor": "",
                      } as any)
                }
                columns={[
                  {
                    title: "",
                    width: 40,
                    render: (_, member) => (
                      <span
                        className="drag-handle"
                        data-sort-handle="members"
                        aria-disabled={locked(member)}
                        aria-label="候補を移動"
                      >
                        <HolderOutlined />
                      </span>
                    ),
                  },
                  {
                    title: "名前",
                    render: (_, member, i) => (
                      <Entry
                        disabled={locked(member)}
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
                        disabled={locked(member)}
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
                    render: (_, member, i) => (
                      <Dropdown
                        menu={{
                          items: [
                            {
                              key: "up",
                              label: "上へ",
                              icon: <UpOutlined />,
                              disabled: locked(member) || i <= minimum,
                            },
                            {
                              key: "down",
                              label: "下へ",
                              icon: <DownOutlined />,
                              disabled:
                                locked(member) || i === members.length - 1,
                            },
                            {
                              key: "delete",
                              label: "削除",
                              icon: <DeleteOutlined />,
                              danger: true,
                              disabled: locked(member),
                            },
                          ],
                          onClick: ({ key }) =>
                            key === "delete"
                              ? editing.run(
                                  () =>
                                    setMembers(
                                      members.filter(
                                        (_: any, j: number) => i !== j,
                                      ),
                                      "候補を削除",
                                    ),
                                  [identity(member)],
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
