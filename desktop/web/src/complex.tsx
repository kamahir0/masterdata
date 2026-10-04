import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import {
  Alert,
  Button,
  Checkbox,
  Drawer,
  Dropdown,
  Empty,
  Input,
  Select,
  Space,
  Spin,
  Tag,
  Typography,
  type InputRef,
  type MenuProps,
} from "antd";
import {
  ArrowDownOutlined,
  ArrowUpOutlined,
  DeleteOutlined,
  HolderOutlined,
  LeftOutlined,
  MoreOutlined,
  PlusOutlined,
  RightOutlined,
  WarningOutlined,
} from "@ant-design/icons";
import { useInteraction, useSurface } from "./app";
import { desktop, type ComplexView, type EditorNode } from "./workspace";
import { beginArrayDrag, consumeSpatialClick } from "./spatial";
const same = (a: string[], b: string[]) =>
  a.length === b.length && a.every((s, i) => s === b[i]);
type Typing = {
  authority: ComplexView;
  path: string[];
  initial: string;
  text: string;
  composing: boolean;
  label: string;
  cancel: () => void;
};
export function ComplexPanel() {
  const { complex: target } = useInteraction(),
    s = useSurface();
  const [view, setView] = useState<ComplexView | null>(null),
    [error, setError] = useState<string | null>(null),
    [loading, setLoading] = useState(false);
  const [active, setActive] = useState<string[] | null>(null),
    [nonNull, setNonNull] = useState(false);
  const [menu, setMenu] = useState<{
    child: EditorNode;
    index: number;
    x: number;
    y: number;
  } | null>(null);
  const typing = useRef<Typing | null>(null),
    committing = useRef<Promise<boolean> | null>(null),
    body = useRef<HTMLDivElement>(null),
    request = useRef(0),
    viewRef = useRef(view);
  const focusedTarget = useRef<typeof target>(null);
  // focus-only / window changes can retain the same value path. Focus must
  // wait for that target's descriptor; a later pending state would make its
  // newly focused control inert and silently lose focus.
  const resolvedTarget = useRef<typeof target>(null);
  const restoreFocus = useRef<{
    target: typeof target;
    intent: number;
    element: HTMLElement;
    path?: string;
  } | null>(null);
  const blocked =
    s.pending ||
    s.busy ||
    s.status.recoveryRequired ||
    s.status.uncertain.includes(target?.source ?? "");
  viewRef.current = view;
  useEffect(() => {
    const mine = ++request.current;
    if (s.pending && s.externalPending) return;
    if (
      !target ||
      s.pending ||
      target.epoch !== s.status.epoch ||
      target.source !== s.projection?.source
    ) {
      setView(null);
      resolvedTarget.current = null;
      setMenu(null);
      return;
    }
    setLoading(true);
    setError(null);
    setMenu(null);
    void desktop
      .complexView(target)
      .then((value) => {
        if (
          mine !== request.current ||
          desktop.interaction.complex !== target ||
          value?.generation !== desktop.surface.projection?.generation
        )
          return;
        resolvedTarget.current = target;
        setView(value);
        setLoading(false);
      })
      .catch((e) => {
        if (mine === request.current) {
          setError(
            typeof e === "object" && e && "message" in e
              ? String(e.message)
              : String(e),
          );
          setView(null);
          setLoading(false);
        }
      });
    return () => {
      request.current++;
    };
  }, [
    target,
    s.projection?.revision,
    s.projection?.generation,
    s.pending,
    s.externalPending,
    s.status.epoch,
  ]);
  const operation = useCallback(
    async (
      path: string[],
      op: Record<string, unknown>,
      observed?: ComplexView,
    ) => {
      const value = observed ?? viewRef.current;
      if (!value) return false;
      const element = document.activeElement as HTMLElement | null;
      restoreFocus.current =
        element && body.current?.contains(element)
          ? {
              target: desktop.interaction.complex,
              intent: desktop.inputIntent,
              element,
              path: element.closest<HTMLElement>("[data-value-path]")?.dataset
                .valuePath,
            }
          : null;
      const ok = await desktop.complexEdit(value, path, op);
      if (ok) {
        setActive(null);
        setNonNull(false);
        typing.current = null;
        setMenu(null);
      } else restoreFocus.current = null;
      return ok;
    },
    [],
  );
  const commit = useCallback((): Promise<boolean> => {
    if (committing.current) return committing.current;
    const current = typing.current;
    if (!current || current.text === current.initial)
      return Promise.resolve(true);
    if (current.composing) return Promise.resolve(false);
    committing.current = operation(
      current.path,
      {
        kind: "text",
        text: current.text,
      },
      current.authority,
    ).finally(() => {
      committing.current = null;
    });
    return committing.current;
  }, [operation]);
  useLayoutEffect(() => {
    if (!target) return;
    return desktop.bindEditor(commit, () => {
      const value = typing.current;
      return value ? { source: value.authority.source, revision: value.authority.revision, generation: value.authority.generation, label: `${value.authority.node.label} · ${value.label}`, text: value.text, dirty: value.text !== value.initial, cancel: () => { value.cancel(); typing.current = null; setActive(null); setNonNull(false); } } : null;
    });
  }, [target, commit]);
  useEffect(() => {
    setActive(null);
    setNonNull(false);
    typing.current = null;
    setMenu(null);
  }, [target?.row, target?.source, target?.path]);
  const close = async () => {
    const current = desktop.interaction.complex;
    if ((await commit()) && desktop.interaction.complex === current) {
      desktop.closeComplex();
      desktop.viewport?.focus();
    }
  };
  const navigate = async (path: string[], start = 0) => {
    const previous = desktop.interaction.complex;
    if ((await commit()) && desktop.interaction.complex === previous)
      desktop.complexPath(path, start);
  };
  const change = async (path: string[], op: Record<string, unknown>) => {
    const previous = desktop.interaction.complex,
      observed = viewRef.current,
      p = desktop.surface.projection;
    if (
      !previous ||
      !observed ||
      p?.revision !== observed.revision ||
      p.generation !== observed.generation
    )
      return false;
    if (!(await commit()) || desktop.interaction.complex !== previous)
      return false;
    const current = desktop.surface.projection;
    if (
      current?.revision !== observed.revision ||
      current.generation !== observed.generation
    ) {
      // Completing this editor's own typing can advance the descriptor. Fetch
      // its bounded replacement before composing the user's next direct action.
      const next = await desktop.complexView(previous);
      if (
        !next ||
        desktop.interaction.complex !== previous ||
        next.generation !== desktop.surface.projection?.generation ||
        next.revision !== desktop.surface.projection.revision
      )
        return false;
      viewRef.current = next;
      setView(next);
    }
    return operation(path, op);
  };
  const keydown = (e: KeyboardEvent) => {
    if (
      e.isDefaultPrevented() ||
      e.nativeEvent.isComposing ||
      typing.current?.composing
    )
      return;
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      if (typing.current && typing.current.text !== typing.current.initial) {
        typing.current.cancel();
        typing.current = null;
      } else if (active || nonNull) {
        setActive(null);
        setNonNull(false);
        typing.current = null;
      } else if (menu) setMenu(null);
      else void close();
    } else if (
      (e.metaKey || e.ctrlKey) &&
      e.key.toLowerCase() === "z" &&
      !(e.target as Element).closest("input,textarea")
    ) {
      e.preventDefault();
      e.stopPropagation();
      void desktop.undo(e.shiftKey);
    } else if (
      e.ctrlKey &&
      e.key.toLowerCase() === "y" &&
      !(e.target as Element).closest("input,textarea")
    ) {
      e.preventDefault();
      e.stopPropagation();
      void desktop.undo(true);
    }
  };
  const control = (node: EditorNode) => {
    if (!node.editable || !node.shape)
      return (
        <Typography.Text type="secondary" title={node.reason ?? undefined}>
          {node.display}
        </Typography.Text>
      );
    if (
      node.shape.array ||
      ["custom", "flags"].includes(node.shape.category) ||
      node.shape.nullable
    )
      return (
        <Button
          type="text"
          className="complex-value-link"
          aria-invalid={!node.valid}
          icon={node.valid ? undefined : <WarningOutlined />}
          onClick={() => void navigate(node.path)}
        >
          {node.display}
          <RightOutlined />
        </Button>
      );
    if (active && same(active, node.path))
      return (
        <LeafInput
          node={node}
          authority={view!}
          update={(value) => {
            typing.current = value;
          }}
          commit={commit}
          enumSelect={(symbol) =>
            void change(node.path, { kind: "enum", symbol })
          }
        />
      );
    return (
      <Button
        type="text"
        className="complex-value-link"
        aria-invalid={!node.valid}
        icon={node.valid ? undefined : <WarningOutlined />}
        data-value-path={JSON.stringify(node.path)}
        aria-label={`Edit ${node.label}`}
        onClick={() => setActive(node.path)}
      >
        {node.display}
      </Button>
    );
  };
  const root =
    target &&
    view?.source === target.source &&
    view.row === target.row &&
    same(view.node.path, target.path)
      ? view.node
      : undefined;
  const stale =
    !!view &&
    (resolvedTarget.current !== target ||
      view.revision !== s.projection?.revision ||
      view.generation !== s.projection?.generation);
  useLayoutEffect(() => {
    const restore = restoreFocus.current;
    const validRestore =
      restore?.target === target && restore.intent === desktop.inputIntent;
    if (restore && !validRestore) restoreFocus.current = null;
    if (
      !target ||
      !root ||
      loading ||
      stale ||
      blocked ||
      (!validRestore &&
        target.inputIntent !== undefined &&
        target.inputIntent !== desktop.inputIntent) ||
      (focusedTarget.current === target && !restoreFocus.current)
    )
      return;
    const container = body.current;
    if (!container) return;
    if (focusedTarget.current === target && !validRestore) return;
    const focusPath =
      validRestore && restore.path
        ? restore.path
        : target.focusPath
          ? JSON.stringify(target.focusPath)
          : undefined;
    const requested = focusPath
      ? [...container.querySelectorAll<HTMLElement>("[data-value-path]")].find(
          (element) => element.dataset.valuePath === focusPath,
        )
      : undefined;
    const within = requested ?? container;
    const control = within.querySelector<HTMLElement>(
      "input:not(:disabled),button:not(:disabled),[role=combobox]:not([aria-disabled=true])",
    );
    const focus =
      validRestore && restore.element.isConnected
        ? restore.element
        : (control ?? requested);
    if (focus) {
      focus.focus({ preventScroll: true });
      if (
        document.activeElement === focus ||
        focus.contains(document.activeElement)
      ) {
        focusedTarget.current = target;
        restoreFocus.current = null;
      }
    }
  }, [target, root, loading, stale, blocked]);
  const arrayItems = (invalidFlag = false) =>
    root?.children
      .filter((child) => !invalidFlag || !child.valid)
      .map((child, index) => (
        <div
          key={child.path.at(-1)}
          className="complex-item"
          data-item={child.path.at(-1)}
        >
          <button
            className="spatial-handle element-grip"
            title="Reorder element"
            aria-label={`Reorder element ${child.label}`}
            disabled={!root?.editable || !root.shape?.array}
            onPointerDown={(e) =>
              view &&
              beginArrayDrag(
                e,
                view,
                child,
                (item, index) =>
                  change(root!.path, { kind: "place", item, index }),
                (path) => desktop.complexPath(root!.path, root!.start, path),
              )
            }
            onClick={(e) => {
              if (consumeSpatialClick(e.detail)) return;
              const r = e.currentTarget.getBoundingClientRect();
              setMenu({ child, index, x: r.left, y: r.bottom });
            }}
          >
            <HolderOutlined />
          </button>
          <span className="element-number">{child.label}</span>
          <div
            className="element-value complex-control"
            data-value-path={JSON.stringify(child.path)}
            tabIndex={-1}
          >
            {invalidFlag ? (
              <Tag color="warning">{child.display}</Tag>
            ) : (
              control(child)
            )}
          </div>
          <button
            className="spatial-handle"
            aria-label={`${child.label} actions`}
            title="Element actions"
            onClick={(e) => {
              const r = e.currentTarget.getBoundingClientRect();
              setMenu({ child, index, x: r.left - 150, y: r.bottom });
            }}
          >
            <MoreOutlined />
          </button>
        </div>
      ));
  const items: MenuProps["items"] = root?.shape?.array
    ? [
        {
          key: "up",
          label: "Move Up",
          icon: <ArrowUpOutlined />,
          disabled: root.start + (menu?.index ?? 0) === 0,
        },
        {
          key: "down",
          label: "Move Down",
          icon: <ArrowDownOutlined />,
          disabled: root.start + (menu?.index ?? 0) + 1 === root.totalChildren,
        },
        { type: "divider" },
        {
          key: "remove",
          label: "Remove",
          icon: <DeleteOutlined />,
          danger: true,
        },
      ]
    : [
        {
          key: "remove",
          label: "Remove",
          icon: <DeleteOutlined />,
          danger: true,
        },
      ];
  return (
    <Drawer
      title={
        <Space>
          {target && target.path.length > 1 && (
            <Button
              type="text"
              icon={<LeftOutlined />}
              aria-label="Parent value"
              onClick={() => void navigate(target.path.slice(0, -1))}
            />
          )}
          <span>{root?.label ?? target?.root ?? "Complex value"}</span>
        </Space>
      }
      open={!!target && (!s.pending || s.externalPending) && target.epoch === s.status.epoch}
      onClose={() => void close()}
      keyboard={false}
      mask={false}
      size={400}
      getContainer={() => document.getElementById("grid-area")!}
      rootStyle={{ position: "absolute" }}
      classNames={{ body: "complex-body" }}
      destroyOnHidden
      autoFocus={false}
    >
      <div
        ref={body}
        onKeyDown={keydown}
        inert={loading || stale || blocked}
        aria-busy={loading || stale || blocked}
      >
        {error ? (
          <Alert
            type="error"
            showIcon
            title="値を取得できません"
            description={error}
          />
        ) : loading && !root ? (
          <Spin size="small" />
        ) : (
          root && (
            <>
              {root.problem && (
                <Alert
                  type="warning"
                  showIcon
                  title={root.problem.message}
                  className="value-problem"
                />
              )}
              {root.shape?.nullable && (
                <Checkbox
                  aria-label={`${root.label} Null`}
                  disabled={!root.editable}
                  checked={root.kind === "null" && !nonNull}
                  onChange={(e) => {
                    if (e.target.checked)
                      void change(root.path, { kind: "null" });
                    else if (
                      root.shape?.array ||
                      ["custom", "flags"].includes(root.shape?.category ?? "")
                    )
                      void change(root.path, { kind: "materialize" });
                    else {
                      setNonNull(true);
                      setActive(root.path);
                    }
                  }}
                >
                  Null
                </Checkbox>
              )}
              {!root.shape || !root.editable ? (
                <Typography.Paragraph type="secondary">
                  {root.display} · {root.reason}
                </Typography.Paragraph>
              ) : root.shape.array && root.kind === "sequence" ? (
                <>
                  {arrayItems()}
                  <Button
                    type="text"
                    icon={<PlusOutlined />}
                    onClick={() => {
                      const previous = target,
                        intent = desktop.inputIntent;
                      void change(root.path, { kind: "add" }).then(
                        async (ok) => {
                          if (
                            !ok ||
                            intent !== desktop.inputIntent ||
                            desktop.interaction.complex !== previous ||
                            !previous
                          )
                            return;
                          const start =
                            Math.floor(root.totalChildren / 64) * 64;
                          const next = await desktop.complexView({
                            ...previous,
                            start,
                          });
                          if (
                            next &&
                            intent === desktop.inputIntent &&
                            desktop.interaction.complex === previous &&
                            next.generation ===
                              desktop.surface.projection?.generation
                          ) {
                            const added =
                              next.node.children[root.totalChildren - start];
                            desktop.complexPath(root.path, start, added?.path);
                          }
                        },
                      );
                    }}
                  >
                    Element
                  </Button>
                </>
              ) : root.shape.category === "flags" &&
                !root.shape.array &&
                root.kind === "sequence" ? (
                <>
                  <div className="flags-members">
                    {root.shape.members?.map((symbol) => (
                      <Checkbox
                        key={symbol}
                        checked={root.selectedMembers.includes(symbol)}
                        onChange={(e) =>
                          void change(root.path, {
                            kind: "flag",
                            symbol,
                            enabled: e.target.checked,
                          })
                        }
                      >
                        {symbol}
                      </Checkbox>
                    ))}
                  </div>
                  {arrayItems(true)}
                </>
              ) : root.shape.category === "custom" &&
                root.kind === "mapping" ? (
                <div className="custom-fields">
                  {root.children.map((child) => (
                    <div key={child.label} className="complex-field">
                      <Typography.Text type="secondary">
                        {child.label}
                      </Typography.Text>
                      <div
                        className="complex-control"
                        data-value-path={JSON.stringify(child.path)}
                        tabIndex={-1}
                      >
                        {control(child)}
                      </div>
                    </div>
                  ))}
                </div>
              ) : root.shape.array ||
                root.shape.category === "custom" ||
                root.shape.category === "flags" ? (
                <Button
                  icon={<PlusOutlined />}
                  onClick={() =>
                    void change(root.path, { kind: "materialize" })
                  }
                >
                  {root.shape.array
                    ? "Arrayを作成"
                    : root.shape.category === "flags"
                      ? "Flagsを作成"
                      : "Custom valueを作成"}
                </Button>
              ) : root.kind !== "null" || nonNull || !root.shape.nullable ? (
                <div
                  className="complex-control"
                  data-value-path={JSON.stringify(root.path)}
                  tabIndex={-1}
                >
                  <LeafInput
                    key={JSON.stringify(root.path)}
                    node={root}
                    authority={view!}
                    nonNull={nonNull}
                    update={(value) => {
                      typing.current = value;
                    }}
                    commit={commit}
                    enumSelect={(symbol) =>
                      void change(root.path, { kind: "enum", symbol })
                    }
                  />
                </div>
              ) : null}
              {root.totalChildren > 64 && (
                <div className="complex-pages">
                  <Button
                    icon={<LeftOutlined />}
                    aria-label="Previous elements"
                    disabled={root.start === 0}
                    onClick={() =>
                      void navigate(root.path, Math.max(0, root.start - 64))
                    }
                  />
                  <Typography.Text type="secondary">
                    {root.start + 1}–
                    {Math.min(root.totalChildren, root.start + 64)} /{" "}
                    {root.totalChildren}
                  </Typography.Text>
                  <Button
                    icon={<RightOutlined />}
                    aria-label="Next elements"
                    disabled={root.start + 64 >= root.totalChildren}
                    onClick={() => void navigate(root.path, root.start + 64)}
                  />
                </div>
              )}
            </>
          )
        )}
      </div>
      <Dropdown
        trigger={[]}
        open={!!menu}
        autoFocus
        onOpenChange={(open) => {
          if (!open) setMenu(null);
        }}
        menu={{
          items,
          onClick: ({ key }) => {
            if (!menu || !root) return;
            const item = menu.child.path.at(-1);
            const previous = target,
              path = menu.child.path;
            const destination = Math.max(
              0,
              root.start + menu.index + (key === "up" ? -1 : 1),
            );
            setMenu(null);
            void change(
              root.path,
              key === "remove"
                ? { kind: "remove", item }
                : { kind: "nudge", item, delta: key === "up" ? -1 : 1 },
            ).then((ok) => {
              if (ok && desktop.interaction.complex === previous)
                desktop.complexPath(
                  root.path,
                  key === "remove"
                    ? root.start
                    : Math.floor(destination / 64) * 64,
                  key === "remove" ? undefined : path,
                );
            });
          },
        }}
      >
        <span
          className="menu-anchor"
          style={{ position: "fixed", left: menu?.x ?? 0, top: menu?.y ?? 0 }}
          aria-hidden="true"
        />
      </Dropdown>
    </Drawer>
  );
}
function LeafInput({
  node,
  authority,
  nonNull = false,
  update,
  commit,
  enumSelect,
}: {
  node: EditorNode;
  authority: ComplexView;
  nonNull?: boolean;
  update: (typing: Typing) => void;
  commit: () => Promise<boolean>;
  enumSelect: (symbol: string) => void;
}) {
  const input = useRef<InputRef>(null),
    initial = useRef(nonNull ? "" : (node.input ?? "")),
    value = useRef(initial.current),
    composing = useRef(false),
    observed = useRef(authority);
  const [text, set] = useState(initial.current);
  const report = (force = false) =>
    update({
      authority: observed.current,
      path: node.path,
      initial: force ? "\u0000" : initial.current,
      text: value.current,
      composing: composing.current,
      label: node.label,
      cancel: () => {
        value.current = initial.current;
        set(initial.current);
      },
    });
  useLayoutEffect(() => {
    input.current?.focus({ cursor: "all" });
  }, []);
  useLayoutEffect(() => {
    const next = nonNull ? "" : (node.input ?? "");
    // A fresh committed value/Undo updates a clean editor. A background
    // interpretation update cannot discard unfinished input.
    if (value.current === initial.current || value.current === next) {
      initial.current = next;
      value.current = next;
      set(next);
      observed.current = authority;
    }
  }, [node.input, nonNull, authority]);
  if (node.shape?.category === "enum")
    return (
      <Select
        autoFocus
        aria-label={node.label}
        value={node.input ?? undefined}
        placeholder={node.display}
        options={node.shape.members?.map((value) => ({ value, label: value }))}
        onChange={enumSelect}
        className="complex-enum"
        status={node.valid ? undefined : "error"}
      />
    );
  return (
    <div className="complex-input">
      <Input
        ref={input}
        value={text}
        aria-label={node.label}
        status={node.valid ? undefined : "error"}
        onChange={(e) => {
          value.current = e.target.value;
          set(e.target.value);
          report();
        }}
        onCompositionStart={() => {
          composing.current = true;
          report();
        }}
        onCompositionEnd={() => {
          composing.current = false;
          report();
        }}
        onBlur={() => { if (!desktop.surface.externalPending) void commit(); }}
        onKeyDown={(e) => {
          if (e.nativeEvent.isComposing || composing.current) return;
          if (e.key === "Enter") {
            e.preventDefault();
            e.stopPropagation();
            void commit();
          } else if (e.key === "Tab") void commit();
        }}
      />
      {nonNull && (
        <Button
          onClick={() => {
            report(true);
            void commit();
          }}
        >
          入力を確定
        </Button>
      )}
    </div>
  );
}
