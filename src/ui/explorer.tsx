import React, { useEffect, useMemo, useRef, useState } from "react";
import { Button, Dropdown, Empty, Input, Tree } from "antd";
import type { DataNode } from "antd/es/tree";
import {
  ApartmentOutlined,
  DeleteOutlined,
  EditOutlined,
  FolderAddOutlined,
  FolderOutlined,
  FileTextOutlined,
  PlusOutlined,
  SearchOutlined,
  TableOutlined,
  TagOutlined,
  BranchesOutlined,
  DatabaseOutlined,
} from "@ant-design/icons";
import {
  type Snapshot,
  type Source,
  type SavedFile,
  typeKind,
} from "../core/types";
import { session } from "../core/session";
export function SourceIcon({ source }: { source?: Source }) {
  return source?.data?.kind === "schema" ? (
    <TableOutlined />
  ) : source?.data?.kind === "data" ? (
    <DatabaseOutlined />
  ) : typeKind(source) === "enum" || typeKind(source) === "flags" ? (
    <TagOutlined />
  ) : typeKind(source) === "custom" ? (
    <ApartmentOutlined />
  ) : typeKind(source) === "valueObject" ? (
    <BranchesOutlined />
  ) : (
    <FileTextOutlined />
  );
}
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
  const rename = (p: string) => {
    const ext = allPaths.sources.get(p)?.data?.kind
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
      session.move(
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
    run(() => {
      if (kind === "folder") {
        const path = session.addFolder(folder);
        setExpanded((old) => [...old, `folder:${folder}`]);
        rename(path);
      } else {
        const id = session.add(kind, folder);
        onSelect(id, true);
        setKeys([id]);
        const source = session
          .getState()
          .snapshot.sources.find((s) => s.id === id)!;
        rename(source.path);
      }
      setExpanded((old) => [
        ...old,
        ...folder
          .split("/")
          .map((_, i, p) => `folder:${p.slice(0, i + 1).join("/")}`),
      ]);
    });
  }
  const createItems = [
    { key: "table", icon: <TableOutlined />, label: "新しいテーブル" },
    { key: "enum", icon: <TagOutlined />, label: "新しいEnum" },
    { key: "flags", icon: <TagOutlined />, label: "新しいFlags" },
    { key: "custom", icon: <ApartmentOutlined />, label: "新しいCustom Type" },
    {
      key: "valueObject",
      icon: <BranchesOutlined />,
      label: "新しいValue Object",
    },
    { type: "divider" as const },
    { key: "folder", icon: <FolderAddOutlined />, label: "新しいフォルダ" },
  ];
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
                autoFocus
                aria-label="項目の名前"
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
                  onClick: ({ key }) =>
                    run(() => {
                      if (key === "rename") rename(item.path);
                      else if (key === "delete") session.remove([item.path]);
                      else
                        add(
                          key as any,
                          item.folder
                            ? item.path
                            : item.path.split("/").slice(0, -1).join("/"),
                        );
                    }),
                }}
              >
                <span
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
            rename(selectedPath);
          }
          if (e.key === "Delete" || e.key === "Backspace") {
            if (keys.length && !(e.metaKey || e.ctrlKey)) {
              e.preventDefault();
              session.remove(
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
            onDrop={(info) =>
              run(() => {
                const from = (info.dragNode as any).path as string;
                const target = (info.node as any).path as string;
                const isFolder = String(info.node.key).startsWith("folder:");
                const folder =
                  isFolder && !info.dropToGap
                    ? target
                    : target.split("/").slice(0, -1).join("/");
                session.move(
                  from,
                  [folder, from.split("/").at(-1)].filter(Boolean).join("/"),
                );
              })
            }
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
