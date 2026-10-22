import React, { useRef } from "react";
import { identity } from "../core/identity";
import { DefinitionItem } from "./definition-item";
import { useEditing } from "./editing";
import { SearchSelect } from "./search-select";
import { Button, Card, Select, Space, Switch, Typography } from "antd";
import { PlusOutlined } from "@ant-design/icons";
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
  const refRoot = useRef<HTMLDivElement>(null),
    keyRoot = useRef<HTMLDivElement>(null);
  const refEditing = useEditing(refRoot, references.map(identity), {
    context: source.id,
  });
  const keyEditing = useEditing(keyRoot, secondary.map(identity), {
    context: source.id,
  });
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
            onClick={() => {
              const key = { fields: [], nonUnique: true };
              keyEditing.run(() => {
                session.update(
                  source.id,
                  ["secondaryKeys"],
                  [...secondary, key],
                  "副キーを追加",
                );
                return { focus: identity(key) };
              });
            }}
          >
            追加
          </Button>
        }
      >
        <div className="definition-list" ref={keyRoot}>
          {secondary.map((key, i) => (
            <DefinitionItem
              key={identity(key)}
              identity={identity(key)}
              name={<strong>副キー {i + 1}</strong>}
              removeLabel={`副キー${i + 1}を削除`}
              onRemove={() =>
                keyEditing.run(
                  () =>
                    session.update(
                      source.id,
                      ["secondaryKeys"],
                      secondary.filter((_, j) => i !== j),
                    ),
                  [identity(key)],
                )
              }
            >
              <div className="relationship-row">
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
              </div>
            </DefinitionItem>
          ))}
        </div>
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
            onClick={() => {
              const ref = {
                name: `reference${references.length + 1}`,
                fields: [],
                target: { table: "", fields: [] },
              };
              refEditing.run(() => {
                setRefs([...references, ref]);
                return { focus: identity(ref) };
              });
            }}
          >
            追加
          </Button>
        }
      >
        <div className="definition-list" ref={refRoot}>
          {references.map((ref, i) => (
            <DefinitionItem
              key={identity(ref)}
              identity={identity(ref)}
              removeLabel={`参照${i + 1}を削除`}
              onRemove={() =>
                refEditing.run(
                  () => setRefs(references.filter((_, j) => i !== j)),
                  [identity(ref)],
                )
              }
              name={
                <Entry
                  value={ref.name}
                  label={`参照${i + 1}の名前`}
                  onCommit={(v) =>
                    session.update(source.id, ["references", i, "name"], v)
                  }
                />
              }
            >
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
                <SearchSelect
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
            </DefinitionItem>
          ))}
        </div>
        {!references.length && (
          <Typography.Text type="secondary">
            表同士の外部キー参照を定義して、関連するレコードを連携できます。
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
