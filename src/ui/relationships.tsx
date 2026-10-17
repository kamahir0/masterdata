import React from "react";
import { Button, Card, Select, Space, Switch, Typography } from "antd";
import { DeleteOutlined, PlusOutlined } from "@ant-design/icons";
import { type Snapshot, type Source, fieldsOf, tables } from "../core/types";
import { session } from "../core/session";
import { Entry } from "./inputs";
export function Relationships({
  source,
  snapshot,
}: {
  source: Source;
  snapshot: Snapshot;
}) {
  const fields = fieldsOf(source).map((f) => ({
    value: f.name,
    label: `${f.name} · ${f.type}`,
  }));
  const references: any[] = Array.isArray(source.data.references)
    ? source.data.references
    : [];
  const secondary: any[] = Array.isArray(source.data.secondaryKeys)
    ? source.data.secondaryKeys
    : [];
  const setRefs = (next: any[]) =>
    session.update(source.id, ["references"], next, "参照を編集");
  return (
    <div className="relationships editor-scroll">
      <Card title="主キー" size="small">
        <Typography.Paragraph type="secondary">
          レコードを識別する、重複しない項目の組み合わせです。
        </Typography.Paragraph>
        <Select
          aria-label="主キーの項目"
          mode="multiple"
          showSearch
          optionFilterProp="label"
          style={{ width: "100%" }}
          value={source.data.primaryKey?.fields ?? []}
          options={fields}
          onChange={(v) =>
            session.update(
              source.id,
              ["primaryKey"],
              { fields: v },
              "主キーを編集",
            )
          }
        />
      </Card>
      <Card
        title="副キー"
        size="small"
        extra={
          <Button
            size="small"
            type="text"
            icon={<PlusOutlined />}
            onClick={() =>
              session.update(
                source.id,
                ["secondaryKeys"],
                [...secondary, { fields: [], nonUnique: true }],
                "副キーを追加",
              )
            }
          >
            追加
          </Button>
        }
      >
        {secondary.map((key, i) => (
          <div className="relationship-row" key={i}>
            <Select
              aria-label={`副キー${i + 1}の項目`}
              mode="multiple"
              showSearch
              optionFilterProp="label"
              value={key.fields}
              options={fields}
              onChange={(fields) =>
                session.update(
                  source.id,
                  ["secondaryKeys", i, "fields"],
                  fields,
                )
              }
            />
            <Space>
              <Switch
                aria-label={`副キー${i + 1}の重複を許可`}
                size="small"
                checked={!!key.nonUnique}
                onChange={(v) =>
                  session.update(
                    source.id,
                    ["secondaryKeys", i, "nonUnique"],
                    v,
                  )
                }
              />
              <Typography.Text>重複可</Typography.Text>
            </Space>
            <Button
              type="text"
              danger
              icon={<DeleteOutlined />}
              aria-label={`副キー${i + 1}を削除`}
              onClick={() =>
                session.update(
                  source.id,
                  ["secondaryKeys"],
                  secondary.filter((_, j) => i !== j),
                )
              }
            />
          </div>
        ))}
        {!secondary.length && (
          <Typography.Text type="secondary">
            検索に使う項目を必要に応じて追加できます。
          </Typography.Text>
        )}
      </Card>
      <Card
        title="別の表への参照"
        size="small"
        extra={
          <Button
            type="text"
            size="small"
            icon={<PlusOutlined />}
            onClick={() =>
              setRefs([
                ...references,
                {
                  name: `reference${references.length + 1}`,
                  fields: [],
                  target: { table: "", fields: [] },
                },
              ])
            }
          >
            追加
          </Button>
        }
      >
        {references.map((ref, i) => (
          <div className="reference-definition" key={i}>
            <Space.Compact>
              <Entry
                value={ref.name}
                label={`参照${i + 1}の名前`}
                onCommit={(v) =>
                  session.update(source.id, ["references", i, "name"], v)
                }
              />
              <Button
                danger
                type="text"
                icon={<DeleteOutlined />}
                aria-label={`参照${i + 1}を削除`}
                onClick={() => setRefs(references.filter((_, j) => i !== j))}
              />
            </Space.Compact>
            <div className="reference-definition-fields">
              <Select
                aria-label={`参照${i + 1}の元項目`}
                mode="multiple"
                placeholder="この表の項目"
                options={fields}
                value={ref.fields}
                onChange={(v) =>
                  session.update(source.id, ["references", i, "fields"], v)
                }
              />
              <span>→</span>
              <Select
                aria-label={`参照${i + 1}の表`}
                showSearch
                placeholder="参照先の表"
                options={tables(snapshot).map((t) => ({
                  value: t.name,
                  label: t.name,
                }))}
                value={ref.target.table || undefined}
                onChange={(v) =>
                  session.update(source.id, ["references", i, "target"], {
                    table: v,
                    fields: [],
                  })
                }
              />
              <Select
                aria-label={`参照${i + 1}の先項目`}
                mode="multiple"
                placeholder="参照先のキー項目"
                options={(
                  tables(snapshot).find((t) => t.name === ref.target.table)
                    ?.fields ?? []
                ).map((f) => ({ value: f.name, label: f.name }))}
                value={ref.target.fields}
                onChange={(v) =>
                  session.update(
                    source.id,
                    ["references", i, "target", "fields"],
                    v,
                  )
                }
              />
            </div>
          </div>
        ))}
        {!references.length && (
          <Typography.Text type="secondary">
            クエストの報酬アイテムなど、表同士のつながりを定義できます。
          </Typography.Text>
        )}
      </Card>
      <Card title="コード生成" size="small">
        <span>C#の型名</span>
        <Entry
          value={source.data.csharpName ?? ""}
          label="C#の型名"
          placeholder="表名から自動生成"
          onCommit={(v) => session.update(source.id, ["csharpName"], v)}
        />
      </Card>
    </div>
  );
}
