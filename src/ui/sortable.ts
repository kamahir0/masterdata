import { useEffect, useRef, type RefObject } from "react";
import { flushSync } from "react-dom";
import { captureDropPositions, easeOutExpoCss, finishLayoutMotion } from "./motion";

// Preview is DOM-only: persistence, diagnostics and Undo see one edit at drop.
// Individual translate composes with the virtual table's own row positioning.
export function usePointerSort(
  root: RefObject<HTMLElement | null>,
  move: (group: string, from: string, to: string) => void,
) {
  const action = useRef(move);
  action.current = move;
  const attached = useRef<{ host: HTMLElement; dispose: () => void } | null>(
    null,
  );
  useEffect(
    () => () => {
      attached.current?.dispose();
      attached.current = null;
    },
    [],
  );
  useEffect(() => {
    const host = root.current;
    if (attached.current?.host === host) return;
    attached.current?.dispose();
    attached.current = null;
    if (!host) return;
    let cancel = () => {};
    const down = (event: PointerEvent) => {
      const handle = (event.target as Element).closest<HTMLElement>(
        "[data-sort-handle]",
      );
      if (
        !handle ||
        !host.contains(handle) ||
        event.button !== 0 ||
        handle.getAttribute("aria-disabled") === "true"
      )
        return;
      const group = handle.dataset.sortHandle!;
      const item = handle.closest<HTMLElement>(`[data-sort-group="${group}"]`);
      if (!item) return;
      event.preventDefault();
      // Commit a focused editor before its index can change; keep drag listeners
      // across ordinary renders so this blur cannot cancel the gesture.
      flushSync(() => (document.activeElement as HTMLElement | null)?.blur());
      const from = item.dataset.sortKey!;
      const horizontal = group === "columns";
      finishLayoutMotion(host);
      const origin = item.getBoundingClientRect();
      let x = event.clientX,
        y = event.clientY,
        active = false,
        to = from;
      let frame = 0;
      const originalOrder = Number(item.dataset.sortOrder);
      let ghost: { container: HTMLElement; row: HTMLElement } | undefined;
      const scroller =
        host.querySelector<HTMLElement>(
          ".ant-table-tbody-virtual-holder, .ant-table-body",
        ) ??
        host.closest<HTMLElement>(".editor-scroll") ??
        host;
      const makeGhost = () => {
        if (group !== "rows") return;
        // A virtual row can unmount while auto-scrolling. A visual snapshot stays
        // under the pointer; the real editors and draft never move until drop.
        const row = item.cloneNode(true) as HTMLElement;
        const originals = [item, ...item.querySelectorAll<HTMLElement>("*")];
        const copies = [row, ...row.querySelectorAll<HTMLElement>("*")];
        const properties = [
          "display",
          "box-sizing",
          "font",
          "color",
          "background",
          "border",
          "border-radius",
          "padding",
          "margin",
          "gap",
          "align-items",
          "justify-content",
          "flex-direction",
          "flex-grow",
          "flex-shrink",
          "flex-basis",
          "overflow",
          "white-space",
          "text-align",
          "line-height",
          "box-shadow",
          "opacity",
        ];
        originals.forEach((element, index) => {
          const copy = copies[index],
            style = getComputedStyle(element),
            box = element.getBoundingClientRect();
          properties.forEach((property) =>
            copy.style.setProperty(property, style.getPropertyValue(property)),
          );
          copy.style.width = `${box.width}px`;
          copy.style.height = `${box.height}px`;
          copy.removeAttribute("aria-label");
          for (const attribute of [...copy.attributes])
            if (
              attribute.name.startsWith("data-sort-") ||
              attribute.name === "data-row-key" ||
              attribute.name === "data-motion-key" ||
              attribute.name === "id"
            )
              copy.removeAttribute(attribute.name);
          if (
            element instanceof HTMLInputElement &&
            copy instanceof HTMLInputElement
          )
            copy.value = element.value;
        });
        [...row.children].forEach((child, index) => {
          const box = item.children[index].getBoundingClientRect();
          Object.assign((child as HTMLElement).style, {
            position: "absolute",
            left: `${box.left - origin.left}px`,
            top: `${box.top - origin.top}px`,
          });
        });
        const container = document.createElement("div");
        container.className = "sort-ghost";
        container.setAttribute("aria-hidden", "true");
        container.inert = true;
        Object.assign(container.style, {
          position: "fixed",
          overflow: "hidden",
          pointerEvents: "none",
          zIndex: "2000",
        });
        Object.assign(row.style, {
          position: "absolute",
          margin: "0",
          translate: "none",
          transform: "none",
          background: getComputedStyle(host).getPropertyValue("--surface"),
          boxShadow: "0 5px 18px #0003",
        });
        container.append(row);
        document.body.append(container);
        ghost = { container, row };
      };
      const saved = new Map<
        HTMLElement,
        {
          translate: string;
          transition: string;
          zIndex: string;
          opacity: string;
        }
      >();
      const anchors = () => [
        ...host.querySelectorAll<HTMLElement>(
          `[data-sort-group="${group}"][data-sort-anchor]`,
        ),
      ];
      const rect = (element: HTMLElement) => {
        const box = element.getBoundingClientRect();
        const shift = getComputedStyle(element)
          .translate.split(" ")
          .map(parseFloat);
        return {
          left: box.left - (shift[0] || 0),
          top: box.top - (shift[1] || 0),
          width: box.width,
          height: box.height,
        };
      };
      const paint = () => {
        if (!active) return;
        const items = anchors();
        const start = items.findIndex((e) => e.dataset.sortKey === from);
        if (start < 0 && !ghost) {
          finish(false);
          return;
        }
        if (!items.length) return;
        const center = horizontal
          ? origin.left + origin.width / 2 + x - event.clientX
          : origin.top + origin.height / 2 + y - event.clientY;
        let target = start,
          distance = Infinity;
        items.forEach((element, index) => {
          const r = rect(element);
          const d = Math.abs(
            center - (horizontal ? r.left + r.width / 2 : r.top + r.height / 2),
          );
          if (d < distance) {
            distance = d;
            target = index;
          }
        });
        to = items[target].dataset.sortKey!;
        const base = start >= 0 ? rect(items[start]) : origin;
        const startOrder = ghost ? originalOrder : start;
        const targetOrder = ghost
          ? Number(items[target].dataset.sortOrder)
          : target;
        const delta = horizontal
          ? x - event.clientX + origin.left - base.left
          : y - event.clientY + origin.top - base.top;
        const size = horizontal ? base.width : base.height;
        const offsets = new Map(
          items.map((element, index) => [
            element.dataset.sortKey!,
            element.dataset.sortKey === from
              ? delta
              : startOrder < targetOrder &&
                  (ghost ? Number(element.dataset.sortOrder) : index) >
                    startOrder &&
                  (ghost ? Number(element.dataset.sortOrder) : index) <=
                    targetOrder
                ? -size
                : startOrder > targetOrder &&
                    (ghost ? Number(element.dataset.sortOrder) : index) >=
                      targetOrder &&
                    (ghost ? Number(element.dataset.sortOrder) : index) <
                      startOrder
                  ? size
                  : 0,
          ]),
        );
        if (ghost) {
          const clip = scroller.getBoundingClientRect();
          Object.assign(ghost.container.style, {
            left: `${clip.left}px`,
            top: `${clip.top}px`,
            width: `${clip.width}px`,
            height: `${clip.height}px`,
          });
          Object.assign(ghost.row.style, {
            left: `${origin.left - clip.left}px`,
            top: `${origin.top - clip.top + y - event.clientY}px`,
          });
        }
        host
          .querySelectorAll<HTMLElement>(`[data-sort-group="${group}"]`)
          .forEach((element) => {
            if (!saved.has(element))
              saved.set(element, {
                translate: element.style.translate,
                transition: element.style.transition,
                zIndex: element.style.zIndex,
                opacity: element.style.opacity,
              });
            const dragged = element.dataset.sortKey === from;
            element.classList.toggle("sort-dragging", dragged);
            element.style.transition =
              dragged ||
              document.documentElement.dataset.reducedMotion === "true"
                ? "none"
                : `translate 120ms ${easeOutExpoCss}`;
            const offset = offsets.get(element.dataset.sortKey!) ?? 0;
            element.style.translate = horizontal
              ? `${offset}px 0px`
              : `0px ${offset}px`;
            if (dragged) {
              element.style.zIndex = "8";
              if (ghost) element.style.opacity = "0";
            }
          });
      };
      const tick = () => {
        if (!active) return;
        // The scrollable editor ends above Problems, rather than at the window edge.
        const r = scroller.getBoundingClientRect();
        const velocity = (value: number, low: number, high: number) =>
          value < low + 38
            ? -Math.min(16, (low + 38 - value) / 3)
            : value > high - 38
              ? Math.min(16, (value - high + 38) / 3)
              : 0;
        const dy = horizontal ? 0 : velocity(y, r.top, r.bottom);
        const dx = horizontal ? velocity(x, r.left, r.right) : 0;
        if (dy) scroller.scrollTop += dy;
        if (dx)
          scroller.dispatchEvent(
            new WheelEvent("wheel", {
              deltaX: dx,
              bubbles: true,
              cancelable: true,
            }),
          );
        paint();
        frame = requestAnimationFrame(tick);
      };
      const pointerMove = (e: PointerEvent) => {
        if (e.pointerId !== event.pointerId) return;
        x = e.clientX;
        y = e.clientY;
        if (!active && Math.hypot(x - event.clientX, y - event.clientY) > 4) {
          active = true;
          makeGhost();
          frame = requestAnimationFrame(tick);
        }
        paint();
      };
      const finish = (commit: boolean) => {
        cancelAnimationFrame(frame);
        window.removeEventListener("pointermove", pointerMove);
        window.removeEventListener("pointerup", up);
        window.removeEventListener("pointercancel", abort);
        window.removeEventListener("keydown", escape, true);
        window.removeEventListener("blur", abort);
        if (commit && active && from !== to)
          captureDropPositions(
            host,
            ghost ? [from, ghost.row.getBoundingClientRect()] : undefined,
          );
        saved.forEach((style, element) => {
          element.classList.remove("sort-dragging");
          Object.assign(element.style, style);
        });
        ghost?.container.remove();
        if (commit && active && from !== to) action.current(group, from, to);
        active = false;
        cancel = () => {};
      };
      const up = (e: PointerEvent) => {
        if (e.pointerId === event.pointerId) finish(true);
      };
      const abort = () => finish(false);
      const escape = (e: KeyboardEvent) => {
        if (e.key === "Escape") {
          e.preventDefault();
          e.stopImmediatePropagation();
          finish(false);
        }
      };
      cancel();
      cancel = abort;
      window.addEventListener("pointermove", pointerMove);
      window.addEventListener("pointerup", up);
      window.addEventListener("pointercancel", abort);
      window.addEventListener("keydown", escape, true);
      window.addEventListener("blur", abort);
    };
    host.addEventListener("pointerdown", down);
    attached.current = {
      host,
      dispose: () => {
        cancel();
        host.removeEventListener("pointerdown", down);
      },
    };
  });
}
