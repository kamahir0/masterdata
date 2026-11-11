import React, { useEffect, useState } from "react";
import { Alert, Button, Input, Modal, Space, Table, Typography } from "antd";
import {
  DeleteOutlined,
  DownOutlined,
  EditOutlined,
  PlusOutlined,
  UpOutlined,
} from "@ant-design/icons";
import { fieldsOf, type Source } from "../core/types";
import { serialize } from "../core/yaml";
import { itemIdentities } from "../core/identity";
import { session } from "../core/session";
import {
  appendDisplayDefinition,
  displayDefinitionKeys,
  displayDefinitionProblems,
  displayDefinitions,
  displayDocumentProblem,
  editDisplayDefinition,
  isDisplayDefinition,
  moveDisplayDefinition,
  newDisplayDefinition,
  removeDisplayDefinition,
  updateDisplayDefinitions,
  updateRawDisplay,
  type DisplayDefinitionKey,
} from "../core/display-definitions";

const titles: Record<DisplayDefinitionKey, string> = {
  functionColumns: "関数列",
  formatRules: "テーブル書式",
};
interface Draft {
  source: Source;
  project: ReturnType<typeof session.getState>["project"];
  key: DisplayDefinitionKey;
  index: number;
  adding?: boolean;
  name: string;
  script: string;
}
interface RawDraft {
  source: Source;
  project: ReturnType<typeof session.getState>["project"];
  text: string;
}

