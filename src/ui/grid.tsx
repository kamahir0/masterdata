import React, {
  useEffect,
  useMemo,
  useRef,
  useState,
  useDeferredValue,
} from "react";
import {
  Button,
  Dropdown,
  Empty,
  Input,
  Select,
  Space,
  Table,
  Tag,
  Tooltip,
  Typography,
} from "antd";
import type { ColumnsType } from "antd/es/table";
import {
  CopyOutlined,
  DeleteOutlined,
  DownOutlined,
  HolderOutlined,
  MoreOutlined,
  PlusOutlined,
  SearchOutlined,
  TableOutlined,
  UpOutlined,
} from "@ant-design/icons";
import {
  type Snapshot,
  type Source,
  type Field,
  type Problem,
  type TableInfo,
  tables,
  fieldsOf,
  text,
  getIn,
  setIn,
  selected,
  isObject,
  list,
  searchText,
} from "../core/types";
import { identity } from "../core/identity";
import { useLayoutMotion } from "./motion";
import { session } from "../core/session";
import { defaultValue, recordAllocator } from "../core/schema";
import {
  writeTsv,
  readTsv,
  cellText,
  parseCell,
  encodeCells,
  decodeCells,
} from "../core/clipboard";
import {
  Entry,
  TypeSelect,
  ValueControl,
  parseInput,
  type ReferenceOption,
} from "./inputs";
import { ModifierButtons } from "./fields";
import type { Cursor } from "./nested";
interface Row {
  key: string;
  source: Source;
  index: number;
  record: any;
}
export function GridEditor({
  source,
  snapshot,
  problems,
  onOpen,
  navigate,
  profile,
  findRef,
  jump,
}: {
  source: Source;
  snapshot: Snapshot;
  problems: Problem[];
  onOpen: (cursor: Cursor) => void;
  navigate: (source: string) => void;
  profile: string;
  findRef: React.RefObject<any>;
  jump?: { source: string; path: (string | number)[]; nonce: number };
}) {
  const table =
    tables(snapshot).find((t) => t.source.id === source.id) ??
    tables(snapshot).find((t) => t.name === source.data?.table);
  const [query, setQuery] = useState("");
  const deferred = useDeferredValue(query);
  const [chosen, setChosen] = useState("all");
  const [destination, setDestination] = useState(source.id);
  const [rowKeys, setRowKeys] = useState<React.Key[]>([]);
  const [cell, setCell] = useState<{
    row: number;
    col: number;
    anchorRow: number;
    anchorCol: number;
  } | null>(null);
  const [height, setHeight] = useState(400);
  const pendingColumn = useRef<number | null>(null);
  const handledJump = useRef<number | undefined>(undefined);
  const viewport = useRef<HTMLDivElement>(null);
  const tableRef = useRef<any>(null);
  useEffect(() => {
    const observer = new ResizeObserver((entries) =>
      setHeight(Math.max(150, entries[0].contentRect.height - 86)),
    );
    if (viewport.current) observer.observe(viewport.current);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    setChosen(source.data?.kind === "data" ? source.id : "all");
    setDestination(source.id);
    setRowKeys([]);
    setCell(null);
    setQuery("");
  }, [source.id]);
  const rows = useMemo(
    () =>
      !table
        ? []
        : table.sources
            .flatMap((s) =>
              Array.isArray(s.data?.records) &&
              (chosen === "all" || chosen === s.id)
                ? s.data.records.map((record: any, index: number) => ({
                    key: `${s.id}:${identity(record)}`,
                    source: s,
                    index,
                    record,
                  }))
                : [],
            )
            .filter(
              (row) =>
                !deferred ||
                searchText(row.record)
                  .toLowerCase()
                  .includes(deferred.toLowerCase()),
            ),
    [snapshot.sources, source.id, chosen, deferred],
  );
  // Value edits preserve row/field identity and must not replay structural motion.
  useLayoutMotion(
    viewport,
    rows.map((row) => row.key).join("|") +
      ":" +
      (table?.fields.map(identity).join("|") ?? ""),
  );
  useEffect(() => {
    // A diagnostic jump is a command, not a constraint on every later search.
    if (!jump || handledJump.current === jump.nonce) return;
    const i = rows.findIndex(
      (r) => r.source.id === jump.source && r.index === jump.path[1],
    );
    if (i < 0) {
      if (!deferred && chosen === "all") {
        handledJump.current = jump.nonce;
        return;
      }
      setQuery("");
      setChosen("all");
      return;
    }
    const col = Math.max(
      0,
      table?.fields.findIndex((f) => f.name === jump.path[2]) ?? 0,
    );
    setCell({ row: i, col, anchorRow: i, anchorCol: col });
    tableRef.current?.scrollTo({ index: i });
    handledJump.current = jump.nonce;
  }, [jump?.nonce, rows.length, chosen, deferred]);
  useEffect(() => {
    if (pendingColumn.current === null) return;
    const input = viewport.current?.querySelector<HTMLInputElement>(
      `[aria-label="列${pendingColumn.current + 1}の名前"]`,
    );
    if (input) {
      input.focus();
      input.select();
      pendingColumn.current = null;
    }
  }, [table?.fields.length]);
  if (!table)
    return (
      <Empty description="対応する表の定義がありません。ソースから表名を修正できます。" />
    );
  const fail = (error: unknown) =>
    window.dispatchEvent(
      new CustomEvent("masterdata:error", { detail: error }),
    );
  const safe = (action: () => void) => {
    try {
      action();
    } catch (e) {
      fail(e);
    }
  };
  const update = (row: Row, field: Field, value: any) =>
    session.update(row.source.id, ["records", row.index, field.name], value);
  const removeRows = () => {
    const targets = new Set(rowKeys);
    session.change("レコードを削除", (s) => ({
      ...s,
      sources: s.sources.map((f) =>
        Array.isArray(f.data?.records) &&
        f.data.records.some((r: any) => targets.has(`${f.id}:${identity(r)}`))
          ? {
              ...f,
              data: {
                ...f.data,
                records: f.data.records.filter(
                  (_: any, i: number) =>
                    !targets.has(`${f.id}:${identity(f.data.records[i])}`),
                ),
              },
            }
          : f,
      ),
    }));
    setRowKeys([]);
  };
  const move = (row: Row, target: Row) => {
    if (row.key === target.key) return;
    session.change("レコードを移動", (s) => {
      if (row.source.id === target.source.id) {
        const records = [...row.source.data.records];
        records.splice(target.index, 0, records.splice(row.index, 1)[0]);
        return {
          ...s,
          sources: s.sources.map((f) =>
            f.id === row.source.id ? { ...f, data: { ...f.data, records } } : f,
          ),
        };
      }
      return {
        ...s,
        sources: s.sources.map((f) =>
          f.id === row.source.id
            ? {
                ...f,
                data: {
                  ...f.data,
                  records: f.data.records.filter(
                    (_: any, i: number) => i !== row.index,
                  ),
                },
              }
            : f.id === target.source.id
              ? {
                  ...f,
                  data: {
                    ...f.data,
                    records: [
                      ...f.data.records.slice(0, target.index),
                      row.record,
                      ...f.data.records.slice(target.index),
                    ],
                  },
                }
              : f,
        ),
      };
    });
  };
  const columnMove = (from: number, to: number) => {
    if (
      from === to ||
      from < 0 ||
      to < 0 ||
      from >= table.fields.length ||
      to >= table.fields.length
    )
      return;
    const next = [...table.fields];
    next.splice(to, 0, next.splice(from, 1)[0]);
    session.update(table.source.id, ["fields"], next, "列を並べ替え");
  };
  const knownTags = [
    ...new Set(
      snapshot.sources.flatMap((s) =>
        Array.isArray(s.data?.records)
          ? s.data.records.flatMap((r: any) => r?.$tags ?? [])
          : [],
      ),
    ),
  ].map((value) => ({ value: String(value), label: String(value) }));
  const columns: ColumnsType<Row> = [
    {
      key: "grip",
      title: "",
      width: 38,
      fixed: "left",
      render: (_, row) => (
        <span
          className="drag-handle"
          draggable
          aria-label="レコードを移動"
          onDragStart={(e) => {
            e.dataTransfer.setData("row-key", row.key);
            e.dataTransfer.effectAllowed = "move";
          }}
        >
          <HolderOutlined />
        </span>
      ),
    },
    {
      key: "number",
      title: "#",
      width: 48,
      fixed: "left",
      render: (_, _row, i) => <span className="row-number">{i + 1}</span>,
    },
    ...table.fields.map((field, col) => {
      const pk = table.source.data.primaryKey?.fields?.includes(field.name);
      const reference = list(table.source.data.references).find(
        (r: any) =>
          r.fields?.length === 1 &&
          r.fields[0] === field.name &&
          r.target?.fields?.length === 1,
      );
      const target =
        reference &&
        tables(snapshot).find((t) => t.name === reference.target.table);
      const referenceOptions: ReferenceOption[] | undefined =
        target &&
        target.sources.flatMap((s: Source) =>
          (s.data.records ?? []).map((r: any, i: number) => ({
            value: `${s.id}:${i}`,
            label: `${r.name ?? r.title ?? text(r[reference.target.fields[0]])} · ${text(r[reference.target.fields[0]])}`,
            actual: r[reference.target.fields[0]],
            source: s.id,
            index: i,
          })),
        );
      return {
        key: identity(field),
        width:
          field.array || field.nullable || reference || field.type === "string"
            ? 230
            : 185,
        title: (
          <div
            className="column-heading"
            onDragOver={(e) => e.preventDefault()}
            onDrop={(e) => {
              e.preventDefault();
              if (e.dataTransfer.types.includes("column-index"))
                columnMove(Number(e.dataTransfer.getData("column-index")), col);
            }}
          >
            <div className="column-title">
              <span
                draggable
                className="column-grip"
                aria-label="列を移動"
                onDragStart={(e) =>
                  e.dataTransfer.setData("column-index", String(col))
                }
              >
                <HolderOutlined />
              </span>
              {pk && (
                <Tooltip title="主キー">
                  <span className="key-dot">◆</span>
                </Tooltip>
              )}
              <Entry
                value={field.name}
                label={`列${col + 1}の名前`}
                onCommit={(v) => session.renameField(table.source.id, col, v)}
              />
              <Dropdown
                menu={{
                  items: [
                    {
                      key: "select",
                      label: "列を選択",
                      icon: <TableOutlined />,
                      disabled: !rows.length,
                    },
                    { key: "copy", label: "列を複製", icon: <CopyOutlined /> },
                    {
                      key: "left",
                      label: "左へ",
                      icon: <UpOutlined />,
                      disabled: col === 0,
                    },
                    {
                      key: "right",
                      label: "右へ",
                      icon: <DownOutlined />,
                      disabled: col === table.fields.length - 1,
                    },
                    {
                      key: "delete",
                      label: "列を削除",
                      icon: <DeleteOutlined />,
                      danger: true,
                    },
                  ],
                  onClick: ({ key }) =>
                    safe(() => {
                      if (key === "select") {
                        setRowKeys([]);
                        setCell({
                          row: rows.length - 1,
                          col,
                          anchorRow: 0,
                          anchorCol: col,
                        });
                        viewport.current
                          ?.closest<HTMLElement>(".grid-editor")
                          ?.focus();
                      } else if (key === "copy") {
                        pendingColumn.current = col + 1;
                        session.addField(table.source.id, col);
                      } else if (key === "delete")
                        session.removeField(table.source.id, col);
                      else columnMove(col, key === "left" ? col - 1 : col + 1);
                    }),
                }}
              >
                <Button
                  size="small"
                  type="text"
                  aria-label={`${field.name}の操作`}
                  icon={<MoreOutlined />}
                />
              </Dropdown>
            </div>
            <div className="column-type">
              <TypeSelect
                snapshot={snapshot}
                value={field.type}
                onChange={(value) =>
                  session.update(
                    table.source.id,
                    ["fields", col, "type"],
                    value,
                    "型を変更",
                  )
                }
              />
              <ModifierButtons
                field={field}
                onChange={(v) =>
                  session.update(
                    table.source.id,
                    ["fields", col],
                    v,
                    "項目の形を変更",
                  )
                }
              />
            </div>
          </div>
        ),
        onHeaderCell: () =>
          ({ "data-motion-key": `column-${identity(field)}` }) as any,
        onCell: (row: Row, rowIndex?: number) => {
          const i = rowIndex ?? rows.findIndex((r) => r.key === row.key);
          const selectedCell =
            cell &&
            i >= Math.min(cell.row, cell.anchorRow) &&
            i <= Math.max(cell.row, cell.anchorRow) &&
            col >= Math.min(cell.col, cell.anchorCol) &&
            col <= Math.max(cell.col, cell.anchorCol);
          const issues = problems.filter(
            (p) =>
              p.source === row.source.id &&
              p.path[0] === "records" &&
              p.path[1] === row.index &&
              (p.path[2] === field.name || p.path.length < 3),
          );
          return {
            "data-column-index": col,
            className: `data-cell ${pk ? "primary-cell" : ""} ${selectedCell ? "selected-cell" : ""} ${issues.length ? "error-cell" : ""}`,
            title: issues.map((p) => p.message).join("\n"),
            onClickCapture: (e: React.MouseEvent) => {
              if (e.shiftKey && cell && (cell.row !== i || cell.col !== col)) {
                e.preventDefault();
                e.stopPropagation();
                setCell({
                  row: i,
                  col,
                  anchorRow: cell.anchorRow,
                  anchorCol: cell.anchorCol,
                });
              }
            },
            onClick: (e: React.MouseEvent) =>
              setCell((old) => ({
                row: i,
                col,
                anchorRow: e.shiftKey && old ? old.anchorRow : i,
                anchorCol: e.shiftKey && old ? old.anchorCol : col,
              })),
          };
        },
        render: (_: unknown, row: Row) => (
          <ValueControl
            snapshot={snapshot}
            field={field}
            value={row.record?.[field.name]}
            label={`${field.name} 行${row.index + 1}`}
            onChange={(v) => update(row, field, v)}
            onOpen={() =>
              onOpen({
                source: row.source.id,
                path: ["records", row.index, field.name],
                field,
              })
            }
            reference={referenceOptions}
            navigate={(option) => {
              navigate(option.source);
            }}
          />
        ),
      };
    }),
    {
      key: "tags",
      title: <span className="tag-heading">配布タグ</span>,
      width: 210,
      render: (_, row) => (
        <Select
          aria-label={`行${row.index + 1}のタグ`}
          className="record-tags"
          mode="tags"
          maxTagCount="responsive"
          placeholder="タグを追加"
          value={row.record?.$tags ?? []}
          options={knownTags}
          onChange={(v) =>
            session.update(
              row.source.id,
              ["records", row.index, "$tags"],
              v,
              "タグを編集",
            )
          }
        />
      ),
    },
    {
      key: "actions",
      title: "",
      width: 48,
      render: (_, row) => (
        <Dropdown
          menu={{
            items: [
              { key: "copy", label: "複製", icon: <CopyOutlined /> },
              {
                key: "up",
                label: "上へ",
                icon: <UpOutlined />,
                disabled: row.index === 0,
              },
              {
                key: "down",
                label: "下へ",
                icon: <DownOutlined />,
                disabled:
                  row.index === (row.source.data.records?.length ?? 0) - 1,
              },
              {
                key: "delete",
                label: "削除",
                icon: <DeleteOutlined />,
                danger: true,
              },
            ],
            onClick: ({ key }) =>
              safe(() => {
                if (key === "copy")
                  session.addRow(table.source.id, row.source.id, row.record);
                if (key === "delete")
                  session.update(
                    row.source.id,
                    ["records"],
                    row.source.data.records.filter(
                      (_: any, i: number) => i !== row.index,
                    ),
                    "レコードを削除",
                  );
                if (key === "up" || key === "down")
                  move(row, {
                    ...row,
                    index: row.index + (key === "up" ? -1 : 1),
                    key: "destination",
                  });
              }),
          }}
        >
          <Button
            size="small"
            type="text"
            aria-label={`行${row.index + 1}の操作`}
            icon={<MoreOutlined />}
          />
        </Dropdown>
      ),
    },
  ];
  const copy = (event: React.ClipboardEvent) => {
    const target = event.target as HTMLInputElement;
    if (
      !event.clipboardData ||
      (!cell && !rowKeys.length) ||
      (!target.closest(".data-cell") &&
        target !== event.currentTarget &&
        !rowKeys.length) ||
      (target.tagName === "INPUT" &&
        target.selectionStart !== target.selectionEnd)
    )
      return;
    if (rowKeys.length) {
      const selected = new Set(rowKeys);
      const values = rows
        .filter((r) => selected.has(r.key))
        .map((r) => table.fields.map((f) => r.record?.[f.name]));
      event.clipboardData.setData(
        "text/plain",
        writeTsv(values.map((row) => row.map(cellText))),
      );
      event.clipboardData.setData(
        "application/x-masterdata",
        encodeCells(values),
      );
      event.preventDefault();
      return;
    }
    if (!cell) return;
    const firstRow = Math.min(cell.row, cell.anchorRow),
      lastRow = Math.max(cell.row, cell.anchorRow);
    const firstCol = Math.min(cell.col, cell.anchorCol),
      lastCol = Math.max(cell.col, cell.anchorCol);
    const values = rows
      .slice(firstRow, lastRow + 1)
      .map((r) =>
        table.fields
          .slice(firstCol, lastCol + 1)
          .map((f) => r.record?.[f.name]),
      );
    event.clipboardData.setData(
      "text/plain",
      writeTsv(values.map((row) => row.map(cellText))),
    );
    event.clipboardData.setData(
      "application/x-masterdata",
      encodeCells(values),
    );
    event.preventDefault();
  };
  const paste = (event: React.ClipboardEvent) => {
    const target = (event.target as HTMLElement).closest<HTMLElement>(
      ".data-cell",
    );
    if (!target && event.target !== event.currentTarget) return;
    const key = target?.closest("[data-row-key]")?.getAttribute("data-row-key");
    const index = rows.findIndex((r) => r.key === key);
    const anchor =
      target && index >= 0
        ? { row: index, col: Number(target.dataset.columnIndex) }
        : (cell ?? { row: 0, col: 0 });
    const content = event.clipboardData?.getData("text/plain") ?? "";
    const rich = event.clipboardData?.getData("application/x-masterdata") ?? "";
    const field = table.fields[anchor.col];
    if (!field || (!content && !rich)) return;
    if (
      !rich &&
      !content.includes("\t") &&
      !content.includes("\n") &&
      (event.target as HTMLElement).tagName === "INPUT"
    )
      return;
    event.preventDefault();
    safe(() => {
      const cells = rich ? decodeCells(rich) : readTsv(content);
      if (!Array.isArray(cells) || !cells.every(Array.isArray))
        throw new Error("貼り付けるデータを読み取れません。");
      if (cells.some((row) => row.length > table.fields.length - anchor.col))
        throw new Error(
          "貼り付け先の列が足りません。必要な列を追加してから貼り付けてください。",
        );
      (event.target as HTMLElement).blur();
      session.change("セルを貼り付け", (snapshot) => {
        const allocate = recordAllocator(
          snapshot,
          tables(snapshot).find((t) => t.source.id === table.source.id)!,
        );
        const targets = rows.map((row) => ({
          source: row.source.id,
          index: row.index,
        }));
        const targetId = rows[anchor.row]?.source.id ?? destination;
        const source =
          snapshot.sources.find((s) => s.id === targetId) ?? table.source;
        const original = Array.isArray(source.data?.records)
          ? source.data.records
          : [];
        const added: any[] = [];
        while (targets.length < anchor.row + cells.length) {
          targets.push({
            source: source.id,
            index: original.length + added.length,
          });
          added.push(allocate());
        }
        return {
          ...snapshot,
          sources: snapshot.sources.map((s) => {
            let data =
              s.id === source.id && added.length
                ? { ...s.data, records: [...original, ...added] }
                : s.data;
            cells.forEach((values, i) =>
              values.forEach((value, j) => {
                const row = targets[anchor.row + i];
                const field = table.fields[anchor.col + j];
                if (row.source === s.id)
                  data = setIn(
                    data,
                    ["records", row.index, field.name],
                    rich ? value : parseCell(snapshot, field, value),
                  );
              }),
            );
            return data === s.data ? s : { ...s, data };
          }),
        };
      });
    });
  };
  return (
    <div
      className="grid-editor"
      onCopy={copy}
      onPasteCapture={paste}
      onKeyDown={(e) => {
        if (
          e.nativeEvent.isComposing ||
          (e.target as HTMLElement).closest("input, textarea, [role=combobox]")
        )
          return;
        if (
          (e.metaKey || e.ctrlKey) &&
          e.key.toLowerCase() === "a" &&
          rows.length
        ) {
          e.preventDefault();
          setRowKeys([]);
          setCell({
            row: rows.length - 1,
            col: table.fields.length - 1,
            anchorRow: 0,
            anchorCol: 0,
          });
          return;
        }
        if (e.key === "Delete" && cell && !rowKeys.length) {
          e.preventDefault();
          session.change("セルをクリア", (s) => ({
            ...s,
            sources: s.sources.map((source) => {
              let data = source.data;
              for (
                let i = Math.min(cell.row, cell.anchorRow);
                i <= Math.max(cell.row, cell.anchorRow);
                i++
              )
                for (
                  let j = Math.min(cell.col, cell.anchorCol);
                  j <= Math.max(cell.col, cell.anchorCol);
                  j++
                ) {
                  const row = rows[i],
                    field = table.fields[j];
                  if (row?.source.id === source.id && field)
                    data = setIn(
                      data,
                      ["records", row.index, field.name],
                      defaultValue(s, field),
                    );
                }
              return data === source.data ? source : { ...source, data };
            }),
          }));
          return;
        }
        if (e.key === "Escape") {
          setRowKeys([]);
          setCell(null);
        }
        if (e.key === "Delete" && rowKeys.length) {
          e.preventDefault();
          removeRows();
        }
        if (cell && (e.key === "Enter" || e.key === "F2")) {
          e.preventDefault();
          const row = viewport.current?.querySelector(
            `[data-row-key="${CSS.escape(rows[cell.row]?.key ?? "")}"]`,
          );
          const target = row
            ?.querySelectorAll(".data-cell")
            [cell.col]?.querySelector<HTMLElement>(
              "input, button, [role=combobox]",
            );
          target?.focus();
          return;
        }
        if (cell && ["Home", "End", "PageUp", "PageDown"].includes(e.key)) {
          e.preventDefault();
          const row =
            e.key === "Home" && (e.metaKey || e.ctrlKey)
              ? 0
              : e.key === "End" && (e.metaKey || e.ctrlKey)
                ? rows.length - 1
                : Math.max(
                    0,
                    Math.min(
                      rows.length - 1,
                      cell.row +
                        (e.key === "PageDown"
                          ? 10
                          : e.key === "PageUp"
                            ? -10
                            : 0),
                    ),
                  );
          const col =
            e.key === "Home"
              ? 0
              : e.key === "End"
                ? table.fields.length - 1
                : cell.col;
          setCell({ row, col, anchorRow: row, anchorCol: col });
          tableRef.current?.scrollTo({ index: row });
          return;
        }
        if (
          cell &&
          ["ArrowDown", "ArrowUp", "ArrowLeft", "ArrowRight"].includes(e.key)
        ) {
          e.preventDefault();
          const row = Math.max(
            0,
            Math.min(
              rows.length - 1,
              cell.row +
                (e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0),
            ),
          );
          const col = Math.max(
            0,
            Math.min(
              table.fields.length - 1,
              cell.col +
                (e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0),
            ),
          );
          setCell({
            row,
            col,
            anchorRow: e.shiftKey ? cell.anchorRow : row,
            anchorCol: e.shiftKey ? cell.anchorCol : col,
          });
          tableRef.current?.scrollTo({ index: row });
        }
      }}
      tabIndex={0}
    >
      <div className="grid-toolbar">
        <Space>
          <Input
            ref={findRef}
            aria-label="表を検索"
            className="table-search"
            prefix={<SearchOutlined />}
            placeholder="この表を検索"
            allowClear
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <Select
            aria-label="表示するデータ"
            value={chosen}
            style={{ width: 190 }}
            onChange={setChosen}
            options={[
              { value: "all", label: "すべてのデータ" },
              ...table.sources.map((s) => ({
                value: s.id,
                label:
                  s.id === table.source.id
                    ? `${s.path
                        .split("/")
                        .at(-1)
                        ?.replace(/\.ya?ml$/, "")}（定義内）`
                    : s.path
                        .split("/")
                        .at(-1)
                        ?.replace(/\.ya?ml$/, ""),
              })),
            ]}
          />
        </Space>
        <Space>
          {rowKeys.length > 0 && (
            <Button danger icon={<DeleteOutlined />} onClick={removeRows}>
              {rowKeys.length}件を削除
            </Button>
          )}
          <Button
            icon={<PlusOutlined />}
            onClick={() => {
              pendingColumn.current = table.fields.length;
              session.addField(table.source.id);
            }}
          >
            列を追加
          </Button>
        </Space>
      </div>
      <div
        className="grid-viewport"
        ref={viewport}
        onDragOver={(e) => {
          if (!e.dataTransfer.types.includes("row-key")) return;
          e.preventDefault();
          const scroller = viewport.current?.querySelector(
            ".ant-table-tbody-virtual-holder, .ant-table-body",
          ) as HTMLElement | null;
          if (!scroller) return;
          const r = scroller.getBoundingClientRect();
          if (e.clientY > r.bottom - 60) scroller.scrollTop += 22;
          if (e.clientY < r.top + 60) scroller.scrollTop -= 22;
          if (e.clientX > r.right - 50) scroller.scrollLeft += 20;
          if (e.clientX < r.left + 50) scroller.scrollLeft -= 20;
        }}
      >
        <Table<Row>
          ref={tableRef}
          className="data-grid"
          size="small"
          virtual
          scroll={{
            x: columns.reduce((n, c) => n + Number(c.width ?? 180), 0) + 40,
            y: height,
          }}
          rowKey="key"
          dataSource={rows}
          columns={columns}
          pagination={false}
          rowSelection={{
            selectedRowKeys: rowKeys,
            onChange: (keys) => {
              setRowKeys(keys);
              setCell(null);
            },
            columnWidth: 36,
            fixed: true,
          }}
          onRow={(row) => ({
            className:
              profile &&
              !selected(row.record, snapshot.config.profiles[profile])
                ? "excluded-record"
                : "",
            onDragOver: (e) => e.preventDefault(),
            onDrop: (e) => {
              e.preventDefault();
              const from = rows.find(
                (r) => r.key === e.dataTransfer.getData("row-key"),
              );
              if (from) move(from, row);
            },
          })}
          locale={{
            emptyText: (
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description={
                  query
                    ? "一致するデータはありません"
                    : "最初のレコードを追加しましょう"
                }
              />
            ),
          }}
        />
      </div>
      <div className="grid-footer">
        <Space>
          <Button
            type="text"
            icon={<PlusOutlined />}
            onClick={() =>
              safe(() =>
                session.addRow(
                  table.source.id,
                  table.sources.some((s) => s.id === destination)
                    ? destination
                    : table.source.id,
                ),
              )
            }
          >
            レコードを追加
          </Button>
          <Select
            aria-label="追加先"
            size="small"
            value={destination}
            options={table.sources.map((s) => ({
              value: s.id,
              label:
                s.id === table.source.id
                  ? "定義ファイル内"
                  : s.path
                      .split("/")
                      .at(-1)
                      ?.replace(/\.ya?ml$/, ""),
            }))}
            onChange={setDestination}
            style={{ width: 155 }}
          />
          <Tooltip title="同じ表のデータを別のファイルに分けます">
            <Button
              size="small"
              type="text"
              onClick={() => {
                const id = session.add(
                  "data",
                  table.source.path.split("/").slice(0, -1).join("/"),
                  table.name,
                );
                setDestination(id);
              }}
            >
              データを分割
            </Button>
          </Tooltip>
        </Space>
        <Typography.Text type="secondary">
          {rows.length} レコード
          {profile
            ? ` · ${rows.filter((r) => selected(r.record, snapshot.config.profiles[profile])).length}件が出力対象`
            : ""}{" "}
          · {table.fields.length} 列
        </Typography.Text>
      </div>
    </div>
  );
}
