import {
  memo,
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  useSyncExternalStore,
  type CSSProperties,
  type KeyboardEvent,
} from "react";
import {
  Button,
  Dropdown,
  Input,
  Select,
  Tooltip,
  type InputRef,
  type MenuProps,
} from "antd";
import {
  HolderOutlined,
  MoreOutlined,
  QuestionOutlined,
  SwapOutlined,
  ArrowUpOutlined,
  ArrowDownOutlined,
  DeleteOutlined,
  PlusOutlined,
  UndoOutlined,
} from "@ant-design/icons";
import type { Projection, Row } from "./types";
import { desktop, GRID, type Editor } from "./workspace";

function isInput(target: EventTarget | null) {
  return (
    target instanceof HTMLElement &&
    !!target.closest("input,textarea,[contenteditable=true],[role=combobox]")
  );
}
type MenuTarget = {
  p: Projection;
  row?: Row;
  column?: number;
  x: number;
  y: number;
};
export const AuthoringGrid = memo(function AuthoringGrid({
  projection: p,
  types,
  pending,
  busy,
}: {
  projection: Projection | null;
  types: string[];
  pending: boolean;
  busy: boolean;
}) {
  const viewport = useRef<HTMLDivElement>(null),
    frame = useRef<number | null>(null);
  const [columns, setColumns] = useState({ first: 0, last: 8 });
  const [menu, setMenu] = useState<MenuTarget | null>(null);
  useLayoutEffect(() => {
    desktop.viewport = viewport.current;
    if (p && !pending) desktop.committed(p);
  }, [p, pending]);
  useEffect(() => {
    const v = viewport.current;
    if (!v) return;
    const update = () => {
      setColumns({
        first: Math.max(0, Math.floor(v.scrollLeft / GRID.column) - 1),
        last: Math.ceil((v.scrollLeft + v.clientWidth) / GRID.column) + 1,
      });
      void desktop.ensureWindow();
    };
    const observer = new ResizeObserver(update);
    observer.observe(v);
    update();
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    setMenu(null);
  }, [p?.clicked, p?.revision, p?.schemaRevision, p?.generation, pending]);
  const onScroll = () => {
    if (frame.current !== null) return;
    frame.current = requestAnimationFrame(() => {
      frame.current = null;
      const v = viewport.current;
      if (!v) return;
      const first = Math.max(0, Math.floor(v.scrollLeft / GRID.column) - 1),
        last = Math.ceil((v.scrollLeft + v.clientWidth) / GRID.column) + 1;
      setColumns((previous) =>
        previous.first === first && previous.last === last
          ? previous
          : { first, last },
      );
      void desktop.ensureWindow();
    });
  };
  const openRowMenu = useCallback((row: Row, x: number, y: number) => {
    const p = desktop.surface.projection;
    if (p) setMenu({ p, row, x, y });
  }, []);
  const openColumnMenu = useCallback((column: number, x: number, y: number) => {
    const p = desktop.surface.projection;
    if (p) setMenu({ p, column, x, y });
  }, []);
  const keydown = (e: KeyboardEvent) => {
    if (isInput(e.target) || e.nativeEvent.isComposing || pending) return;
    const command = e.metaKey || e.ctrlKey;
    if (command && e.key.toLowerCase() === "z") {
      e.preventDefault();
      void desktop.undo(e.shiftKey);
      return;
    }
    if (command && e.key.toLowerCase() === "c") {
      e.preventDefault();
      void desktop.copyGrid();
      return;
    }
    if (command && e.key.toLowerCase() === "v") {
      e.preventDefault();
      void desktop.pasteGrid();
      return;
    }
    if (e.key === "Enter" || e.key === "F2") {
      e.preventDefault();
      desktop.beginEditor();
      return;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      desktop.closeComplex();
      setMenu(null);
      return;
    }
    const moves: Record<string, [number, number]> = {
      ArrowDown: [1, 0],
      ArrowUp: [-1, 0],
      ArrowLeft: [0, -1],
      ArrowRight: [0, 1],
      Tab: [0, e.shiftKey ? -1 : 1],
    };
    if (moves[e.key]) {
      e.preventDefault();
      desktop.move(...moves[e.key], e.shiftKey && e.key !== "Tab");
    }
  };
  const width = GRID.identity + (p?.columns.length ?? 0) * GRID.column;
  const style = {
    "--row-height": `${GRID.row}px`,
    "--header-height": `${GRID.header}px`,
    "--identity-width": `${GRID.identity}px`,
    "--column-width": `${GRID.column}px`,
  } as CSSProperties;
  return (
    <>
      <div
        id="viewport"
        ref={viewport}
        role="grid"
        aria-label="Table values"
        tabIndex={0}
        aria-rowcount={(p?.totalRows ?? 0) + 1}
        aria-colcount={(p?.columns.length ?? 0) + 1}
        aria-busy={pending}
        aria-hidden={pending}
        inert={pending}
        className={pending ? "grid-pending" : ""}
        style={style}
        onScroll={onScroll}
        onKeyDown={keydown}
        onPaste={(e) => {
          if (!isInput(e.target)) {
            e.preventDefault();
            void desktop.pasteGrid(e.clipboardData.getData("text/plain"));
          }
        }}
      >
        <div
          id="grid-header"
          role="row"
          style={{
            width,
            gridTemplateColumns: `${GRID.identity}px repeat(${p?.columns.length ?? 0}, ${GRID.column}px)`,
          }}
        >
          <div className="corner" role="columnheader">
            #
          </div>
          {p?.columns.map((col, index) => (
            <ColumnHeader
              key={col.field.name}
              index={index}
              p={p}
              types={types}
              active={index >= columns.first && index < columns.last}
              busy={busy}
              menu={openColumnMenu}
            />
          ))}
        </div>
        <div
          id="grid-body"
          style={{ width, height: (p?.totalRows ?? 0) * GRID.row }}
        >
          <div id="rows" style={{ top: (p?.rowStart ?? 0) * GRID.row, width }}>
            {p?.rows.map((row) => (
              <GridRow
                key={row.id}
                row={row}
                p={p}
                first={columns.first}
                last={columns.last}
                menu={openRowMenu}
              />
            ))}
          </div>
        </div>
        <SelectionOverlay projection={p} />
        <ActiveEditor />
      </div>
      <GridMenu
        target={menu}
        close={() => {
          setMenu(null);
          viewport.current?.focus();
        }}
      />
    </>
  );
});
const ColumnHeader = memo(function ColumnHeader({
  p,
  index,
  active,
  types,
  busy,
  menu,
}: {
  p: Projection;
  index: number;
  active: boolean;
  types: string[];
  busy: boolean;
  menu: (column: number, x: number, y: number) => void;
}) {
  const field = p.columns[index].field;
  const options = [
    ...new Set([
      "int",
      "uint",
      "long",
      "ulong",
      "float",
      "double",
      "bool",
      "string",
      ...types,
      field.typeName,
    ]),
  ].map((value) => ({ value, label: value }));
  return (
    <div
      className="column"
      role="columnheader"
      aria-colindex={index + 2}
      data-field={field.name}
      onFocus={desktop.focusSchema}
    >
      <div className="column-identity">
        <button
          className="spatial-handle column-grip"
          aria-label={`Reorder column ${field.name}`}
          title={`Reorder column ${field.name}`}
          onClick={(e) => {
            const r = e.currentTarget.getBoundingClientRect();
            menu(index, r.left, r.bottom);
          }}
        >
          <HolderOutlined />
        </button>
        <span className="field-name" title={field.name}>
          {field.name}
        </span>
        <button
          className="spatial-handle column-actions"
          aria-label={`${field.name} actions`}
          title={`${field.name} actions`}
          onClick={(e) => {
            const r = e.currentTarget.getBoundingClientRect();
            menu(index, r.right - 180, r.bottom);
          }}
        >
          <MoreOutlined />
        </button>
      </div>
      <div className="column-shape">
        {active ? (
          <>
            <Select
              aria-label={`${field.name} type`}
              variant="borderless"
              value={field.typeName}
              options={options}
              disabled={busy}
              onChange={(typeName) =>
                void desktop.schema(field.name, { typeName })
              }
              className="field-type"
              popupMatchSelectWidth={160}
            />
            <Tooltip title="Nullable">
              <Button
                type="text"
                className={field.nullable ? "modifier active" : "modifier"}
                icon={<QuestionOutlined />}
                aria-label={`${field.name} nullable`}
                aria-pressed={field.nullable}
                disabled={busy}
                onClick={() =>
                  void desktop.schema(field.name, { nullable: !field.nullable })
                }
              />
            </Tooltip>
            <Tooltip title="Array">
              <Button
                type="text"
                className={field.array ? "modifier active" : "modifier"}
                icon={<SwapOutlined />}
                aria-label={`${field.name} array`}
                aria-pressed={field.array}
                disabled={busy}
                onClick={() =>
                  void desktop.schema(field.name, { array: !field.array })
                }
              />
            </Tooltip>
          </>
        ) : (
          <span className="shape-summary">
            {field.typeName}
            {field.nullable ? " ?" : ""}
            {field.array ? " []" : ""}
          </span>
        )}
      </div>
    </div>
  );
});
const GridRow = memo(function GridRow({
  row,
  p,
  first,
  last,
  menu,
}: {
  row: Row;
  p: Projection;
  first: number;
  last: number;
  menu: (row: Row, x: number, y: number) => void;
}) {
  const choose = (column: number, extend: boolean) => {
    const previousComplex = desktop.interaction.complex;
    desktop.setSelection(row.viewIndex, column, extend);
    desktop.viewport?.focus();
    void desktop.commit().then((ok) => {
      if (ok && desktop.interaction.complex === previousComplex)
        desktop.closeComplex();
    });
  };
  return (
    <div
      className={`grid-row${row.pendingDelete ? " pending-delete" : ""}${row.added ? " added-row" : ""}`}
      data-row={row.id}
      role="row"
      aria-rowindex={row.viewIndex + 2}
      style={{
        gridTemplateColumns: `${GRID.identity}px repeat(${p.columns.length}, ${GRID.column}px)`,
      }}
    >
      <div className="row-identity" role="rowheader">
        <button
          className="spatial-handle row-grip"
          aria-label={`Reorder row ${row.occurrence}`}
          title="Reorder row"
          onClick={(e) => {
            const r = e.currentTarget.getBoundingClientRect();
            menu(row, r.left, r.bottom);
          }}
        >
          <HolderOutlined />
        </button>
        <span
          title={
            row.pendingDelete ? "Pending Delete" : row.added ? "Added Row" : ""
          }
        >
          {row.occurrence}
        </span>
        <button
          className="spatial-handle row-actions"
          aria-label={`Row ${row.occurrence} actions`}
          title="Row actions"
          onClick={(e) => {
            const r = e.currentTarget.getBoundingClientRect();
            menu(row, r.left, r.bottom);
          }}
        >
          <MoreOutlined />
        </button>
      </div>
      {row.cells.slice(first, last).map((cell, at) => {
        const column = at + first;
        return (
          <div
            key={p.columns[column].field.name}
            className={`cell${cell.valid ? "" : " invalid"}`}
            id={`cell-${row.viewIndex}-${column}`}
            role="gridcell"
            aria-colindex={column + 2}
            aria-readonly={!cell.editable}
            aria-invalid={!cell.valid}
            data-column={column}
            style={{ gridColumn: column + 2 }}
            title={
              cell.problem
                ? `${cell.problem.code} ${cell.problem.message}`
                : (cell.reason ?? undefined)
            }
            onPointerDown={(e) => {
              if (e.button === 0) choose(column, e.shiftKey);
            }}
            onDoubleClick={() => {
              const intent = desktop.interaction.focusIntent;
              void desktop.commit().then((ok) => {
                if (ok && intent === desktop.interaction.focusIntent)
                  desktop.beginEditor();
              });
            }}
            onContextMenu={(e) => {
              e.preventDefault();
              choose(column, false);
              menu(row, e.clientX, e.clientY);
            }}
          >
            {cell.display}
          </div>
        );
      })}
    </div>
  );
});
function SelectionOverlay({
  projection: p,
}: {
  projection: Projection | null;
}) {
  const { selection: s, anchor } = useSyncExternalStore(
    desktop.subscribeInteraction,
    desktop.interactionSnapshot,
  );
  useLayoutEffect(() => {
    const v = desktop.viewport;
    if (!v) return;
    v.querySelector('[aria-selected="true"]')?.setAttribute(
      "aria-selected",
      "false",
    );
    const cell = document.getElementById(`cell-${s.row}-${s.column}`);
    if (cell) {
      cell.setAttribute("aria-selected", "true");
      v.setAttribute("aria-activedescendant", cell.id);
    } else v.removeAttribute("aria-activedescendant");
  }, [s, p]);
  if (!p?.totalRows || !p.columns.length) return null;
  const rect = (
    row: number,
    column: number,
    width = GRID.column,
    height = GRID.row,
  ) => ({
    left: GRID.identity + column * GRID.column,
    top: GRID.header + row * GRID.row,
    width,
    height,
  });
  return (
    <>
      {anchor && (
        <div
          className="range-outline"
          style={rect(
            Math.min(anchor.row, s.row),
            Math.min(anchor.column, s.column),
            (Math.abs(anchor.column - s.column) + 1) * GRID.column,
            (Math.abs(anchor.row - s.row) + 1) * GRID.row,
          )}
          aria-hidden="true"
        />
      )}
      <div
        className="selection-outline"
        style={rect(s.row, s.column)}
        aria-hidden="true"
      />
    </>
  );
}
function ActiveEditor() {
  const { editor } = useSyncExternalStore(
    desktop.subscribeInteraction,
    desktop.interactionSnapshot,
  );
  return editor ? (
    <CellInput
      key={`${editor.epoch}:${editor.source}:${editor.row}:${editor.field}`}
      editor={editor}
    />
  ) : null;
}
function CellInput({ editor }: { editor: Editor }) {
  const input = useRef<InputRef>(null),
    value = useRef(editor.initial),
    composing = useRef(false),
    committing = useRef<Promise<boolean> | null>(null);
  const [text, setText] = useState(editor.initial);
  const commit = useCallback(() => {
    if (composing.current) return Promise.resolve(false);
    if (committing.current) return committing.current;
    committing.current = desktop.edit(editor, value.current).finally(() => {
      committing.current = null;
    });
    return committing.current;
  }, [editor]);
  useLayoutEffect(() => {
    const unbind = desktop.bindEditor(commit);
    input.current?.focus({ cursor: "all" });
    return unbind;
  }, [commit]);
  return (
    <div
      className="active-cell-editor"
      style={{
        top: GRID.header + editor.rowIndex * GRID.row,
        left: GRID.identity + editor.column * GRID.column,
        width: GRID.column,
        height: GRID.row,
      }}
    >
      <Input
        ref={input}
        aria-label={`${editor.field} row ${editor.rowIndex + 1}`}
        value={text}
        onChange={(e) => {
          value.current = e.target.value;
          setText(e.target.value);
        }}
        onCompositionStart={() => {
          composing.current = true;
        }}
        onCompositionEnd={() => {
          composing.current = false;
        }}
        onBlur={() => void commit()}
        onKeyDown={(e) => {
          e.stopPropagation();
          if (e.nativeEvent.isComposing || composing.current) return;
          if (e.key === "Escape") {
            e.preventDefault();
            desktop.closeEditor();
            desktop.viewport?.focus();
          } else if (e.key === "Enter" || e.key === "Tab") {
            e.preventDefault();
            const key = e.key,
              direction = e.shiftKey ? -1 : 1;
            void commit().then((ok) => {
              if (
                ok &&
                desktop.surface.projection?.source === editor.source &&
                desktop.surface.status.epoch === editor.epoch
              ) {
                desktop.move(
                  key === "Enter" ? direction : 0,
                  key === "Tab" ? direction : 0,
                );
                desktop.viewport?.focus();
              }
            });
          }
        }}
      />
    </div>
  );
}
function GridMenu({
  target,
  close,
}: {
  target: MenuTarget | null;
  close: () => void;
}) {
  const items: MenuProps["items"] = [];
  if (target?.row) {
    const row = target.row,
      p = target.p,
      position = !!p.viewState.search || row.pendingDelete;
    if (row.pendingDelete)
      items.push({
        key: "restore",
        label: "Undo Delete",
        icon: <UndoOutlined />,
      });
    else
      items.push(
        {
          key: "above",
          label: "Insert Above",
          icon: <PlusOutlined />,
          disabled: position || !p.canAdd,
        },
        {
          key: "below",
          label: "Insert Below",
          icon: <PlusOutlined />,
          disabled: position || !p.canAdd,
        },
        { type: "divider" },
        {
          key: "up",
          label: "Move Up",
          icon: <ArrowUpOutlined />,
          disabled: position,
        },
        {
          key: "down",
          label: "Move Down",
          icon: <ArrowDownOutlined />,
          disabled: position,
        },
        { type: "divider" },
        {
          key: "delete",
          label: "Delete",
          icon: <DeleteOutlined />,
          danger: true,
        },
      );
  } else if (target?.column !== undefined) {
    items.push(
      { key: "left", label: "Move Left", disabled: target.column === 0 },
      {
        key: "right",
        label: "Move Right",
        disabled: target.column === target.p.columns.length - 1,
      },
    );
  }
  return (
    <Dropdown
      open={!!target}
      trigger={[]}
      autoFocus
      onOpenChange={(open) => {
        if (!open) close();
      }}
      menu={{
        items,
        onClick: ({ key }) => {
          if (!target) return;
          close();
          if (target.row) void desktop.rowAction(key, target.p, target.row.id);
          else if (target.column !== undefined) {
            const order = target.p.columns.map((c) => c.field.name),
              at = target.column,
              next = at + (key === "left" ? -1 : 1);
            [order[at], order[next]] = [order[next], order[at]];
            void desktop.columns(order, target.p);
          }
        },
      }}
    >
      <span
        className="menu-anchor"
        style={{ position: "fixed", left: target?.x ?? 0, top: target?.y ?? 0 }}
        aria-hidden="true"
      />
    </Dropdown>
  );
}
