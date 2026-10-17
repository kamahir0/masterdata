import { useLayoutEffect, useRef, type RefObject } from "react";
export function useLayoutMotion(
  root: RefObject<HTMLElement | null>,
  revision: unknown,
) {
  const previous = useRef(new Map<string, DOMRect>());
  useLayoutEffect(() => {
    if (!root.current) return;
    const next = new Map<string, DOMRect>();
    root.current
      .querySelectorAll<HTMLElement>("[data-row-key], [data-motion-key]")
      .forEach((element) => {
        const key = element.dataset.rowKey ?? element.dataset.motionKey!;
        const rect = element.getBoundingClientRect();
        const old = previous.current.get(key);
        next.set(key, rect);
        if (!old || document.documentElement.dataset.reducedMotion === "true")
          return;
        const x = old.left - rect.left,
          y = old.top - rect.top;
        if ((Math.abs(x) > 1 || Math.abs(y) > 1) && Math.abs(y) < 800)
          element.animate(
            [{ translate: `${x}px ${y}px` }, { translate: "0px 0px" }],
            { duration: 170, easing: "cubic-bezier(.2,.7,.2,1)" },
          );
      });
    previous.current = next;
  }, [revision]);
}
