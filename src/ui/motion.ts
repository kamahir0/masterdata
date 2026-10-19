import { useEffect, useLayoutEffect, useRef, type RefObject } from "react";
export const easeOutExpo = [0.16, 1, 0.3, 1] as const;
export const easeOutExpoCss = "cubic-bezier(.16, 1, .3, 1)";
export const quickTransition = { duration: 0.12, ease: easeOutExpo };
export const resultMotion = {
  enter: 280,
  move: 260,
  easing: "cubic-bezier(.22,.61,.36,1)",
};
export const deletionMotion = {
  duration: 300,
  easing: "cubic-bezier(.4,0,.2,1)",
};
export const resultTransition = {
  duration: resultMotion.enter / 1000,
  ease: [0.22, 0.61, 0.36, 1] as const,
};
const selector = "[data-row-key], [data-motion-key]";
const layoutSelector = `${selector}, [data-motion-layout]`;
const owned = new WeakMap<HTMLElement, Animation>();
const dropPositions = new WeakMap<HTMLElement, Map<string, DOMRect>>();
const keyOf = (element: HTMLElement) =>
  element.dataset.rowKey ??
  element.dataset.motionKey ??
  element.dataset.motionLayout!;
function offset(element: HTMLElement) {
  if (!owned.has(element)) return [0, 0];
  const value = getComputedStyle(element).translate.split(" ");
  return [parseFloat(value[0]) || 0, parseFloat(value[1]) || 0];
}
function positions(
  root: HTMLElement,
  layout = false,
  visualTarget = (node: HTMLElement) => node,
) {
  return new Map(
    [...root.querySelectorAll<HTMLElement>(layoutSelector)].map((node) => {
      const element = visualTarget(node);
      const box = element.getBoundingClientRect();
      const [x, y] = layout ? offset(element) : [0, 0];
      return [
        keyOf(node),
        new DOMRect(box.x - x, box.y - y, box.width, box.height),
      ] as const;
    }),
  );
}
function animate(
  element: HTMLElement,
  frames: Keyframe[],
  duration: number,
  easing = resultMotion.easing,
) {
  owned.get(element)?.cancel();
  const animation = element.animate(frames, {
    duration,
    easing,
  });
  owned.set(element, animation);
  const clear = () => {
    if (owned.get(element) === animation) owned.delete(element);
  };
  animation.onfinish = clear;
  animation.oncancel = clear;
}
export function finishLayoutMotion(
  root: HTMLElement,
  visualTarget = (node: HTMLElement) => node,
) {
  root.querySelectorAll<HTMLElement>(layoutSelector).forEach((node) => {
    const element = visualTarget(node);
    const animation = owned.get(element);
    animation?.finish();
    animation?.cancel();
    owned.delete(element);
  });
}
export function captureDropPositions(
  root: HTMLElement,
  extra?: [string, DOMRect],
) {
  const captured = positions(root);
  if (extra) captured.set(...extra);
  dropPositions.set(root, captured);
}
export function viewport(root: HTMLElement) {
  let box = root.getBoundingClientRect();
  let left = Math.max(0, box.left),
    right = Math.min(innerWidth, box.right);
  let top = Math.max(0, box.top),
    bottom = Math.min(innerHeight, box.bottom);
  for (let parent = root.parentElement; parent; parent = parent.parentElement) {
    const style = getComputedStyle(parent);
    if (
      /(auto|scroll|hidden|clip)/.test(
        style.overflow + style.overflowX + style.overflowY,
      )
    ) {
      box = parent.getBoundingClientRect();
      left = Math.max(left, box.left);
      right = Math.min(right, box.right);
      top = Math.max(top, box.top);
      bottom = Math.min(bottom, box.bottom);
    }
  }
  return { left, right, top, bottom };
}
function scrollPosition(root: HTMLElement) {
  const box = root.getBoundingClientRect();
  return (
    `${box.left},${box.width};` +
    [
      root,
      ...root.querySelectorAll<HTMLElement>(
        ".ant-table-header, .ant-table-body, .ant-table-tbody-virtual-holder, .ant-tree-list-holder",
      ),
      ...(root.closest<HTMLElement>(".editor-scroll")
        ? [root.closest<HTMLElement>(".editor-scroll")!]
        : []),
    ]
      .map((element) => `${element.scrollLeft},${element.scrollTop}`)
      .join(";")
  );
}