export function DisplayDefinitionEditor({ source }: { source: Source }) {
  const [draft, setDraft] = useState<Draft | null>(null);
  const [rawDraft, setRawDraft] = useState<RawDraft | null>(null);
  const [error, setError] = useState("");
  const path = `${source.path}.view`;
  const documentProblem = displayDocumentProblem(source);
  const current = draft ?? rawDraft;
  // A stale draft remains available for copying/cancellation, but can never be
  // applied over an Undo, raw repair or a replacement document.
  const stale =
    !!current &&
    (current.source.display !== source.display ||
      current.source.id !== source.id ||
      current.project !== session.getState().project);
  useEffect(() => {
    setDraft(null);
    setRawDraft(null);
    setError("");
  }, [source.id]);
  const run = (action: () => void) => {
    try {
      action();
      setError("");
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  };
  const cancel = () => {
    setDraft(null);
    setRawDraft(null);
    setError("");
  };
  const apply = () =>
    run(() => {
      if (stale)
        throw new Error(
          "対象の表示定義が変わりました。入力をコピーするか取り消し、開き直してください。",
        );
      if (rawDraft) {
        updateRawDisplay(session, rawDraft.source, rawDraft.text);
        setRawDraft(null);
      } else if (draft) {
        const items = displayDefinitions(draft.source, draft.key).items;
        const previous = items[draft.index];
        if (
          draft.adding ||
          (isDisplayDefinition(previous) &&
            (previous.name !== draft.name || previous.script !== draft.script))
        )
          updateDisplayDefinitions(
            session,
            draft.source,
            draft.key,
            draft.adding
              ? appendDisplayDefinition(items, {
                  name: draft.name,
                  script: draft.script,
                })
              : editDisplayDefinition(
                  items,
                  draft.index,
                  draft.name,
                  draft.script,
                ),
            `${titles[draft.key]}の定義を編集`,
          );
        setDraft(null);
      }
    });
  const editRaw = () => {
    setError("");
    setRawDraft({
      source,
      project: session.getState().project,
      text: source.display
        ? (session
            .files([source.id])
            .find((file) => file.id === `display:${source.id}`)?.text ??
          serialize(source.display))
        : "{}\n",
    });
  };
  const field = JSON.stringify(fieldsOf(source)[0]?.name ?? "id");
  const example =
    draft?.key === "formatRules"
      ? `return column.name === ${field}\n  ? { bold: true, color: "#D4380D" } : null;`
      : `return { text: String(row[${field}]),\n  style: { bold: true } };`;
  const edit = (key: DisplayDefinitionKey, index: number, adding = false) => {
    const items = displayDefinitions(source, key).items;
    const value = adding ? newDisplayDefinition(items, key) : items[index];
    if (!isDisplayDefinition(value)) return;
    setError("");
    setDraft({
      source,
      project: session.getState().project,
      key,
      index,
      adding,
      name: value.name,
      script: value.script,
    });
  };

  return (
    <section className="display-definitions" aria-label="表示定義">
      <Space wrap>
        <Typography.Text type="secondary">
          表示ファイル: sources/{path}
        </Typography.Text>
        <Button size="small" onClick={editRaw}>
          表示YAMLを編集
        </Button>
      </Space>
      {error && !current && (
        <Alert type="error" title={<span role="alert">{error}</span>} />
      )}
      {documentProblem && (
        <Alert
          type="warning"
          title="表示YAMLを確認してください"
          description={
            <>
              {documentProblem}
              <br />
              <Button size="small" onClick={editRaw}>
                表示YAMLを修正
              </Button>
            </>
          }
        />
      )}
      {displayDefinitionKeys.map((key) => {
        const list = displayDefinitions(source, key);
        const problems = displayDefinitionProblems(source, key);
        const slots = itemIdentities(list.items);
        const rows = list.items.map((value, index) => ({
          value,
          index,
          id: slots[index],
        }));
        const title = titles[key];
        return (
          <section
            key={key}
            className="display-definition-group"
            aria-label={`${title}の定義`}
          >
            <Typography.Title level={4}>{title}</Typography.Title>
            {list.kind === "invalid" ? (
              !documentProblem && (
                <Alert
                  type="warning"
                  title={`${key}は配列ではありません。`}
                  description={
                    <Button size="small" onClick={editRaw}>
                      表示YAMLで修正
                    </Button>
                  }
                />
              )
            ) : (
              <div className="definition-table">
                <Table
                  size="small"
                  pagination={false}
                  rowKey="id"
                  dataSource={rows}
                  scroll={{ x: 580 }}
                  locale={{ emptyText: `${title}はありません` }}
                  columns={[
                    {
                      title: "順序",
                      width: 60,
                      render: (_, row) => row.index + 1,
                    },
                    {
                      title: "名前",
                      render: (_, { value, index }) => (
                        <div>
                          <strong>
                            {isDisplayDefinition(value)
                              ? value.name || "（名前なし）"
                              : "編集できない形式"}
                          </strong>
                          {problems[index].map((problem) => (
                            <div
                              key={problem}
                              className="display-definition-problem"
                            >
                              {problem}
                            </div>
                          ))}
                          {!isDisplayDefinition(value) && (
                            <Button size="small" onClick={editRaw}>
                              表示YAMLで修正
                            </Button>
                          )}
                        </div>
                      ),
                    },
                    {
                      title: "操作",
                      width: 175,
                      render: (_, { value, index }) => (
                        <Space size={0}>
                          <Button
                            type="text"
                            icon={<EditOutlined />}
                            aria-label={`${title}${index + 1}を編集`}
                            disabled={!isDisplayDefinition(value)}
                            onClick={() => edit(key, index)}
                          />
                          <Button
                            type="text"
                            icon={<UpOutlined />}
                            aria-label={`${title}${index + 1}を上へ`}
                            disabled={index === 0}
                            onClick={() =>
                              run(() =>
                                updateDisplayDefinitions(
                                  session,
                                  source,
                                  key,
                                  moveDisplayDefinition(
                                    list.items,
                                    index,
                                    index - 1,
                                  ),
                                  `${title}を並べ替え`,
                                ),
                              )
                            }
                          />
                          <Button
                            type="text"
                            icon={<DownOutlined />}
                            aria-label={`${title}${index + 1}を下へ`}
                            disabled={index === rows.length - 1}
                            onClick={() =>
                              run(() =>
                                updateDisplayDefinitions(
                                  session,
                                  source,
                                  key,
                                  moveDisplayDefinition(
                                    list.items,
                                    index,
                                    index + 1,
                                  ),
                                  `${title}を並べ替え`,
                                ),
                              )
                            }
                          />
                          <Button
                            type="text"
                            danger
                            icon={<DeleteOutlined />}
                            aria-label={`${title}${index + 1}を削除`}
                            onClick={() =>
                              run(() =>
                                updateDisplayDefinitions(
                                  session,
                                  source,
                                  key,
                                  removeDisplayDefinition(list.items, index),
                                  `${title}を削除`,
                                ),
                              )
                            }
                          />
                        </Space>
                      ),
                    },
                  ]}
                />
                <div className="definition-add">
                  <Button
                    type="dashed"
                    icon={<PlusOutlined />}
                    onClick={() => edit(key, rows.length, true)}
                  >
                    {title}を追加
                  </Button>
                </div>
              </div>
            )}
          </section>
        );
      })}
      <Modal
        open={!!current}
        title={
          rawDraft
            ? "表示YAMLの編集"
            : draft
              ? `${titles[draft.key]}の編集`
              : "表示定義の編集"
        }
        onOk={apply}
        onCancel={cancel}
        okText="適用"
        cancelText="取消"
        width={800}
        mask={{ closable: false }}
        destroyOnHidden
      >
        <div
          className="display-definition-form"
          onKeyDown={(event) => {
            if (
              (event.metaKey || event.ctrlKey) &&
              ["z", "y"].includes(event.key.toLowerCase())
            )
              event.stopPropagation();
          }}
        >
          <Typography.Text type="secondary">sources/{path}</Typography.Text>
          {rawDraft ? (
            <Input.TextArea
              aria-label="表示YAML"
              value={rawDraft.text}
              rows={16}
              spellCheck={false}
              className="display-definition-code"
              onChange={(event) =>
                setRawDraft({ ...rawDraft, text: event.target.value })
              }
            />
          ) : (
            draft && (
              <>
                <label>
                  名前
                  <Input
                    aria-label={`${titles[draft.key]}の名前`}
                    value={draft.name}
                    onChange={(event) =>
                      setDraft({ ...draft, name: event.target.value })
                    }
                  />
                </label>
                <label>
                  JavaScript関数の本体
                  <Input.TextArea
                    aria-label={`${titles[draft.key]}のコード`}
                    value={draft.script}
                    rows={10}
                    spellCheck={false}
                    className="display-definition-code"
                    onChange={(event) =>
                      setDraft({ ...draft, script: event.target.value })
                    }
                  />
                </label>
                <Typography.Paragraph type="secondary">
                  <code>row[{field}]</code>: 現在行。<code>table.rows</code>:
                  分割ファイルを含む表の全行。
                  <br />
                  <code>
                    table.group({field}, row[{field}])
                  </code>
                  : 同じ値の行。
                  <br />
                  <code>table.sumBy("poolId", row.poolId, "weight")</code>:
                  グループ合計。整数はBigInt。
                  <br />
                  <code>column.name</code>: 列名。<code>column.kind</code>:
                  "data" / "function"。
                  <br />
                  {draft.key === "functionColumns"
                    ? "戻り値: { text, style? }"
                    : "戻り値: 書式またはnull"}
                  。 書式はbold・italic・color・backgroundColor（色は#RRGGBB）。
                </Typography.Paragraph>
                <pre className="display-definition-code">{example}</pre>
              </>
            )
          )}
          <Typography.Text type="secondary">
            未完成でも適用・保存できます。適用後はUndo /
            Redo、取消でこの入力を破棄します。
          </Typography.Text>
          {stale && (
            <Alert
              type="warning"
              title="対象が変更されました。入力をコピーするか取り消し、開き直してください。"
            />
          )}
          {error && (
            <Alert type="error" title={<span role="alert">{error}</span>} />
          )}
        </div>
      </Modal>
    </section>
  );
}
