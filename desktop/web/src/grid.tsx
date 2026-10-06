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
  WarningOutlined,
  PlusOutlined,
  UndoOutlined,
  TagsOutlined,
} from "@ant-design/icons";
import type { Field, Projection, Row } from "./types";
import { desktop, GRID, type Editor } from "./workspace";
import {
  beginColumnDrag,
  beginRowDrag,
  beginRangeSelection,
  consumeSpatialClick,
} from "./spatial";

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
  const [rename, setRename] = useState<string | null>(null);
  useLayoutEffect(() => {
    desktop.viewport = viewport.current;
    if (p && !pending) {
      desktop.committed(p);
      if(document.activeElement===document.getElementById('editor-pane'))viewport.current?.focus({preventScroll:true});
    }
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
  useEffect(()=>setRename(null),[p?.clicked]);
  const beginRename=useCallback(async (field: string | null) => {
    if(field && !(await desktop.commit())) return;
    setRename(field);
  },[]);
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
    if (p && (e.key === "ContextMenu" || (e.shiftKey && e.key === "F10"))) {
      e.preventDefault();
      const selected = p.rows.find(row => row.viewIndex === desktop.interaction.selection.row);
      const anchor = viewport.current?.querySelector<HTMLElement>(`[data-row="${CSS.escape(selected?.id ?? "")}"] .row-actions`);
      if(selected && anchor) {
        const rect = anchor.getBoundingClientRect();
        openRowMenu(selected, rect.left, rect.bottom);
      }
      return;
    }
    const command = e.metaKey || e.ctrlKey;
    if (command && e.key.toLowerCase() === "z") {
      e.preventDefault();
      void desktop.undo(e.shiftKey);
      return;
    }
    if (e.ctrlKey && e.key.toLowerCase() === "y") {
      e.preventDefault();
      void desktop.undo(true);
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
    if ((e.target as Element).closest("button")) return;
    if (e.key === "Enter" || e.key === "F2") {
      e.preventDefault();
      desktop.beginEditor();
      return;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      desktop.closeComplex();
      desktop.collapseRange();
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
  const width = GRID.identity + (p?.columns.length ?? 0) * GRID.column + 44;
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
        aria-hidden={pending || !p}
        inert={pending || !p}
        className={pending || !p ? "grid-pending" : ""}
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
            gridTemplateColumns: `${GRID.identity}px repeat(${p?.columns.length ?? 0}, ${GRID.column}px) 44px`,
          }}
        >
          <div className="corner" role="columnheader">
            #
          </div>
          {p?.columns.map((col, index) => (
            <ColumnHeader
              key={col.field.name}
              index={index}
              field={col.field}
              types={types}
              active={index >= columns.first && index < columns.last}
              busy={busy}
              menu={openColumnMenu}
              renaming={rename===col.field.name}
              rename={beginRename}
            />
          ))}
          <div className="add-column"><Tooltip title="Add nullable string field"><Button type="text" icon={<PlusOutlined />} aria-label="Add column" disabled={!p || busy || pending || desktop.surface.status.recoveryRequired} onClick={()=>{if(p) void desktop.fieldAction({kind:"add",neighbor:null,after:false},p);}} /></Tooltip></div>
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
const ColumnHeader = memo(
  function ColumnHeader({
    field,
    index,
    active,
    types,
    busy,
    menu,
    renaming,
    rename,
  }: {
    field: Field;
    index: number;
    active: boolean;
    types: string[];
    busy: boolean;
    menu: (column: number, x: number, y: number) => void;
    renaming: boolean;
    rename: (field: string | null) => Promise<void>;
  }) {
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
            disabled={busy}
            onPointerDown={(e) => beginColumnDrag(e, index)}
            onClick={(e) => {
              if (consumeSpatialClick(e.detail)) return;
              const r = e.currentTarget.getBoundingClientRect();
              menu(index, r.left, r.bottom);
            }}
          >
            <HolderOutlined />
          </button>
          <HeaderName field={field} active={active} editing={renaming} busy={busy} change={rename} />
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
                    void desktop.schema(field.name, {
                      nullable: !field.nullable,
                    })
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
  },
  (a, b) =>
    a.index === b.index &&
    a.active === b.active &&
    a.types === b.types &&
    a.busy === b.busy &&
    a.menu === b.menu &&
    a.renaming === b.renaming &&
    a.rename === b.rename &&
    a.field.name === b.field.name &&
    a.field.key === b.field.key &&
    a.field.typeName === b.field.typeName &&
    a.field.nullable === b.field.nullable &&
    a.field.array === b.field.array,
);
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
    const previousTags = desktop.interaction.tags;
    desktop.setSelection(row.viewIndex, column, extend);
    desktop.viewport?.focus();
    void desktop.commit().then((ok) => {
      if (ok && desktop.interaction.complex === previousComplex)
        desktop.closeComplex();
      if (ok && desktop.interaction.tags === previousTags) desktop.closeTags();
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
          disabled={
            row.pendingDelete ||
            !!p.viewState.search ||
            p.writeStates.some(
              (s) =>
                s.outcome === "OutcomeUnknown" ||
                s.outcome === "RecoveryRequired",
            )
          }
          onPointerDown={(e) => beginRowDrag(e, row)}
          onClick={(e) => {
            if (consumeSpatialClick(e.detail)) return;
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
              beginRangeSelection(e, row, column);
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
      <span className="sr-only" role="status" aria-live="polite">
        {anchor
          ? `${Math.abs(anchor.row - s.row) + 1} rows, ${Math.abs(anchor.column - s.column) + 1} columns selected`
          : `Row ${s.row + 1}, ${s.field ?? "cell"}`}
      </span>
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
    const unbind = desktop.bindEditor(commit, () => ({ source: editor.source, revision: editor.revision, generation: editor.generation, label: `${editor.field} · Row ${editor.rowIndex + 1}`, text: value.current, dirty: value.current !== editor.initial, cancel: desktop.closeEditor }));
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
        onBlur={() => { if (!desktop.surface.externalPending) void commit(); }}
        onKeyDown={(e) => {
          if (
            (e.metaKey || e.ctrlKey) &&
            ["s", "f"].includes(e.key.toLowerCase()) &&
            !e.nativeEvent.isComposing &&
            !composing.current
          )
            return;
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
function HeaderName({field,active,editing,busy,change}:{field:Field;active:boolean;editing:boolean;busy:boolean;change:(field:string|null)=>Promise<void>}) {
  const input=useRef<InputRef>(null), text=useRef(field.name), origin=useRef<Projection|null>(null), running=useRef<Promise<boolean>|null>(null), composing=useRef(false);
  const [value,setValue]=useState(field.name),[reason,setReason]=useState<string|null>(null);
  const cancel=useCallback(()=>{text.current=field.name;setValue(field.name);setReason(null);void change(null);},[field.name,change]);
  const commit=useCallback(():Promise<boolean>=>{
    if(composing.current)return Promise.resolve(false);
    if(running.current) return running.current;
    if(text.current===field.name){void change(null);return Promise.resolve(true);}
    const previous=origin.current,current=desktop.surface.projection;
    if(!previous || !current || current.clicked!==previous.clicked || current.sessionEpoch!==previous.sessionEpoch)return Promise.resolve(false);
    const expected=current;
    const intent=desktop.inputIntent;
    running.current=(async()=>{
      try {
        const review=await desktop.fieldOperation({kind:"rename",field:field.name,newName:text.current},expected,true);
        if(!review)return false;
        void change(null);
        const name=review.command.newName;
        if(name && desktop.surface.target===expected.clicked && intent===desktop.inputIntent) requestAnimationFrame(()=>{if(desktop.surface.target===expected.clicked && intent===desktop.inputIntent)desktop.viewport?.querySelector<HTMLButtonElement>(`.column[data-field="${CSS.escape(name)}"] .field-name-button`)?.focus();});
        return true;
      } catch(e) {
        setReason(typeof e==="object" && e && "message" in e ? String(e.message) : String(e));
        if(desktop.surface.target===expected.clicked && intent===desktop.inputIntent)requestAnimationFrame(()=>{if(desktop.surface.target===expected.clicked && intent===desktop.inputIntent)input.current?.focus({preventScroll:true});});
        return false;
      }
      finally {running.current=null;}
    })();
    return running.current;
  },[field.name,change]);
  useLayoutEffect(()=>{
    if(!editing)return;
    origin.current=desktop.surface.projection; text.current=field.name;setValue(field.name);setReason(null);
    input.current?.focus({cursor:"all"});
  },[editing,field.name]);
  useEffect(()=>{
    if(!editing)return;
    return desktop.bindEditor(commit,()=>{
      const p=origin.current;
      return p ? {source:p.table.source,revision:p.schemaRevision,generation:p.generation,label:`${field.name} name`,text:text.current,dirty:text.current!==field.name,cancel} : null;
    });
  },[editing,field.name,commit,cancel]);
  // Keep the Input and suffix DOM stable during pending/error feedback: replacing
  // Ant's affix structure or disabling the input loses the user's keyboard focus.
  return <span className="field-name" title={field.name}>
    {editing ? <Tooltip title={reason}><Input ref={input} aria-label={`Rename ${field.name}`} aria-invalid={!!reason} aria-busy={busy} status={reason ? "error" : undefined} value={value} readOnly={busy} suffix={<span>{reason && <WarningOutlined/>}</span>} onCompositionStart={()=>{composing.current=true;}} onCompositionEnd={()=>{composing.current=false;}} onChange={e=>{text.current=e.target.value;setValue(e.target.value);setReason(null);}} onKeyDown={e=>{
      if(e.nativeEvent.isComposing || e.keyCode===229)return;
      if(e.key==="Escape"){e.preventDefault();e.stopPropagation();const clicked=origin.current?.clicked,intent=desktop.inputIntent;cancel();requestAnimationFrame(()=>{if(desktop.surface.target===clicked&&!desktop.surface.pending&&intent===desktop.inputIntent)desktop.viewport?.querySelector<HTMLButtonElement>(`.column[data-field="${CSS.escape(field.name)}"] .field-name-button`)?.focus();});}
      else if(e.key==="Enter" || e.key==="Tab"){e.preventDefault();e.stopPropagation();void commit();}
    }} /></Tooltip> : active ? <Button type="text" className="field-name-button" aria-label={`Rename ${field.name}`} disabled={busy} onClick={()=>void change(field.name)}>{field.name}</Button> : field.name}
  </span>;
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
        {key: "tags", label: "Tags…", icon: <TagsOutlined />},
        {type: "divider"},
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
      {key:"insertLeft",label:"Insert Left",icon:<PlusOutlined/>},
      {key:"insertRight",label:"Insert Right",icon:<PlusOutlined/>},
      {type:"divider"},
      { key: "left", label: "Move Left", disabled: target.column === 0 },
      {
        key: "right",
        label: "Move Right",
        disabled: target.column === target.p.columns.length - 1,
      },
      {type:"divider"},
      {key:"drop",label:"Drop Field…",icon:<DeleteOutlined/>,danger:true},
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
            const field=target.p.columns[target.column].field.name;
            if(key==="drop") {void desktop.fieldAction({kind:"drop",field},target.p);return;}
            if(key==="insertLeft" || key==="insertRight") {void desktop.fieldAction({kind:"add",neighbor:field,after:key==="insertRight"},target.p);return;}
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
