import { cloneElement, type HTMLAttributes, type ReactElement } from "react";
import { App, Dropdown, type MenuProps } from "antd";
import { CopyOutlined, DeleteOutlined, EditOutlined } from "@ant-design/icons";
import { session } from "../core/session";

export interface FileOperations {
  rename(path: string): void;
  remove(paths: string[]): void;
}
export function sourcePath(path: string) {
  const root = session.getState().project?.root;
  if (!root) throw new Error("プロジェクトが開かれていません。");
  const separator = root.includes("\\") ? "\\" : "/";
  return `${root.replace(/[\\/]$/, "")}${separator}sources${separator}${path.replaceAll("/", separator)}`;
}
async function copyPath(path: string) {
  const value = sourcePath(path);
  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(value);
      return;
    } catch {}
  }
  // WKWebView custom origins may not expose the asynchronous Clipboard API.
  // Keep the fallback inside the user's menu command and restore editor focus.
  const focused = document.activeElement as HTMLElement | null;
  const field = document.createElement("textarea");
  field.value = value;
  field.readOnly = true;
  Object.assign(field.style, {
    position: "fixed",
    left: "0",
    top: "0",
    opacity: "0",
  });
  document.body.append(field);
  field.select();
  const copied = document.execCommand("copy");
  field.remove();
  focused?.focus({ preventScroll: true });
  if (!copied) throw new Error("パスをコピーできませんでした。");
}
export function FileContextMenu({
  path,
  operations,
  createItems,
  create,
  fail,
  children,
  ...accessibility
}: {
  path: string;
  operations: FileOperations;
  createItems?: MenuProps["items"];
  create?: (kind: string) => void;
  fail: (error: unknown) => void;
  children: ReactElement<HTMLAttributes<HTMLElement>>;
  "aria-selected"?: boolean;
  tabIndex?: number;
}) {
  const { message } = App.useApp();
  return (
    <Dropdown
      trigger={["contextMenu"]}
      menu={{
        items: [
          ...(createItems
            ? [...createItems, { type: "divider" as const }]
            : []),
          { key: "rename", label: "名前を変更", icon: <EditOutlined aria-hidden /> },
          { key: "copy-path", label: "パスをコピー", icon: <CopyOutlined aria-hidden /> },
          {
            key: "delete",
            label: "削除",
            icon: <DeleteOutlined aria-hidden />,
            danger: true,
          },
        ],
        onClick: ({ key, domEvent }) => {
          domEvent.stopPropagation();
          try {
            if (key === "rename") operations.rename(path);
            else if (key === "delete") operations.remove([path]);
            else if (key === "copy-path")
              void copyPath(path)
                .then(() => message.success("パスをコピーしました"))
                .catch(fail);
            else create?.(key);
          } catch (error) {
            fail(error);
          }
        },
      }}
    >
      {cloneElement(children, accessibility)}
    </Dropdown>
  );
}
