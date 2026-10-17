import { useLayoutEffect, type RefObject } from "react";
import { flushSync } from "react-dom";

export function useTableViewport(
  root: RefObject<HTMLElement | null>,
  table: RefObject<any>,
  sourceId?: string,
) {
  useLayoutEffect(() => {
    const header =
      root.current?.querySelector<HTMLElement>(".ant-table-header");
    const body = root.current?.querySelector<HTMLElement>(
      ".ant-table-tbody-virtual-holder",
    );
    if (!header || !body) return;
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
        flushSync(() => table.current?.scrollTo({ left: header.scrollLeft }));
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
    root.current!.addEventListener("scroll", onScroll, true);
    const element = root.current!;
    return () => element.removeEventListener("scroll", onScroll, true);
  }, [root, table, sourceId]);
}
