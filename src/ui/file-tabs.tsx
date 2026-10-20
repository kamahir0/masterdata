import {
  useLayoutEffect,
  useRef,
  cloneElement,
  type ReactElement,
} from "react";
import { Button, Tooltip } from "antd";
import { CloseOutlined } from "@ant-design/icons";
import type { Source } from "../core/types";
import {
  AnimatePresence,
  motion,
  usePresence,
  useAnimationControls,
  useReducedMotion,
} from "motion/react";
import { resultTransition } from "./motion";
import { FileContextMenu, type FileOperations } from "./file-context-menu";
import { SourceIcon } from "./explorer";

// AntD Tabs hides overflow items before layout. Keep every file tab mounted
// so the same strip can wrap; in-document modes still use AntD Tabs.
export function FileTabs({
  sources,
  active,
  preview,
  wrap,
  reduced,
  dirty,
  select,
  pin,
  close,
  operations,
  fail,
}: {
  sources: Source[];
  active: string;
  preview: string | null;
  wrap: boolean;
  reduced: boolean;
  dirty: (source: Source) => boolean;
  select: (id: string) => void;
  pin: (id: string) => void;
  close: (id: string) => void;
  operations: FileOperations;
  fail: (error: unknown) => void;
}) {
  const container = useRef<HTMLDivElement>(null);
  const reveal = () => {
    const host = container.current,
      tab = host?.querySelector<HTMLElement>('[aria-selected="true"]');
    if (!host || !tab) return;
    const box = tab.getBoundingClientRect(),
      clip = host.getBoundingClientRect();
    if (wrap) {
      if (box.bottom > clip.bottom) host.scrollTop += box.bottom - clip.bottom;
      else if (box.top < clip.top) host.scrollTop += box.top - clip.top;
    } else {
      if (box.right > clip.right) host.scrollLeft += box.right - clip.right;
      else if (box.left < clip.left) host.scrollLeft += box.left - clip.left;
    }
  };
  useLayoutEffect(reveal, [active, sources.length, wrap]);
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
            container.current?.querySelectorAll(
              '.file-tab-slot:not([inert]) [role="tab"]',
            )[next] as HTMLElement
          )?.focus();
        }
      }}
    >
      <AnimatePresence initial={false}>
        {sources.map((source) => {
          const name =
            source.path
              .split("/")
              .at(-1)
              ?.replace(/\.ya?ml$/, "") ?? source.path;
          const temporary = preview === source.id;
          return (
            <TabSlot
              key={source.id}
              reveal={reveal}
              reduced={reduced}
              selected={source.id === active}
            >
              <FileContextMenu
                path={source.path}
                operations={operations}
                fail={fail}
              >
                <div
                  data-motion-key={`tab-${source.id}`}
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
              </FileContextMenu>
            </TabSlot>
          );
        })}
      </AnimatePresence>
    </div>
  );
}

function TabSlot({
  children,
  reveal,
  reduced,
  selected,
}: {
  children: ReactElement<{ "aria-selected"?: boolean; tabIndex?: number }>;
  reveal: () => void;
  reduced: boolean;
  selected: boolean;
}) {
  const [present, safeToRemove] = usePresence();
  const controls = useAnimationControls();
  const systemReduced = useReducedMotion();
  const noMotion = reduced || !!systemReduced;
  const node = useRef<HTMLDivElement>(null);
  const remove = useRef(safeToRemove);
  remove.current = safeToRemove;
  useLayoutEffect(() => {
    const slot = node.current;
    if (!slot) return;
    if (present) {
      void controls.start({
        width: "auto",
        opacity: 1,
        transition: {
          ...resultTransition,
          duration: noMotion ? 0 : 0.2,
        },
      });
      return () => controls.stop();
    }
    controls.stop();
    if (noMotion) {
      remove.current?.();
      return;
    }
    // Auto-width measurement and compositor opacity used separate clocks.
    // One native animation keeps the departing tab visible while its space
    // collapses, even when the newly selected editor occupies the main thread.
    const animation = slot.animate(
      [
        {
          width: `${slot.getBoundingClientRect().width}px`,
          opacity: getComputedStyle(slot).opacity,
        },
        { width: "0px", opacity: 0 },
      ],
      { duration: 280, easing: "cubic-bezier(.4,0,.2,1)", fill: "both" },
    );
    let active = true;
    void animation.finished.then(
      () => {
        if (active) remove.current?.();
      },
      () => {},
    );
    return () => {
      active = false;
      const width = slot.getBoundingClientRect().width;
      const opacity = Number(getComputedStyle(slot).opacity);
      animation.cancel();
      // A re-open continues from the visible exit state instead of flashing
      // back to the old natural width. Unmount also cancels the owned effect.
      if (slot.isConnected) controls.set({ width, opacity });
    };
  }, [present, noMotion, controls]);
  return (
    <motion.div
      ref={node}
      className="file-tab-slot"
      data-exiting-active={!present && selected ? "true" : undefined}
      inert={!present}
      aria-hidden={!present || undefined}
      initial={noMotion ? false : { width: 0, opacity: 0 }}
      animate={controls}
      onUpdate={present ? reveal : undefined}
    >
      {present
        ? children
        : cloneElement(children, { "aria-selected": false, tabIndex: -1 })}
    </motion.div>
  );
}
