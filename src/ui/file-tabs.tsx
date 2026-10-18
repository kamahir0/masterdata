import { useLayoutEffect, useRef } from "react";
import { Button, Tooltip } from "antd";
import { CloseOutlined } from "@ant-design/icons";
import type { Source } from "../core/types";
import { SourceIcon } from "./explorer";

// AntD Tabs hides overflow items before layout. Keep every file tab mounted
// so the same strip can wrap; in-document modes still use AntD Tabs.
export function FileTabs({
  sources,
  active,
  preview,
  wrap,
  dirty,
  select,
  pin,
  close,
}: {
  sources: Source[];
  active: string;
  preview: string | null;
  wrap: boolean;
  dirty: (source: Source) => boolean;
  select: (id: string) => void;
  pin: (id: string) => void;
  close: (id: string) => void;
}) {
  const container = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    container.current
      ?.querySelector('[aria-selected="true"]')
      ?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [active, sources.length, wrap]);
  return (
    <div
      ref={container}
      className={`file-tabs ${wrap ? "wrap-tabs" : "scroll-tabs"}`}
      role="tablist"
      aria-label="開いているファイル"
      onKeyDown={(event) => {
        if ((event.target as HTMLElement).closest("button")) return;
        const index = sources.findIndex((s) => s.id === active);
        let next = index;
        if (event.key === "ArrowRight") next = (index + 1) % sources.length;
        else if (event.key === "ArrowLeft")
          next = (index + sources.length - 1) % sources.length;
        else if (event.key === "Home") next = 0;
        else if (event.key === "End") next = sources.length - 1;
        else return;
        event.preventDefault();
        if (sources[next]) {
          select(sources[next].id);
          (
            container.current?.querySelectorAll('[role="tab"]')[
              next
            ] as HTMLElement
          )?.focus();
        }
      }}
    >
      {sources.map((source) => {
        const name =
          source.path
            .split("/")
            .at(-1)
            ?.replace(/\.ya?ml$/, "") ?? source.path;
        const temporary = preview === source.id;
        return (
          <div
            key={source.id}
            role="tab"
            aria-label={name}
            aria-description={`${temporary ? "プレビュー。ダブルクリックで保持。" : ""}${dirty(source) ? "未保存の変更" : ""}`}
            aria-selected={source.id === active}
            tabIndex={source.id === active ? 0 : -1}
            className={`file-tab ${temporary ? "preview-tab" : ""}`}
            title={`${source.path}${temporary ? " — ダブルクリックでタブを保持" : ""}`}
            onClick={() => select(source.id)}
            onDoubleClick={() => pin(source.id)}
            onKeyDown={(event) => {
              if (
                event.target === event.currentTarget &&
                (event.key === "Enter" || event.key === " ")
              ) {
                event.preventDefault();
                select(source.id);
              }
            }}
          >
            <span className="file-tab-label">
              <SourceIcon source={source} />
              <span className="file-tab-name">{name}</span>
              {dirty(source) && (
                <span className="dirty-dot" aria-label="未保存の変更" />
              )}
            </span>
            <Tooltip title="ファイルを閉じる">
              <Button
                type="text"
                tabIndex={source.id === active ? 0 : -1}
                size="small"
                icon={<CloseOutlined />}
                aria-label={`${name}を閉じる`}
                onClick={(event) => {
                  event.stopPropagation();
                  close(source.id);
                }}
                onDoubleClick={(event) => event.stopPropagation()}
              />
            </Tooltip>
          </div>
        );
      })}
    </div>
  );
}
