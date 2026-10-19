import React, { useEffect, useMemo, useRef, useState } from "react";
import { Button, Dropdown, Empty, Input, Tree } from "antd";
import type { DataNode } from "antd/es/tree";
import {
  DeleteOutlined,
  EditOutlined,
  FolderAddOutlined,
  FolderOutlined,
  PlusOutlined,
  SearchOutlined,
} from "@ant-design/icons";
import { type Snapshot, type Source, type SavedFile } from "../core/types";
import { session } from "../core/session";
export { SourceIcon } from "./source-icon";
import { SourceKindIcon, SourceIcon } from "./source-icon";
import { useEditing } from "./editing";
export function Explorer({
  snapshot,
  active,
  onSelect,
  fail,
  saved,
}: {
  snapshot: Snapshot;
  active: string;
  onSelect: (id: string, pin?: boolean) => void;
  fail: (error: unknown) => void;
  saved: Map<string, SavedFile>;
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
  const container = useRef<HTMLDivElement>(null);
  const treeRef = useRef<any>(null);
  const dropTarget = useRef<{
    folder: string;
    row: HTMLElement;
    inside: boolean;
    valid: boolean;
  } | null>(null);
  const dragSource = useRef("");
  const clearDrop = () => {
    dropTarget.current?.row.classList.remove("folder-drop-target");
    dropTarget.current = null;
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
    const row = (event.target as HTMLElement).closest<HTMLElement>(
      ".ant-tree-treenode",
    );
    const pathNode = row?.querySelector<HTMLElement>("[data-path]");
    if (!row || !pathNode) {
      clearDrop();
      return;
    }
    const path = pathNode.dataset.path!,
      box = row.getBoundingClientRect();
    const inside =
      allPaths.folders.has(path) &&
      event.clientY > box.top + 3 &&
      event.clientY < box.bottom - 3;
    const folder = inside ? path : path.split("/").slice(0, -1).join("/");
    clearDrop();
    const destination = [folder, dragSource.current.split("/").at(-1)]
      .filter(Boolean)
      .join("/");
    const valid =
      !!dragSource.current &&
      folder !== dragSource.current &&
      !folder.startsWith(`${dragSource.current}/`) &&
      (destination === dragSource.current ||
        (!allPaths.sources.has(destination) &&
          !allPaths.folders.has(destination)));
    dropTarget.current = { folder, row, inside, valid };
    if (inside && valid) row.classList.add("folder-drop-target");
  };
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
              <Dropdown
                trigger={["contextMenu"]}
                menu={{
                  items: [
                    ...(item.folder
                      ? [...createItems, { type: "divider" as const }]
                      : []),
                    {
                      key: "rename",
                      icon: <EditOutlined />,
                      label: "名前を変更",
                    },
                    {
                      key: "delete",
                      icon: <DeleteOutlined />,
                      label: "削除",
                      danger: true,
                    },
                  ],
                  onClick: ({ key, domEvent }) => {
                    domEvent.stopPropagation();
                    run(() => {
                      if (key === "rename")
                        editing.run(() => {
                          rename(item.path);
                          return {
                            focus: `node-${item.source?.id ?? item.path}`,
                          };
                        });
                      else if (key === "delete") remove([item.path]);
                      else
                        add(
                          key as any,
                          item.folder
                            ? item.path
                            : item.path.split("/").slice(0, -1).join("/"),
                        );
                    });
                  },
                }}
              >
                <span
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
              </Dropdown>
            );
          return [
            {
              key: item.folder ? `folder:${item.path}` : item.source!.id,
              title,
              icon: item.folder ? (
                <FolderOutlined />
              ) : (
                <SourceIcon source={item.source} />
              ),
              isLeaf: !item.folder,
              children: nested,
              path: item.path,
            } as DataNode,
          ];
        });
    };
    return build("");
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
    <aside className="explorer">
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
        onDragOverCapture={proposeDrop}
        onDragLeave={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget as Node))
            clearDrop();
        }}
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
        {tree.length ? (
          <Tree
            ref={treeRef}
            aria-label="プロジェクトの項目"
            blockNode
            showIcon
            virtual
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
            draggable={{ icon: false }}
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
            onDragStart={(info) => {
              dragSource.current = (info.node as any).path;
            }}
            onDragEnd={() => {
              clearDrop();
              dragSource.current = "";
            }}
            allowDrop={({ dragNode, dropNode }) => {
              const from = (dragNode as any).path,
                target = (dropNode as any).path;
              return (
                (dropTarget.current?.valid ?? true) &&
                target !== from &&
                !target.startsWith(`${from}/`)
              );
            }}
            dropIndicatorRender={(props) =>
              dropTarget.current?.inside ? null : (
                <div
                  className="ant-tree-drop-indicator"
                  style={{
                    left: props.dropLevelOffset * props.indent + 16,
                    right: 0,
                    [props.dropPosition === -1 ? "top" : "bottom"]: -3,
                  }}
                />
              )
            }
            onDrop={(info) => {
              proposeDrop(info.event);
              if (dropTarget.current?.valid === false) {
                clearDrop();
                return;
              }
              const from = (info.dragNode as any).path as string;
              const target = (info.node as any).path as string;
              const folder =
                dropTarget.current?.folder ??
                (String(info.node.key).startsWith("folder:") && !info.dropToGap
                  ? target
                  : target.split("/").slice(0, -1).join("/"));
              clearDrop();
              run(() => {
                movePath(
                  from,
                  [folder, from.split("/").at(-1)].filter(Boolean).join("/"),
                );
                setExpanded((old) => [
                  ...new Set([...old, `folder:${folder}`]),
                ]);
              });
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
    </aside>
  );
}
