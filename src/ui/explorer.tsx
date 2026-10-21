import React, {
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
} from "react";
import { Alert, Button, Dropdown, Empty, Input, Tree } from "antd";
import type { DataNode } from "antd/es/tree";
import {
  FolderAddOutlined,
  FolderOutlined,
  PlusOutlined,
  SearchOutlined,
} from "@ant-design/icons";
import { type Snapshot, type Source, type SavedFile } from "../core/types";
import { session } from "../core/session";
export { SourceIcon } from "./source-icon";
import { SourceKindIcon, SourceIcon } from "./source-icon";
import { FileContextMenu, type FileOperations } from "./file-context-menu";
import { useEditing } from "./editing";
export function Explorer({
  snapshot,
  active,
  onSelect,
  fail,
  saved,
  ref,
}: {
  snapshot: Snapshot;
  active: string;
  onSelect: (id: string, pin?: boolean) => void;
  fail: (error: unknown) => void;
  saved: Map<string, SavedFile>;
  ref?: React.Ref<FileOperations>;
}) {
  const [search, setSearch] = useState("");
  const [expanded, setExpanded] = useState<React.Key[]>([]);
  const [keys, setKeys] = useState<React.Key[]>([]);
  const [renaming, setRenaming] = useState<{
    path: string;
    value: string;
    ext: string;
  } | null>(null);
  const [height, setHeight] = useState(500);
  const [dropProblem, setDropProblem] = useState<string>();
  const container = useRef<HTMLDivElement>(null);
  const treeRef = useRef<any>(null);
  const dropTarget = useRef<{
    folder: string;
    row?: HTMLElement;
    inside: boolean;
    valid: boolean;
    reason?: string;
  } | null>(null);
  const dragSource = useRef("");
  const dropLine = useRef<HTMLDivElement>(null);
  const hover = useRef<{
    folder: string;
    timer: ReturnType<typeof setTimeout>;
  } | null>(null);
  const clearDropVisual = () => {
    dropTarget.current?.row?.classList.remove("folder-drop-target");
    container.current?.closest("aside")?.classList.remove("root-drop-target");
    if (dropLine.current) dropLine.current.hidden = true;
    dropTarget.current = null;
  };
  const clearDrop = () => {
    clearDropVisual();
    setDropProblem(undefined);
    if (hover.current) clearTimeout(hover.current.timer);
    hover.current = null;
  };
  useEffect(() => clearDrop, []);
  useEffect(() => {
    const observer = new ResizeObserver((entries) =>
      setHeight(entries[0].contentRect.height),
    );
    if (container.current) observer.observe(container.current);
    return () => observer.disconnect();
  }, []);
  const allPaths = useMemo(() => {
    const folders = new Set(snapshot.folders);
    snapshot.sources.forEach((s) => {
      const parts = s.path.split("/");
      parts.pop();
      while (parts.length) {
        folders.add(parts.join("/"));
        parts.pop();
      }
    });
    const sources = new Map(snapshot.sources.map((s) => [s.path, s]));
    const children = new Map<
      string,
      { path: string; folder: boolean; source?: Source }[]
    >();
    const add = (entry: { path: string; folder: boolean; source?: Source }) => {
      const parent = entry.path.split("/").slice(0, -1).join("/");
      const list = children.get(parent) ?? [];
      list.push(entry);
      children.set(parent, list);
    };
    folders.forEach((path) => add({ path, folder: true }));
    sources.forEach((source, path) => add({ path, folder: false, source }));
    return { folders, sources, children };
  }, [snapshot.sources, snapshot.folders]);
  const proposeDrop = (event: React.DragEvent) => {
    if (!dragSource.current) return;
    // Own the actual HTML drop as well as its indicator. Letting rc-tree run a
    // second ancestor/gap policy made highlighted destinations reject the drop.
    event.preventDefault();
    event.stopPropagation();
    const row =
      (event.target as HTMLElement).closest<HTMLElement>(
        ".ant-tree-treenode",
      ) ?? undefined;
    const pathNode = row?.querySelector<HTMLElement>("[data-path]");
    const path = pathNode?.dataset.path ?? "",
      box = row?.getBoundingClientRect();
    const inside =
      !!pathNode &&
      allPaths.folders.has(path) &&
      !!box &&
      event.clientY > box.top + 3 &&
      event.clientY < box.bottom - 3;
    const folder = inside ? path : path.split("/").slice(0, -1).join("/");
    const from = dragSource.current;
    const destination = [folder, from.split("/").at(-1)]
      .filter(Boolean)
      .join("/");
    const name = from.split("/").at(-1)!;
    const reason =
      folder === from || folder.startsWith(`${from}/`)
        ? `「${name}」自身や、その配下のフォルダへは移動できません。別のフォルダを選んでください。`
        : destination !== from &&
            (allPaths.sources.has(destination) || allPaths.folders.has(destination))
          ? `「${name}」が移動先「${folder || "ルートフォルダ"}」に既にあります。名前を変更するか、別のフォルダを選んでください。`
          : undefined;
    const valid = !reason;
    clearDropVisual();
    dropTarget.current = { folder, row, inside, valid, reason };
    // The OS may suppress drop entirely for a forbidden cursor. Explain the
    // same decision during hover, not only in a drop handler the user may never reach.
    setDropProblem(reason);
    event.dataTransfer.dropEffect = valid ? "move" : "none";
    if (valid && !row)
      container.current?.closest("aside")?.classList.add("root-drop-target");
    else if (valid && inside) row?.classList.add("folder-drop-target");
    else if (valid && box && dropLine.current && container.current) {
      const host = container.current.getBoundingClientRect();
      const icon = row!
        .querySelector(".ant-tree-iconEle")
        ?.getBoundingClientRect();
      Object.assign(dropLine.current.style, {
        top: `${(event.clientY < box.top + box.height / 2 ? box.top : box.bottom) - host.top}px`,
        left: `${(icon?.left ?? box.left) - host.left}px`,
      });
      dropLine.current.hidden = false;
    }
    const hoverFolder = valid && inside ? folder : "";
    if (hover.current?.folder !== hoverFolder) {
      if (hover.current) clearTimeout(hover.current.timer);
      hover.current = hoverFolder
        ? {
            folder: hoverFolder,
            timer: setTimeout(() => {
              setExpanded((old) => [
                ...new Set([...old, `folder:${hoverFolder}`]),
              ]);
            }, 220),
          }
        : null;
    }
  };
  const drop = (event: React.DragEvent) => {
    if (!dragSource.current) return;
    proposeDrop(event);
    const target = dropTarget.current,
      from = dragSource.current;
    clearDrop();
    dragSource.current = "";
    if (!target?.valid) {
      if (target?.reason) fail(new Error(target.reason));
      return;
    }
    const destination = [target.folder, from.split("/").at(-1)]
      .filter(Boolean)
      .join("/");
    if (destination === from) return;
    run(() => {
      movePath(from, destination);
      setExpanded((old) => [
        ...new Set([
          ...old,
          ...target.folder
            .split("/")
            .filter(Boolean)
            .map((_, i, p) => `folder:${p.slice(0, i + 1).join("/")}`),
        ]),
      ]);
    });
  };
  const dragProps = (path: string) => ({
    draggable: true,
    onDragStart: (event: React.DragEvent) => {
      clearDrop();
      dragSource.current = path;
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("application/x-masterdata-source", path);
    },
    onDragEnd: () => {
      clearDrop();
      dragSource.current = "";
    },
  });
  const selectedPath = keys[0]?.toString().startsWith("folder:")
    ? String(keys[0]).slice(7)
    : snapshot.sources.find((s) => s.id === keys[0])?.path;
  const parent = selectedPath
    ? allPaths.folders.has(selectedPath)
      ? selectedPath
      : selectedPath.split("/").slice(0, -1).join("/")
    : "";
  const run = (fn: () => void) => {
    try {
      fn();
    } catch (error) {
      fail(error);
    }
  };
  const movePath = (from: string, to: string) => {
    session.move(from, to);
    const remap = (key: React.Key) => {
      const path = String(key);
      return path === `folder:${from}` || path.startsWith(`folder:${from}/`)
        ? `folder:${to}${path.slice(`folder:${from}`.length)}`
        : key;
    };
    setKeys((old) => old.map(remap));
    setExpanded((old) => old.map(remap));
  };
  const rename = (p: string) => {
    const ext = session.getState().snapshot.sources.find((s) => s.path === p)
      ?.data?.kind
      ? (p.match(/\.ya?ml$/i)?.[0] ?? "")
      : "";
    setRenaming({
      path: p,
      value: p
        .split("/")
        .at(-1)!
        .slice(0, ext ? -ext.length : undefined),
      ext,
    });
  };
  const commitRename = () => {
    if (!renaming) return;
    run(() =>
      movePath(
        renaming.path,
        [
          ...renaming.path.split("/").slice(0, -1),
          renaming.value + renaming.ext,
        ].join("/"),
      ),
    );
    setRenaming(null);
  };
  function add(
    kind: "table" | "enum" | "flags" | "custom" | "valueObject" | "folder",
    folder = parent,
  ) {
    run(() =>
      editing.run(() => {
        setSearch("");
        let path: string, key: string;
        if (kind === "folder") {
          path = session.addFolder(folder);
          key = `folder:${path}`;
        } else {
          key = session.add(kind, folder);
          path = session
            .getState()
            .snapshot.sources.find((s) => s.id === key)!.path;
          onSelect(key, true);
        }
        setKeys([key]);
        setExpanded((old) => [
          ...new Set([
            ...old,
            ...folder
              .split("/")
              .filter(Boolean)
              .map((_, i, p) => `folder:${p.slice(0, i + 1).join("/")}`),
          ]),
        ]);
        rename(path);
        return { focus: `node-${key.startsWith("folder:") ? path : key}` };
      }),
    );
  }

  const createItems = [
    {
      key: "table",
      icon: <SourceKindIcon kind="table" />,
      label: "新しいテーブル",
    },
    { key: "enum", icon: <SourceKindIcon kind="enum" />, label: "新しいEnum" },
    {
      key: "flags",
      icon: <SourceKindIcon kind="flags" />,
      label: "新しいFlags",
    },
    {
      key: "custom",
      icon: <SourceKindIcon kind="custom" />,
      label: "新しいCustom Type",
    },
    {
      key: "valueObject",
      icon: <SourceKindIcon kind="valueObject" />,
      label: "新しいValue Object",
    },
    { type: "divider" as const },
    { key: "folder", icon: <FolderAddOutlined />, label: "新しいフォルダ" },
  ];
  const editing = useEditing(
    container,
    [
      ...snapshot.sources.map((source) => `node-${source.id}`),
      ...[...allPaths.folders].map((path) => `node-${path}`),
    ],
    {
      enterOnGrowth: true,
      visualTarget: (node) =>
        node.closest<HTMLElement>(".ant-tree-treenode") ?? node,
      reveal: (key) => {
        const id = key.slice(5);
        treeRef.current?.scrollTo({
          key: allPaths.folders.has(id) ? `folder:${id}` : id,
        });
      },
    },
  );
  const remove = (paths: string[]) => {
    const removed = [
      ...snapshot.sources
        .filter((s) =>
          paths.some((p) => s.path === p || s.path.startsWith(`${p}/`)),
        )
        .map((s) => `node-${s.id}`),
      ...[...allPaths.folders]
        .filter((p) =>
          paths.some((parent) => p === parent || p.startsWith(`${parent}/`)),
        )
        .map((p) => `node-${p}`),
    ];
    editing.run(() => session.remove(paths), removed);
    setKeys([]);
  };
  function beginRename(path: string) {
    setSearch("");
    const source = allPaths.sources.get(path);
    setKeys([source?.id ?? `folder:${path}`]);
    setExpanded((old) => [
      ...new Set([
        ...old,
        ...path
          .split("/")
          .slice(0, -1)
          .map((_, i, p) => `folder:${p.slice(0, i + 1).join("/")}`),
      ]),
    ]);
    editing.run(() => {
      rename(path);
      return { focus: `node-${source?.id ?? path}` };
    });
  }
  useImperativeHandle(ref, () => ({ rename: beginRename, remove }));
  const tree = useMemo(() => {
    const build = (folder: string): DataNode[] => {
      const children = [...(allPaths.children.get(folder) ?? [])];
      return children
        .sort(
          (a, b) =>
            Number(b.folder) - Number(a.folder) || a.path.localeCompare(b.path),
        )
        .flatMap((item) => {
          const nested = item.folder ? build(item.path) : undefined;
          const label = item.path.split("/").at(-1)!;
          const matches =
            `${label} ${item.source?.data?.name ?? ""} ${item.source?.data?.table ?? ""}`
              .toLowerCase()
              .includes(search.toLowerCase());
          if (search && !matches && !nested?.length) return [];
          const title =
            renaming?.path === item.path ? (
              <Input
                aria-label="項目の名前"
                data-path={item.path}
                data-motion-key={`node-${item.source?.id ?? item.path}`}
                size="small"
                value={renaming.value}
                onClick={(e) => e.stopPropagation()}
                onChange={(e) =>
                  setRenaming({ ...renaming, value: e.target.value })
                }
                onBlur={commitRename}
                onKeyDown={(e) => {
                  e.stopPropagation();
                  if (e.nativeEvent.isComposing) return;
                  if (e.key === "Enter") commitRename();
                  if (e.key === "Escape") setRenaming(null);
                }}
              />
            ) : (
              <FileContextMenu
                path={item.path}
                operations={{ rename: beginRename, remove }}
                fail={fail}
                createItems={item.folder ? createItems : undefined}
                create={(kind) => add(kind as any, item.path)}
              >
                <span
                  {...dragProps(item.path)}
                  data-motion-key={`node-${item.source?.id ?? item.path}`}
                  data-path={item.path}
                  onContextMenu={(event) => event.stopPropagation()}
                  className={`tree-name ${item.source?.syntaxError ? "tree-error" : ""}`}
                  title={item.path}
                >
                  <span className="tree-label">
                    {item.source?.data?.kind
                      ? label.replace(/\.ya?ml$/i, "")
                      : label}
                  </span>
                  {item.source && session.dirtySource(item.source) && (
                    <span className="dirty-dot" />
                  )}
                </span>
              </FileContextMenu>
            );
          return [
            {
              key: item.folder ? `folder:${item.path}` : item.source!.id,
              title,
              icon: <span className="tree-drag-icon" data-path={item.path} {...dragProps(item.path)}>
                {item.folder ? <FolderOutlined /> : <SourceIcon source={item.source} />}
              </span>,
              isLeaf: !item.folder,
              children: nested,
              path: item.path,
            } as DataNode,
          ];
        });
    };
    // A parent's branch ends at its last direct child, not that child's last
    // descendant. Carry only continuing sibling branches down the subtree.
    // Keep these styles on the node data so virtual row reuse / moves update
    // hierarchy and paint together, without a second DOM-observer lifecycle.
    const withGuides = (
      nodes: DataNode[],
      continuing: boolean[] = [],
      nested = false,
    ): DataNode[] =>
      nodes.map((node, index) => {
        const more = index < nodes.length - 1;
        const layers: string[] = [];
        const line = (x: number, y: number, width: number, height: number) =>
          layers.push(
            `linear-gradient(var(--tree-guide),var(--tree-guide)) ${x}px ${y}px / ${width}px ${height}px no-repeat`,
          );
        continuing.forEach((visible, level) => {
          if (visible) line(level * 16 + 8, 0, 1, 28);
        });
        if (nested) {
          const x = continuing.length * 16 + 8;
          line(x, 0, 1, more ? 28 : 14);
          // Folder elbows stop before the disclosure triangle; leaf elbows
          // cross its empty switcher slot and stop before the file icon.
          line(x, 14, node.isLeaf ? 21 : 10, 1);
        }
        return {
          ...node,
          style: {
            "--tree-guides": layers.join(",") || "none",
          } as React.CSSProperties,
          children: node.children && withGuides(
            node.children,
            nested ? [...continuing, more] : [],
            true,
          ),
        };
      });
    return withGuides(build(""));
  }, [allPaths, search, renaming, snapshot, saved]);
  useEffect(() => {
    if (!active) return;
    const source = snapshot.sources.find((s) => s.id === active);
    if (!source) return;
    setKeys([active]);
    const parts = source.path.split("/");
    parts.pop();
    setExpanded((old) => [
      ...new Set([
        ...old,
        ...parts.map((_, i) => `folder:${parts.slice(0, i + 1).join("/")}`),
      ]),
    ]);
  }, [active]);
  return (
    <aside
      className="explorer"
      onDragEnterCapture={proposeDrop}
      onDragOverCapture={proposeDrop}
      onDropCapture={drop}
      onDragLeave={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node))
          clearDrop();
      }}
    >
      <div className="explorer-heading">
        <span>エクスプローラー</span>
        <Dropdown
          menu={{ items: createItems, onClick: ({ key }) => add(key as any) }}
          trigger={["click"]}
        >
          <Button
            type="text"
            size="small"
            icon={<PlusOutlined />}
            aria-label="新しい項目"
          />
        </Dropdown>
      </div>
      <div className="explorer-search">
        <Input
          size="small"
          aria-label="表・型を検索"
          prefix={<SearchOutlined />}
          placeholder="表・型を検索"
          allowClear
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </div>
      <div
        className="tree-scroll"
        ref={container}
        onKeyDown={(e) => {
          if (e.key === "Escape" && !renaming) {
            // An open context menu owns the first Escape.
            if (
              [...document.querySelectorAll<HTMLElement>(".ant-dropdown")].some(
                (menu) => menu.offsetHeight > 0,
              )
            )
              return;
            e.preventDefault();
            e.stopPropagation();
            setKeys([]);
            return;
          }
          if ((e.target as HTMLElement).tagName === "INPUT") return;
          if (e.key === "F2" && selectedPath) {
            e.preventDefault();
            editing.run(() => {
              rename(selectedPath);
              return {
                focus: `node-${allPaths.sources.get(selectedPath)?.id ?? selectedPath}`,
              };
            });
          }
          if (e.key === "Delete" || e.key === "Backspace") {
            if (keys.length && !(e.metaKey || e.ctrlKey)) {
              e.preventDefault();
              remove(
                keys
                  .map((k) =>
                    String(k).startsWith("folder:")
                      ? String(k).slice(7)
                      : (snapshot.sources.find((s) => s.id === k)?.path ?? ""),
                  )
                  .filter(Boolean),
              );
            }
          }
        }}
      >
        <div
          ref={dropLine}
          className="explorer-drop-line"
          hidden
          aria-hidden="true"
        />
        {tree.length ? (
          <Tree
            ref={treeRef}
            aria-label="プロジェクトの項目"
            blockNode
            showIcon
            virtual
            // rc-tree reparents expansion rows when motion ends. An inline
            // editor must live in the final row from the start: replacing a
            // focused input drops focus without delivering its blur commit.
            motion={renaming ? false : undefined}
            height={height}
            treeData={tree}
            selectedKeys={keys}
            expandedKeys={
              search
                ? [...allPaths.folders].map((p) => `folder:${p}`)
                : expanded
            }
            onExpand={setExpanded}
            onDoubleClick={(_, node) => {
              const id = String(node.key);
              if (!id.startsWith("folder:")) onSelect(id, true);
            }}
            multiple
            onSelect={(selected, info) => {
              const event = info.nativeEvent;
              if (
                event.button === 2 ||
                (event.target as HTMLElement).closest("input, .ant-dropdown")
              )
                return;
              setKeys(
                event.ctrlKey || event.metaKey || event.shiftKey
                  ? selected
                  : [info.node.key],
              );
              const key = String(info.node.key);
              if (key.startsWith("folder:")) {
                setExpanded((old) =>
                  old.includes(key)
                    ? old.filter((k) => k !== key)
                    : [...old, key],
                );
              } else onSelect(key);
            }}
          />
        ) : (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={search ? "見つかりません" : "最初のテーブルを作成"}
          >
            <Button size="small" onClick={() => add("table", "")}>
              テーブルを追加
            </Button>
          </Empty>
        )}
      </div>
      {dropProblem && (
        <div className="explorer-drop-feedback">
          <Alert
            role="status"
            aria-live="polite"
            type="warning"
            showIcon
            title="移動できません"
            description={dropProblem}
          />
        </div>
      )}
    </aside>
  );
}
