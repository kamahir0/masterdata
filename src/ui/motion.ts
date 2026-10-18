import { useLayoutEffect, useRef, type RefObject } from "react";
export const easeOutExpo = [0.16, 1, 0.3, 1] as const;
export const easeOutExpoCss = "cubic-bezier(.16, 1, .3, 1)";
export const quickTransition = { duration: 0.12, ease: easeOutExpo };
const dropPositions = new WeakMap<HTMLElement, Map<string, DOMRect>>();
function positions(root: HTMLElement) {
  return new Map(
    [
      ...root.querySelectorAll<HTMLElement>(
        "[data-row-key], [data-motion-key]",
      ),
    ].map((element) => [
      element.dataset.rowKey ?? element.dataset.motionKey!,
      element.getBoundingClientRect(),
    ]),
  );
}
export function captureDropPositions(
  root: HTMLElement,
  extra?: [string, DOMRect],
) {
  const captured = positions(root);
  if (extra) captured.set(...extra);
  dropPositions.set(root, captured);
}
export function useLayoutMotion(
  root: RefObject<HTMLElement | null>,
  revision: unknown,
) {
  const previous = useRef(new Map<string, DOMRect>());
  const previousScroll = useRef("");
  const initialized = useRef(false);
  useLayoutEffect(() => {
    if (!root.current) return;
    const scroll = [
      ...root.current.querySelectorAll<HTMLElement>(
        ".ant-table-header, .ant-table-body, .ant-table-tbody-virtual-holder",
      ),
    ]
      .map((element) => `${element.scrollLeft},${element.scrollTop}`)
      .join(";");
    // Viewport movement is not a data reorder. Comparing against a pre-scroll
    // rectangle makes an ordinary switch edit appear to animate from scrollLeft=0.
    const scrolled = scroll !== previousScroll.current;
    const before = dropPositions.get(root.current) ?? previous.current;
    dropPositions.delete(root.current);
    const next = new Map<string, DOMRect>();
    root.current
      .querySelectorAll<HTMLElement>("[data-row-key], [data-motion-key]")
      .forEach((element) => {
        const key = element.dataset.rowKey ?? element.dataset.motionKey!;
        const rect = element.getBoundingClientRect();
        const old = before.get(key);
        const row =
          element.parentElement?.closest<HTMLElement>("[data-row-key]");
        next.set(key, rect);
        if (document.documentElement.dataset.reducedMotion === "true") return;
        if (!old) {
          if (initialized.current && (!row || before.has(row.dataset.rowKey!)))
            element.animate(
              [
                { opacity: 0, translate: "0px 5px" },
                { opacity: 1, translate: "0px 0px" },
              ],
              { duration: 140, easing: easeOutExpoCss },
            );
          return;
        }
        if (scrolled) return;
        const x = old.left - rect.left,
          // Row translation already carries its cells vertically. Cells own
          // horizontal column motion, otherwise a reorder would move them twice.
          y = row ? 0 : old.top - rect.top;
        if ((Math.abs(x) > 1 || Math.abs(y) > 1) && Math.abs(y) < 800)
          element.animate(
            [{ translate: `${x}px ${y}px` }, { translate: "0px 0px" }],
            { duration: 120, easing: easeOutExpoCss },
          );
      });
    initialized.current = true;
    previous.current = next;
    previousScroll.current = scroll;
  }, [revision]);
}
