import { useLayoutEffect, useRef, type RefObject } from "react";
export const easeOutExpo = [0.16, 1, 0.3, 1] as const;
export const easeOutExpoCss = "cubic-bezier(.16, 1, .3, 1)";
export const quickTransition = { duration: 0.12, ease: easeOutExpo };
export function useLayoutMotion(
  root: RefObject<HTMLElement | null>,
  revision: unknown,
) {
  const previous = useRef(new Map<string, DOMRect>());
  const previousScroll = useRef("");
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
    const next = new Map<string, DOMRect>();
    root.current
      .querySelectorAll<HTMLElement>("[data-row-key], [data-motion-key]")
      .forEach((element) => {
        const key = element.dataset.rowKey ?? element.dataset.motionKey!;
        const rect = element.getBoundingClientRect();
        const old = previous.current.get(key);
        next.set(key, rect);
        if (
          scrolled ||
          !old ||
          document.documentElement.dataset.reducedMotion === "true"
        )
          return;
        const x = old.left - rect.left,
          y = old.top - rect.top;
        if ((Math.abs(x) > 1 || Math.abs(y) > 1) && Math.abs(y) < 800)
          element.animate(
            [{ translate: `${x}px ${y}px` }, { translate: "0px 0px" }],
            { duration: 120, easing: easeOutExpoCss },
          );
      });
    previous.current = next;
    previousScroll.current = scroll;
  }, [revision]);
}
