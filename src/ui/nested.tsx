import React, { useRef } from "react";
import {
  Alert,
  Breadcrumb,
  Button,
  Drawer,
  Empty,
  Space,
  Tag,
  Typography,
} from "antd";
import {
  CopyOutlined,
  DeleteOutlined,
  HolderOutlined,
  PlusOutlined,
  UpOutlined,
  DownOutlined,
  ArrowLeftOutlined,
} from "@ant-design/icons";
import {
  type Field,
  type Snapshot,
  getIn,
  typeSource,
  fieldsOf,
  isObject,
} from "../core/types";
import { defaultValue } from "../core/schema";
import { itemIdentities, moveItem, spliceItems } from "../core/identity";
import { session } from "../core/session";
import { ValueControl } from "./inputs";
import { usePointerSort } from "./sortable";
import { useEditing } from "./editing";
export interface Cursor {
  source: string;
  path: (string | number)[];
  field: Field;
}
export function NestedEditor({
  cursor,
  snapshot,
  setCursor,
}: {
  cursor: Cursor | null;
  snapshot: Snapshot;
  setCursor: (cursor: Cursor | null) => void;
}) {
  const body = useRef<HTMLDivElement>(null);
  const source = snapshot.sources.find((s) => s.id === cursor?.source);
  const value = cursor && source ? getIn(source.data, cursor.path) : undefined;
  const field = cursor?.field;
  const array = !!field?.array;
  const valid = array ? Array.isArray(value) : isObject(value);
  const update = (next: any) =>
    cursor &&
    session.update(cursor.source, cursor.path, next, "ネストした値を編集");
  const move = (from: number, to: number) => {
    if (
      !Array.isArray(value) ||
      from < 0 ||
      from >= value.length ||
      to < 0 ||
      to >= value.length ||
      from === to
    )
      return;
    update(moveItem(value, from, to));
  };
  const itemKeys = Array.isArray(value) ? itemIdentities(value) : [];
  const editing = useEditing(body, itemKeys, {
    context: JSON.stringify([cursor?.source, cursor?.path]),
    input:
      ".value-control input:not([disabled]), .value-control button:not([disabled]), .value-control .entry",
  });
  const insert = (at: number, item: any) =>
    editing.run(() => {
      const next = spliceItems(value, at, 0, item);
      update(next);
      return { focus: itemIdentities(next)[at] };
    });
  usePointerSort(body, (_, from, to) =>
    move(itemKeys.indexOf(from), itemKeys.indexOf(to)),
  );
  const navigate = (depth: number) => {
    if (!cursor || depth < 2) return;
    let schema = snapshot.sources.find(
      (s) => s.data?.table === source?.data?.table && s.data?.kind === "schema",
    );
    let field: Field | undefined;
    for (let i = 2; i <= depth; i++) {
      const part = cursor.path[i];
      if (typeof part === "string") {
        field = fieldsOf(schema!).find((f) => f.name === part);
        schema = field ? typeSource(snapshot, field.type) : undefined;
      } else if (field) field = { ...field, array: false, nullable: false };
    }
    if (field)
      setCursor({ ...cursor, path: cursor.path.slice(0, depth + 1), field });
  };
  return (
    <Drawer
      title={
        <div className="nested-title">
          <Button
            type="text"
            size="small"
            icon={<ArrowLeftOutlined />}
            aria-label="一つ上の値へ戻る"
            disabled={!cursor || cursor.path.length <= 3}
            onClick={() => cursor && navigate(cursor.path.length - 2)}
          />
          <Tag color="blue">{array ? "[ ] 配列" : field?.type}</Tag>
          <Typography.Text>値を編集</Typography.Text>
        </div>
      }
      placement="right"
      width={470}
      open={!!cursor && !!source}
      onClose={() => setCursor(null)}
      mask={false}
      getContainer={false}
      rootStyle={{ position: "absolute" }}
      styles={{
        body: {
          padding: 0,
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        },
      }}
    >
      {cursor && field && source && (
        <React.Fragment key={JSON.stringify([source.id, ...cursor.path])}>
          <div className="breadcrumb-scroll">
            <Breadcrumb
              items={cursor.path.slice(2).map((part, i) => ({
                title: (
                  <button
                    aria-current={
                      i === cursor.path.length - 3 ? "page" : undefined
                    }
                    onClick={() => navigate(i + 2)}
                  >
                    {typeof part === "number" ? `[${part}]` : part}
                  </button>
                ),
              }))}
            />
          </div>
          {!valid ? (
            <div className="nested-repair">
              <Alert
                type="warning"
                showIcon
                title={
                  array
                    ? "この値は配列の形式になっていません"
                    : "この値は構造体の形式になっていません"
                }
                description="元の値を履歴に残して初期値を作成できます。取り消しで元に戻せます。"
              />
              <Button
                type="primary"
                icon={<PlusOutlined />}
                onClick={() =>
                  update(defaultValue(snapshot, { ...field, nullable: false }))
                }
              >
                初期値で修復
              </Button>
              <pre>{String(value)}</pre>
            </div>
          ) : (
            <>
              <div className="nested-body" ref={body}>
                {array ? (
                  <>
                    {(value as any[]).map((item, index) => (
                      <div
                        className="array-item"
                        key={itemKeys[index]}
                        data-motion-key={itemKeys[index]}
                        data-sort-group="array"
                        data-sort-key={itemKeys[index]}
                        data-sort-anchor=""
                      >
                        <span
                          className="drag-handle"
                          data-sort-handle="array"
                          aria-label={`要素${index + 1}を移動`}
                        >
                          <HolderOutlined />
                        </span>
                        <span className="array-index">{index}</span>
                        <ValueControl
                          snapshot={snapshot}
                          field={{ ...field, array: false, nullable: false }}
                          value={item}
                          label={`要素${index + 1}`}
                          onChange={(v) =>
                            session.update(
                              source.id,
                              [...cursor.path, index],
                              v,
                            )
                          }
                          onOpen={() =>
                            setCursor({
                              source: source.id,
                              path: [...cursor.path, index],
                              field: {
                                ...field,
                                array: false,
                                nullable: false,
                              },
                            })
                          }
                        />
                        <div className="array-actions">
                          <Button
                            size="small"
                            type="text"
                            icon={<UpOutlined />}
                            aria-label="要素を上へ"
                            disabled={index === 0}
                            onClick={() => move(index, index - 1)}
                          />
                          <Button
                            size="small"
                            type="text"
                            icon={<DownOutlined />}
                            aria-label="要素を下へ"
                            disabled={index === value.length - 1}
                            onClick={() => move(index, index + 1)}
                          />
                          <Button
                            size="small"
                            type="text"
                            icon={<CopyOutlined />}
                            aria-label="要素を複製"
                            onClick={() =>
                              insert(index + 1, structuredClone(item))
                            }
                          />
                          <Button
                            size="small"
                            type="text"
                            danger
                            icon={<DeleteOutlined />}
                            aria-label="要素を削除"
                            onClick={() =>
                              editing.run(() => {
                                update(spliceItems(value, index, 1));
                              }, [itemKeys[index]])
                            }
                          />
                        </div>
                      </div>
                    ))}
                  </>
                ) : (
                  fieldsOf(typeSource(snapshot, field.type)!).map((child) => (
                    <div className="nested-field" key={child.name}>
                      <div className="nested-label">
                        <strong>{child.name}</strong>
                        <span>
                          {child.type}
                          {child.array ? " [ ]" : ""}
                          {child.nullable ? " ?" : ""}
                        </span>
                      </div>
                      <ValueControl
                        snapshot={snapshot}
                        field={child}
                        value={value[child.name]}
                        label={child.name}
                        onChange={(v) =>
                          session.update(
                            source.id,
                            [...cursor.path, child.name],
                            v,
                          )
                        }
                        onOpen={() =>
                          setCursor({
                            source: source.id,
                            path: [...cursor.path, child.name],
                            field: child,
                          })
                        }
                      />
                    </div>
                  ))
                )}
                {array && !value.length && (
                  <Empty
                    image={Empty.PRESENTED_IMAGE_SIMPLE}
                    description="要素はありません"
                  />
                )}
              </div>
              {array && (
                <div className="nested-footer">
                  <Button
                    type="dashed"
                    block
                    icon={<PlusOutlined />}
                    onClick={() =>
                      insert(
                        value.length,
                        defaultValue(snapshot, {
                          ...field,
                          array: false,
                          nullable: false,
                        }),
                      )
                    }
                  >
                    要素を追加
                  </Button>
                </div>
              )}
            </>
          )}
        </React.Fragment>
      )}
    </Drawer>
  );
}