// Model keys say what was created; DOM mount alone also happens during virtual
// scrolling and folder expansion. Pending births bridge late virtual mounts and
// focus-driven scroll without replaying existing values or serializing drafts.
export function useLayoutMotion(
  root: RefObject<HTMLElement | null>,
  keys: readonly string[],
  options: {
    context?: unknown;
    layout?: boolean;
    enterOnGrowth?: boolean;
    visualTarget?: (node: HTMLElement) => HTMLElement;
  } = {},
) {
  const state = useRef({
    host: null as HTMLElement | null,
    known: new Set<string>(),
    before: new Map<string, DOMRect>(),
    nodes: new Map<string, HTMLElement>(),
    pending: new Map<string, number>(),
    entered: new Map<string, number>(),
    scroll: "",
    context: options.context,
    transition: null as { duration: number; easing: string } | null,
  });
  const attached = useRef<{ host: HTMLElement; dispose: () => void } | null>(
    null,
  );
  const signature = keys.join("\0");
  const reveal = () => {
    const current = state.current,
      host = root.current;
    if (
      !host ||
      !current.pending.size ||
      document.documentElement.dataset.reducedMotion === "true" ||
      host.querySelector(".sort-dragging")
    )
      return;
    current.entered.forEach((_, key) => {
      if (
        !current.pending.has(key) &&
        !current.pending.has(current.nodes.get(key)?.dataset.motionOwner ?? key)
      )
        current.entered.delete(key);
    });
    const clip = viewport(host);
    host.querySelectorAll<HTMLElement>(selector).forEach((node) => {
      const element = options.visualTarget?.(node) ?? node;
      const key = keyOf(node),
        owner = element.dataset.motionOwner ?? key;
      if (current.nodes.get(key) !== element) {
        const rect = element.getBoundingClientRect(),
          [x, y] = offset(element);
        current.nodes.set(key, element);
        current.before.set(
          key,
          new DOMRect(rect.x - x, rect.y - y, rect.width, rect.height),
        );
      }
      const born = current.pending.get(owner);
      if (born === undefined || current.entered.get(key) === born) return;
      const row = element.parentElement?.closest<HTMLElement>("[data-row-key]");
      if (row && current.pending.has(row.dataset.rowKey!) && owned.has(row)) {
        // The parent already reveals these cells; don't fade them a second time
        // when a later virtual-list mutation is observed.
        current.entered.set(key, born);
        return;
      }
      const box = element.getBoundingClientRect();
      if (
        box.bottom <= clip.top ||
        box.top >= clip.bottom ||
        box.right <= clip.left ||
        box.left >= clip.right
      )
        return;
      current.entered.set(key, born);
      animate(
        element,
        [
          { opacity: 0, translate: "0px 6px" },
          { opacity: getComputedStyle(element).opacity, translate: "0px 0px" },
        ],
        resultMotion.enter,
      );
    });
  };
  const reconcile = () => {
    const host = root.current,
      current = state.current;
    if (!host || !current.transition) return;
    if (
      scrollPosition(host) !== current.scroll ||
      document.documentElement.dataset.reducedMotion === "true"
    ) {
      current.transition = null;
      return;
    }
    const next = positions(host, true, options.visualTarget);
    host.querySelectorAll<HTMLElement>(layoutSelector).forEach((node) => {
      const element = options.visualTarget?.(node) ?? node;
      const key = keyOf(node),
        old = current.before.get(key),
        box = next.get(key)!;
      current.nodes.set(key, element);
      if (!old) return;
      const moved = Math.abs(old.x - box.x) > 1 || Math.abs(old.y - box.y) > 1;
      const resize =
        node.dataset.motionSize === "height" &&
        Math.abs(old.height - box.height) > 1;
      if (!moved && owned.has(element)) {
        next.set(key, old);
        return;
      }
      if (!moved && !resize) return;
      const [dx, dy] = offset(element);
      const row = element.parentElement?.closest<HTMLElement>("[data-row-key]");
      const x = old.x + dx - box.x,
        y = row ? 0 : old.y + dy - box.y;
      if (
        Math.abs(y) >= 800 ||
        (Math.abs(x) <= 1 && Math.abs(y) <= 1 && !resize)
      )
        return;
      animate(
        element,
        [
          {
            translate: `${x}px ${y}px`,
            ...(resize ? { height: `${old.height}px` } : {}),
          },
          {
            translate: "0px 0px",
            ...(resize ? { height: `${box.height}px` } : {}),
          },
        ],
        current.transition!.duration,
        current.transition!.easing,
      );
    });
    current.before = next;
  };
  const latestReconcile = useRef(reconcile);
  latestReconcile.current = reconcile;
  const latestReveal = useRef(reveal);
  latestReveal.current = reveal;
  useLayoutEffect(() => {
    const host = root.current;
    if (!host) return;
    const current = state.current;
    const initialized = current.host === host;
    const now = performance.now(),
      nextKeys = new Set(keys);
    if (initialized) {
      nextKeys.forEach((key) => {
        if (
          !current.known.has(key) &&
          (!options.enterOnGrowth || nextKeys.size > current.known.size)
        )
          current.pending.set(key, now);
      });
      current.pending.forEach((_, key) => {
        if (!nextKeys.has(key)) current.pending.delete(key);
      });
    } else {
      current.pending.clear();
      current.entered.clear();
    }
    const captured = dropPositions.get(host);
    const before =
      captured ??
      new Map(
        [...current.before].map(([key, box]) => {
          const element = current.nodes.get(key);
          const [x, y] = element ? offset(element) : [0, 0];
          return [
            key,
            new DOMRect(box.x + x, box.y + y, box.width, box.height),
          ] as const;
        }),
      );
    current.nodes.forEach((element) => {
      owned.get(element)?.cancel();
      owned.delete(element);
    });
    dropPositions.delete(host);
    const scroll = scrollPosition(host);
    const movedViewport =
      scroll !== current.scroll || options.context !== current.context;
    const deleting = [...current.known].some((key) => !nextKeys.has(key));
    const timing = deleting
      ? deletionMotion
      : { duration: resultMotion.move, easing: resultMotion.easing };
    current.transition =
      initialized && !movedViewport && options.layout !== false ? timing : null;
    current.before = before;
    current.scroll = scroll;
    current.nodes = new Map(
      [...host.querySelectorAll<HTMLElement>(layoutSelector)].map((node) => [
        keyOf(node),
        options.visualTarget?.(node) ?? node,
      ]),
    );
    if (current.transition) reconcile();
    else current.before = positions(host, true, options.visualTarget);
    current.host = host;
    current.known = nextKeys;
    current.scroll = scroll;
    current.context = options.context;
    reveal();
  }, [signature, options.context]);
  useEffect(() => {
    const host = root.current;
    if (!host) return;
    if (document.documentElement.dataset.reducedMotion === "true") {
      finishLayoutMotion(host, options.visualTarget);
      state.current.pending.clear();
      state.current.entered.clear();
    }
    if (attached.current?.host === host) return;
    attached.current?.dispose();
    let frame = 0;
    const schedule = () => {
      // Virtual collections can reposition their children after the owner's
      // layout effect. Reconcile that DOM change before paint, using the same
      // operation baseline instead of silently losing the movement feedback.
      latestReconcile.current();
      if (!state.current.pending.size || frame) return;
      frame = requestAnimationFrame(() => {
        frame = 0;
        latestReveal.current();
      });
    };
    // Rendering may take longer than any fixed TTL (Linux WebKit's large-row
    // trace did). Retain creation until it can paint; a later user gesture ends
    // this feedback window, so ordinary scrolling never reveals stale births.
    const retire = () => {
      state.current.pending.clear();
      state.current.entered.clear();
      state.current.transition = null;
    };
    const gestures = ["pointerdown", "keydown", "wheel", "touchstart"] as const;
    gestures.forEach((event) =>
      document.addEventListener(event, retire, {
        capture: true,
        passive: true,
      }),
    );
    const observer = new MutationObserver(schedule);
    observer.observe(host, { childList: true, subtree: true });
    host.addEventListener("scroll", schedule, true);
    attached.current = {
      host,
      dispose: () => {
        observer.disconnect();
        gestures.forEach((event) =>
          document.removeEventListener(event, retire, true),
        );
        host.removeEventListener("scroll", schedule, true);
        cancelAnimationFrame(frame);
        finishLayoutMotion(host, options.visualTarget);
      },
    };
    latestReconcile.current();
    latestReveal.current();
  });
  useEffect(
    () => () => {
      attached.current?.dispose();
      attached.current = null;
    },
    [],
  );
  return {
    reveal,
    capture() {
      const host = root.current;
      if (!host) return;
      // Drawer opening, tree expansion and scrolling can change geometry without
      // changing model keys. An edit starts from what is actually visible now.
      state.current.host = host;
      state.current.known = new Set(keys);
      state.current.context = options.context;
      state.current.before = positions(host, true, options.visualTarget);
      state.current.nodes = new Map(
        [...host.querySelectorAll<HTMLElement>(layoutSelector)].map((node) => [
          keyOf(node),
          options.visualTarget?.(node) ?? node,
        ]),
      );
      state.current.scroll = scrollPosition(host);
    },
  };
}
