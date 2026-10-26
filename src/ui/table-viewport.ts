import { useLayoutEffect, useState, type RefObject } from "react";
import { flushSync } from "react-dom";

export function useTableViewport(
  root: RefObject<HTMLElement | null>,
  table: RefObject<any>,
  sourceId?: string,
  hasRows = true,
) {
  const [height, setHeight] = useState(400);
  useLayoutEffect(() => {
    const element = root.current;
    const header =
      root.current?.querySelector<HTMLElement>(".ant-table-header");
    // AntD swaps the virtual holder for a native body when results become empty.
    // Rebind and remeasure across that transition, not just source changes.
    const body = root.current?.querySelector<HTMLElement>(
      ".ant-table-tbody-virtual-holder, .ant-table-body",
    );
    if (!element || !header || !body) return;
    const measure = () => {
      setHeight(Math.max(
        1,
        element.getBoundingClientRect().bottom - body.getBoundingClientRect().top,
      ));
    };
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    observer.observe(header);
    measure();
    let synchronizing = false;
    const scrollTo = (left: number) => {
      if (synchronizing) return;
      synchronizing = true;
      // AntD's virtual table forwards positional scroll configs to its virtual
      // list. Native focus/scrollIntoView instead scrolls the DOM holder, bypassing
      // that offset and separating body cells from headers. Fold the native delta
      // into the virtual offset before the next paint (covered by focus regression).
      try {
        // Imperative virtual scroll does not emit onVirtualScroll; synchronize
        // the native header explicitly as well as the virtual body.
        header.scrollLeft = left;
        if (body.matches(".ant-table-body")) body.scrollLeft = header.scrollLeft;
        else flushSync(() => table.current?.scrollTo({ left: header.scrollLeft }));
      } finally {
        synchronizing = false;
      }
    };
    const onScroll = (event: Event) => {
      const target = event.target;
      if (!(target instanceof HTMLElement) || synchronizing) return;
      if (target === header) {
        scrollTo(header.scrollLeft);
        return;
      }
      if (body.matches(".ant-table-body")) return;
      const row = body.querySelector("[data-row-key]");
      // The browser may choose the holder's anonymous overflow wrapper rather
      // than the holder itself. Capture ancestor scrolls, but leave text inputs
      // and other independently scrolling controls alone.
      const viewportScroll =
        target.contains(body) ||
        (row && body.contains(target) && target.contains(row));
      if (!viewportScroll || !target.scrollLeft) return;
      const left = header.scrollLeft + target.scrollLeft;
      target.scrollLeft = 0;
      scrollTo(left);
    };
    element.addEventListener("scroll", onScroll, true);
    return () => {
      observer.disconnect();
      element.removeEventListener("scroll", onScroll, true);
    };
  }, [root, table, sourceId, hasRows]);
  return height;
}
