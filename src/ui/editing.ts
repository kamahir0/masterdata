import { useEffect, useLayoutEffect, useRef, type RefObject } from "react";
import { deletionMotion, useLayoutMotion, viewport } from "./motion";
import { visualCopy } from "./visual-snapshot";

export interface EditResult {
  focus?: string;
}
const selector = "[data-row-key], [data-motion-key]";
const keyOf = (node: HTMLElement) =>
  node.dataset.rowKey ?? node.dataset.motionKey;

// Every collection uses the same completion lifecycle. Views provide only their
// reveal geometry (virtual rows/columns/tree); intent and cancellation stay here.
export function useEditing(
  root: RefObject<HTMLElement | null>,
  keys: readonly string[],
  options: {
    context?: unknown;
    layout?: boolean;
    enterOnGrowth?: boolean;
    reveal?: (key: string, node: HTMLElement | undefined) => void;
    focused?: (key: string) => void;
    input?: string;
    visualTarget?: (node: HTMLElement) => HTMLElement;
    clip?: (
      node: HTMLElement,
      clip: ReturnType<typeof viewport>,
    ) => ReturnType<typeof viewport>;
  } = {},
) {
  const motion = useLayoutMotion(root, keys, options);
  const pending = useRef<string | undefined>(undefined);
  const latest = useRef(options);
  latest.current = options;
  const exits = useRef(new Set<HTMLElement>());
  const discardExit = (layer: HTMLElement) => {
    layer.getAnimations().forEach((animation) => animation.cancel());
    layer.remove();
    exits.current.delete(layer);
  };
  const clearExits = () => exits.current.forEach(discardExit);
  const attached = useRef<{ host: HTMLElement; dispose: () => void } | null>(
    null,
  );
  const scope = useRef(options.context);
  const cancel = () => {
    pending.current = undefined;
  };
  const complete = () => {
    const key = pending.current,
      host = root.current;
    if (!key || !host) return;
    const node = [...host.querySelectorAll<HTMLElement>(selector)].find(
      (n) => keyOf(n) === key,
    );
    latest.current.reveal?.(key, node);
    if (!node) return;
    if (!latest.current.reveal)
      node.scrollIntoView({ block: "nearest", inline: "nearest" });
    // Reveal can synchronously scroll an offscreen header. Start its feedback
    // before handing over input; the next keystroke legitimately retires births
    // and must not race a deferred scroll/rAF callback (large WebKit regression).
    motion.reveal();
    const input = node.matches(
      "input,button,textarea,select,[contenteditable=true]",
    )
      ? node
      : node.querySelector<HTMLElement>(
          latest.current.input ??
            "input:not([disabled]), .entry:not([disabled]), button:not([disabled])",
        );
    if (!input) return;
    pending.current = undefined;
    input.focus({ preventScroll: true });
    if (input instanceof HTMLInputElement) input.select();
    latest.current.focused?.(key);
  };
  useLayoutEffect(() => {
    if (scope.current !== options.context) {
      cancel();
      clearExits();
      scope.current = options.context;
    }
    // Undo may restore a deleted item before its visual exit has finished.
    exits.current.forEach((layer) => {
      if (
        keys.includes(layer.dataset.exitKey!) ||
        document.documentElement.dataset.reducedMotion === "true"
      ) {
        discardExit(layer);
      }
    });
    complete();
  });
  useEffect(() => {
    const host = root.current;
    if (attached.current?.host === host) return;
    attached.current?.dispose();
    attached.current = null;
    if (!host) return;
    const observer = new MutationObserver(complete);
    observer.observe(host, { childList: true, subtree: true });
    const events = ["pointerdown", "keydown", "wheel", "touchstart"] as const;
    events.forEach((event) => document.addEventListener(event, cancel, true));
    attached.current = {
      host,
      dispose: () => {
        observer.disconnect();
        cancel();
        events.forEach((event) =>
          document.removeEventListener(event, cancel, true),
        );
        clearExits();
      },
    };
  });
  useEffect(() => () => attached.current?.dispose(), []);
  return {
    run(command: () => EditResult | void, removed: readonly string[] = []) {
      cancel();
      motion.capture();
      // Capture before mutation; saving and Undo see the deletion immediately.
      // Snapshot only mounted, visible targets, never the entire virtual model.
      clearExits();
      const host = root.current;
      if (
        host &&
        document.documentElement.dataset.reducedMotion !== "true" &&
        removed.length
      ) {
        const nodes = [...host.querySelectorAll<HTMLElement>(selector)];
        const targets = nodes.filter(
          (n) =>
            removed.includes(keyOf(n)!) ||
            removed.includes(n.dataset.motionOwner!),
        );
        const snapshots = targets
          .filter(
            (n) =>
              !targets.some((parent) => parent !== n && parent.contains(n)),
          )
          .map((node) => {
            const visual = latest.current.visualTarget?.(node) ?? node;
            return { node, visual, box: visual.getBoundingClientRect() };
          });
        snapshots.forEach(({ node, visual, box }) => {
          const clip =
            latest.current.clip?.(node, viewport(host)) ?? viewport(host);
          if (
            box.bottom <= clip.top ||
            box.top >= clip.bottom ||
            box.right <= clip.left ||
            box.left >= clip.right
          )
            return;
          const layer = document.createElement("div");
          layer.className = "edit-exit";
          layer.inert = true;
          layer.setAttribute("aria-hidden", "true");
          layer.dataset.exitKey = node.dataset.motionOwner ?? keyOf(node);
          const left = Math.max(box.left, clip.left),
            top = Math.max(box.top, clip.top);
          Object.assign(layer.style, {
            position: "fixed",
            pointerEvents: "none",
            overflow: "hidden",
            zIndex: "1100",
            background: getComputedStyle(host).getPropertyValue("--surface"),
            left: `${left}px`,
            top: `${top}px`,
            width: `${Math.min(box.right, clip.right) - left}px`,
            height: `${Math.min(box.bottom, clip.bottom) - top}px`,
          });
          const copy = visualCopy(visual, clip);
          Object.assign(copy.style, {
            position: "absolute",
            margin: "0",
            left: `${box.left - left}px`,
            top: `${box.top - top}px`,
            transform: "none",
            translate: "none",
          });
          layer.append(copy);
          document.body.append(layer);
          exits.current.add(layer);
          // Match the surviving items' FLIP: collapse the occupied region,
          // rather than laying a fading full-size copy over their destination.
          // Keep the outline visible early in the motion, including last-item
          // deletion where the trailing Add control is the only moving item.
          const horizontal = node.dataset.motionAxis === "x";
          const preceding = snapshots.reduce((size, other) => {
            if (
              other.node === node ||
              (other.node.dataset.motionAxis === "x") !== horizontal
            )
              return size;
            const previous = horizontal
              ? other.box.right <= box.left + 1 &&
                other.box.bottom > box.top &&
                other.box.top < box.bottom
              : other.box.bottom <= box.top + 1 &&
                other.box.right > box.left &&
                other.box.left < box.right;
            return (
              size +
              (previous ? (horizontal ? other.box.width : other.box.height) : 0)
            );
          }, 0);
          const animation = layer.animate(
            [
              { opacity: 1, clipPath: "inset(0 0 0 0)", translate: "0px 0px" },
              {
                opacity: 0,
                clipPath: horizontal
                  ? "inset(0 100% 0 0)"
                  : "inset(0 0 100% 0)",
                translate: horizontal
                  ? `${-preceding}px 0px`
                  : `0px ${-preceding}px`,
              },
            ],
            deletionMotion,
          );
          const cleanup = () => {
            layer.remove();
            exits.current.delete(layer);
          };
          animation.onfinish = cleanup;
          animation.oncancel = cleanup;
        });
      }
      try {
        pending.current = command()?.focus;
      } catch (error) {
        clearExits();
        throw error;
      }
    },
  };
}
