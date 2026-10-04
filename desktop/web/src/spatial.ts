import type { PointerEvent as ReactPointerEvent } from "react";
import type { Projection, Row } from "./types";
import { desktop, GRID, type ComplexView, type EditorNode } from "./workspace";

// Only the three authoring reorder gestures share this bounded paint/cancel
// lifetime. It owns no values, history, or write authority; Rust applies a drop.
type Point = { x: number; y: number };
type Paint = {
  clip: DOMRect;
  x: number;
  y: number;
  landingX: number;
  landingY: number;
  target: number;
  valid: boolean;
  neighbours: [HTMLElement, number, number][];
};
type Gesture = {
  axis: "x" | "y";
  original: HTMLElement;
  scroll: HTMLElement;
  valid: () => boolean;
  paint: (point: Point) => Paint;
  preview: () => HTMLElement;
  drop: (target: number) => Promise<boolean>;
  originals?: () => HTMLElement[];
  updatePreview?: (ghost: HTMLElement) => void;
  from: number;
  header: number;
  origin: () => Point;
};
let cancelActive: (() => void) | null = null;
let clickUntil = 0;
export function consumeSpatialClick(detail: number) {
  return detail > 0 && performance.now() < clickUntil;
}
export function beginRangeSelection(
  event: ReactPointerEvent<HTMLElement>,
  row: Row,
  column: number,
) {
  const initial = desktop.surface.projection,
    v = desktop.viewport;
  if (
    !initial ||
    !v ||
    event.button !== 0 ||
    !event.isPrimary ||
    desktop.surface.busy ||
    desktop.surface.pending
  )
    return;
  event.preventDefault();
  cancelActive?.();
  const pointer = event.pointerId,
    first = { x: event.clientX, y: event.clientY },
    previousComplex = desktop.interaction.complex;
  let point = first,
    scope = initial,
    frame = 0,
    alive = true,
    dragged = false;
  desktop.setSelection(row.viewIndex, column, event.shiftKey);
  v.focus();
  const finish = () => {
    if (!alive) return;
    alive = false;
    cancelAnimationFrame(frame);
    document.removeEventListener("pointermove", move, true);
    document.removeEventListener("pointerup", up, true);
    document.removeEventListener("pointercancel", up, true);
    document.removeEventListener("keydown", key, true);
    window.removeEventListener("blur", finish);
    document.removeEventListener("visibilitychange", visibility);
    v.removeEventListener("lostpointercapture", up);
    unsubscribe();
    if (v.hasPointerCapture(pointer)) v.releasePointerCapture(pointer);
    if (cancelActive === finish) cancelActive = null;
  };
  const unsubscribe = desktop.subscribe(() => {
    if (!sameContext(scope, false)) finish();
  });
  const move = (e: PointerEvent) => {
    if (e.pointerId === pointer) point = { x: e.clientX, y: e.clientY };
  };
  const up = (e: PointerEvent) => {
    if (e.pointerId === pointer) finish();
  };
  const key = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopImmediatePropagation();
      desktop.collapseRange();
      finish();
    }
  };
  const visibility = () => {
    if (document.visibilityState !== "visible") finish();
  };
  const paint = () => {
    if (!alive || !sameContext(scope)) {
      finish();
      return;
    }
    if (!dragged && Math.hypot(point.x - first.x, point.y - first.y) >= 4) {
      dragged = true;
      try {
        v.setPointerCapture(pointer);
      } catch {
        /* Native capture is unavailable in DOM boundary tests. */
      }
    }
    if (dragged) {
      const rect = v.getBoundingClientRect(),
        edge = 24;
      const scroll = (position: number, low: number, high: number) =>
        position < low + edge
          ? -Math.min(16, (low + edge - position) * 0.5)
          : position > high - edge
            ? Math.min(16, (position - high + edge) * 0.5)
            : 0;
      v.scrollTop += scroll(
        point.y,
        rect.top + GRID.header,
        rect.top + v.clientHeight,
      );
      v.scrollLeft += scroll(
        point.x,
        rect.left + GRID.identity,
        rect.left + v.clientWidth,
      );
      const current = desktop.surface.projection!;
      const at = Math.max(
        0,
        Math.min(
          current.totalRows - 1,
          Math.floor(
            (point.y - rect.top - GRID.header + v.scrollTop) / GRID.row,
          ),
        ),
      );
      const col = Math.max(
        0,
        Math.min(
          current.columns.length - 1,
          Math.floor(
            (point.x - rect.left - GRID.identity + v.scrollLeft) / GRID.column,
          ),
        ),
      );
      if (
        current.rows.some((r) => r.viewIndex === at) &&
        (at !== desktop.interaction.selection.row ||
          col !== desktop.interaction.selection.column)
      )
        desktop.setSelection(at, col, true);
    }
    frame = requestAnimationFrame(paint);
  };
  cancelActive = finish;
  document.addEventListener("pointermove", move, true);
  document.addEventListener("pointerup", up, true);
  document.addEventListener("pointercancel", up, true);
  document.addEventListener("keydown", key, true);
  window.addEventListener("blur", finish);
  document.addEventListener("visibilitychange", visibility);
  v.addEventListener("lostpointercapture", up);
  void desktop
    .commit()
    .then((ok) => {
      if (!alive || !ok || !sameContext(initial, false)) {
        finish();
        return;
      }
      scope = desktop.surface.projection!;
      if (desktop.interaction.complex === previousComplex)
        desktop.closeComplex();
      frame = requestAnimationFrame(paint);
    })
    .catch((error) => {
      if (sameContext(initial, false)) desktop.showError(error);
      finish();
    });
}
function sameContext(p: Projection, strict = true) {
  const current = desktop.surface.projection;
  return (
    !!current &&
    !desktop.surface.pending &&
    !desktop.surface.queryPending &&
    current.sessionEpoch === p.sessionEpoch &&
    current.clicked === p.clicked &&
    current.source === p.source &&
    current.table.source === p.table.source &&
    (!strict ||
      (current.revision === p.revision &&
        current.schemaRevision === p.schemaRevision &&
        current.generation === p.generation))
  );
}
function cleanClone(node: HTMLElement) {
  const clone = node.cloneNode(true) as HTMLElement;
  for (const element of [clone, ...clone.querySelectorAll<HTMLElement>("*")]) {
    element.removeAttribute("id");
    element.removeAttribute("aria-selected");
    if (element.matches("button,input,select,textarea,[tabindex]"))
      element.tabIndex = -1;
  }
  clone.inert = true;
  clone.setAttribute("aria-hidden", "true");
  return clone;
}
function clipFor(v: HTMLElement, axis: "x" | "y", header: number) {
  const rect = v.getBoundingClientRect();
  return new DOMRect(
    rect.left + (axis === "x" ? GRID.identity : 0),
    rect.top + header,
    v.clientWidth - (axis === "x" ? GRID.identity : 0),
    v.clientHeight - header,
  );
}
function visibleRows(v: HTMLElement) {
  return [...v.querySelectorAll<HTMLElement>(".grid-row")];
}
function rowIndex(row: HTMLElement) {
  return Number(row.getAttribute("aria-rowindex")) - 2;
}
function rowElement(v: HTMLElement, id: string) {
  return visibleRows(v).find((row) => row.dataset.row === id);
}
function begin(
  event: ReactPointerEvent<HTMLElement>,
  create: () => Gesture | null,
) {
  if (event.button !== 0 || !event.isPrimary || desktop.surface.busy) return;
  event.preventDefault();
  event.stopPropagation();
  cancelActive?.();
  const pointer = event.pointerId,
    first = { x: event.clientX, y: event.clientY };
  let point = first,
    gesture: Gesture | null = null,
    overlay: HTMLDivElement | null = null,
    ghost: HTMLElement | null = null,
    frame = 0,
    moved = false,
    alive = true,
    settling = false,
    scope: Projection | null = null;
  const capture = event.currentTarget.closest<HTMLElement>(
    "#viewport,.ant-drawer-body",
  );
  const initialScope = desktop.surface.projection;
  let hidden = new Set<HTMLElement>();
  const hideOriginals = () => {
    const current = new Set(
      gesture?.originals?.() ?? (gesture ? [gesture.original] : []),
    );
    for (const node of hidden)
      if (!current.has(node)) node.classList.remove("spatial-origin");
    for (const node of current) node.classList.add("spatial-origin");
    hidden = current;
  };
  const showOriginals = () => {
    for (const node of hidden) node.classList.remove("spatial-origin");
    hidden.clear();
  };
  let painted = new Map<
    HTMLElement,
    { transform: string; transition: string }
  >();
  const clearNeighbours = () => {
    for (const [element, style] of painted) {
      element.style.transform = style.transform;
      element.style.transition = style.transition;
      element.classList.remove("spatial-displaced");
    }
    painted = new Map();
  };
  const unsubscribe = desktop.subscribe(() => {
    if (
      gesture &&
      (settling ? !scope || !sameContext(scope, false) : !gesture.valid())
    )
      cancel();
  });
  const finish = () => {
    if (!alive) return;
    alive = false;
    cancelAnimationFrame(frame);
    clearNeighbours();
    showOriginals();
    overlay?.remove();
    document.removeEventListener("pointermove", move, true);
    document.removeEventListener("pointerup", up, true);
    document.removeEventListener("pointercancel", pointerCancel, true);
    document.removeEventListener("keydown", key, true);
    window.removeEventListener("blur", cancel);
    document.removeEventListener("visibilitychange", visibility);
    capture?.removeEventListener("lostpointercapture", lostCapture);
    unsubscribe();
    if (cancelActive === cancel) cancelActive = null;
    if (capture?.hasPointerCapture(pointer))
      capture.releasePointerCapture(pointer);
  };
  const cancel = () => {
    if (moved) clickUntil = performance.now() + 100;
    if (ghost && gesture && !settling && gesture.valid()) {
      const p = gesture.paint(point),
        origin = gesture.origin();
      ghost.style.transition =
        "transform var(--md-motion) var(--md-ease), opacity var(--md-motion) var(--md-ease)";
      ghost.style.transform = `translate(${origin.x - p.clip.left}px, ${origin.y - p.clip.top}px)`;
      ghost.style.opacity = "0";
      const preview = overlay;
      overlay = null;
      setTimeout(() => preview?.remove(), 180);
    }
    finish();
  };
  const key = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopImmediatePropagation();
      cancel();
    }
  };
  const visibility = () => {
    if (document.visibilityState !== "visible") cancel();
  };
  const pointerCancel = (e: PointerEvent) => {
    if (e.pointerId === pointer) cancel();
  };
  const lostCapture = (e: PointerEvent) => {
    if (e.pointerId === pointer && !settling) cancel();
  };
  const move = (e: PointerEvent) => {
    if (e.pointerId === pointer) point = { x: e.clientX, y: e.clientY };
  };
  const paint = () => {
    if (!alive || !gesture || settling) return;
    if (!gesture.valid()) {
      cancel();
      return;
    }
    if (!moved && Math.hypot(point.x - first.x, point.y - first.y) >= 4) {
      moved = true;
      overlay = document.createElement("div");
      overlay.className = "spatial-clip";
      overlay.setAttribute("aria-hidden", "true");
      overlay.inert = true;
      ghost = gesture.preview();
      ghost.inert = true;
      ghost.setAttribute("aria-hidden", "true");
      ghost.classList.add("spatial-ghost");
      overlay.append(ghost);
      (gesture.scroll.closest("#grid-area") ??
        gesture.scroll.closest(".ant-drawer-content"))!.append(overlay);
      hideOriginals();
      try {
        capture?.setPointerCapture(pointer);
      } catch {
        /* DOM boundary tests have no native pointer capture. */
      }
    }
    if (moved && overlay && ghost) {
      const v = gesture.scroll,
        rect = v.getBoundingClientRect(),
        edge = Math.min(
          40,
          (gesture.axis === "x" ? v.clientWidth : v.clientHeight) * 0.06,
        );
      const coordinate = gesture.axis === "x" ? point.x : point.y,
        low =
          gesture.axis === "x"
            ? rect.left + GRID.identity
            : rect.top + gesture.header,
        high =
          gesture.axis === "x"
            ? rect.left + v.clientWidth
            : rect.top + v.clientHeight;
      const distance =
        coordinate < low + edge
          ? -Math.min(18, (low + edge - coordinate) * 0.5)
          : coordinate > high - edge
            ? Math.min(18, (coordinate - high + edge) * 0.5)
            : 0;
      if (gesture.axis === "x") v.scrollLeft += distance;
      else v.scrollTop += distance;
      const p = gesture.paint(point),
        parent = overlay.parentElement!.getBoundingClientRect();
      hideOriginals();
      gesture.updatePreview?.(ghost);
      overlay.style.left = `${p.clip.left - parent.left}px`;
      overlay.style.top = `${p.clip.top - parent.top}px`;
      overlay.style.width = `${p.clip.width}px`;
      overlay.style.height = `${p.clip.height}px`;
      ghost.style.transform = `translate(${p.x - p.clip.left}px, ${p.y - p.clip.top}px)`;
      ghost.dataset.dropValid = String(p.valid);
      // Restore the previous bounded window before applying its current subset.
      // Detached windows are never retained as the user scrolls through a Table.
      clearNeighbours();
      for (const [element, x, y] of p.neighbours) {
        painted.set(element, {
          transform: element.style.transform,
          transition: element.style.transition,
        });
        element.classList.add("spatial-displaced");
        element.style.transform = `translate(${x}px, ${y}px)`;
      }
    }
    frame = requestAnimationFrame(paint);
  };
  const up = (e: PointerEvent) => {
    if (e.pointerId !== pointer) return;
    point = { x: e.clientX, y: e.clientY };
    if (!moved || !gesture || !ghost) {
      finish();
      return;
    }
    clickUntil = performance.now() + 100;
    const p = gesture.paint(point);
    const inside =
      point.x >= p.clip.left &&
      point.x <= p.clip.right &&
      point.y >= p.clip.top &&
      point.y <= p.clip.bottom;
    if (!inside || !p.valid || p.target === gesture.from || !gesture.valid()) {
      cancel();
      return;
    }
    settling = true;
    cancelAnimationFrame(frame);
    ghost.style.transition =
      "transform var(--md-motion) var(--md-ease), opacity var(--md-motion) var(--md-ease)";
    ghost.style.transform = `translate(${p.landingX - p.clip.left}px, ${p.landingY - p.clip.top}px)`;
    // The animation never authorizes the drop or delays its shared operation.
    void gesture
      .drop(p.target)
      .catch((error) => {
        if (alive && scope && sameContext(scope, false))
          desktop.showError(error);
        return false;
      })
      .then((ok) => {
        if (alive) {
          if (!ok && scope && sameContext(scope, false)) {
            const origin = gesture!.origin();
            ghost!.style.transform = `translate(${origin.x - p.clip.left}px, ${origin.y - p.clip.top}px)`;
          }
          clearNeighbours();
          showOriginals();
          ghost!.style.opacity = "0";
          setTimeout(
            finish,
            matchMedia("(prefers-reduced-motion: reduce)").matches ? 0 : 120,
          );
        }
      });
  };
  cancelActive = cancel;
  document.addEventListener("pointermove", move, true);
  document.addEventListener("pointerup", up, true);
  document.addEventListener("pointercancel", pointerCancel, true);
  document.addEventListener("keydown", key, true);
  window.addEventListener("blur", cancel);
  document.addEventListener("visibilitychange", visibility);
  capture?.addEventListener("lostpointercapture", lostCapture);
  void desktop
    .commit()
    .then((ok) => {
      if (!alive || !ok) {
        finish();
        return;
      }
      gesture = create();
      if (!gesture) {
        finish();
        return;
      }
      scope = desktop.surface.projection;
      frame = requestAnimationFrame(paint);
    })
    .catch((error) => {
      if (initialScope && sameContext(initialScope, false))
        desktop.showError(error);
      finish();
    });
}

export function beginRowDrag(
  event: ReactPointerEvent<HTMLElement>,
  requested: Row,
) {
  const previous = desktop.surface.projection,
    v = desktop.viewport;
  if (!previous || !v || previous.viewState.search || requested.pendingDelete)
    return;
  const grab =
    event.clientY -
    (rowElement(v, requested.id)?.getBoundingClientRect().top ?? event.clientY);
  begin(event, () => {
    const p = desktop.surface.projection;
    if (
      !p ||
      !sameContext(previous, false) ||
      !p.source ||
      p.viewState.search ||
      p.rows.find((r) => r.id === requested.id)?.pendingDelete
    )
      return null;
    const original = rowElement(v, requested.id),
      from = p.rows.find((r) => r.id === requested.id)?.viewIndex;
    if (!original || from === undefined) return null;
    const origin = () => {
      const r = v.getBoundingClientRect();
      return {
        x: r.left - v.scrollLeft,
        y: r.top + GRID.header + from * GRID.row - v.scrollTop,
      };
    };
    return {
      axis: "y",
      original,
      scroll: v,
      from,
      header: GRID.header,
      valid: () => sameContext(p),
      origin,
      originals: () => {
        const current = rowElement(v, requested.id);
        return current ? [current] : [];
      },
      preview: () => {
        const clone = cleanClone(original);
        clone.style.width = `${original.offsetWidth}px`;
        const identity = clone.querySelector<HTMLElement>(".row-identity")!;
        identity.style.position = "absolute";
        identity.style.left = `${v.scrollLeft}px`;
        identity.style.width = `${GRID.identity}px`;
        identity.style.height = `${GRID.row}px`;
        return clone;
      },
      updatePreview: (clone) => {
        clone.querySelector<HTMLElement>(".row-identity")!.style.left =
          `${v.scrollLeft}px`;
      },
      paint: (point) => {
        const r = v.getBoundingClientRect(),
          clip = clipFor(v, "y", GRID.header);
        const target = Math.max(
          0,
          Math.min(
            p.totalRows - 1,
            Math.floor(
              (point.y - r.top - GRID.header + v.scrollTop) / GRID.row,
            ),
          ),
        );
        const current = desktop.surface.projection!;
        const destination = current.rows.find(
          (row) => row.viewIndex === target,
        );
        return {
          clip,
          x: r.left - v.scrollLeft,
          y: point.y - grab,
          landingX: r.left - v.scrollLeft,
          landingY: r.top + GRID.header + target * GRID.row - v.scrollTop,
          target,
          valid: !!destination && !destination.pendingDelete,
          neighbours: visibleRows(v)
            .filter((element) => {
              const at = rowIndex(element),
                y = GRID.header + at * GRID.row - v.scrollTop;
              if (y + GRID.row <= GRID.header || y >= v.clientHeight)
                return false;
              return target > from
                ? at > from && at <= target
                : at >= target && at < from;
            })
            .map((element) => [
              element,
              0,
              (target > from ? -1 : 1) * GRID.row,
            ]),
        };
      },
      drop: (target) => desktop.moveRow(requested.id, target, p),
    };
  });
}
export function beginColumnDrag(
  event: ReactPointerEvent<HTMLElement>,
  index: number,
) {
  const previous = desktop.surface.projection,
    v = desktop.viewport;
  if (!previous || !v) return;
  const grab =
    event.clientX -
    (v.getBoundingClientRect().left +
      GRID.identity +
      index * GRID.column -
      v.scrollLeft);
  begin(event, () => {
    const p = desktop.surface.projection;
    if (!p || !sameContext(previous, false)) return null;
    const original = [...v.querySelectorAll<HTMLElement>(".column")][index];
    if (!original) return null;
    const origin = () => {
      const r = v.getBoundingClientRect();
      return {
        x: r.left + GRID.identity + index * GRID.column - v.scrollLeft,
        y: r.top,
      };
    };
    let previewKey = "";
    const updatePreview = (clone: HTMLElement) => {
      const key = `${v.scrollTop}:${v.clientHeight}:${visibleRows(v)
        .map((row) => row.dataset.row)
        .join()}`;
      if (key === previewKey) return;
      previewKey = key;
      clone.replaceChildren();
      clone.style.height = `${v.clientHeight}px`;
      const header = cleanClone(original);
      header.style.height = `${GRID.header}px`;
      clone.append(header);
      for (const row of visibleRows(v)) {
        const cell = row.querySelector<HTMLElement>(`[data-column="${index}"]`);
        if (!cell) continue;
        const y = GRID.header + rowIndex(row) * GRID.row - v.scrollTop;
        if (y + GRID.row <= GRID.header || y >= v.clientHeight) continue;
        const value = cleanClone(cell);
        value.classList.remove("spatial-origin");
        value.style.position = "absolute";
        value.style.top = `${y}px`;
        value.style.width = `${GRID.column}px`;
        clone.append(value);
      }
      header.classList.remove("spatial-origin");
    };
    return {
      axis: "x",
      original,
      scroll: v,
      from: index,
      header: 0,
      valid: () => sameContext(p),
      origin,
      originals: () => [
        original,
        ...visibleRows(v).flatMap((row) => {
          const cell = row.querySelector<HTMLElement>(
            `[data-column="${index}"]`,
          );
          return cell ? [cell] : [];
        }),
      ],
      preview: () => {
        const clone = document.createElement("div");
        clone.className = "column-preview";
        clone.style.width = `${GRID.column}px`;
        updatePreview(clone);
        return clone;
      },
      updatePreview,
      paint: (point) => {
        const r = v.getBoundingClientRect(),
          clip = clipFor(v, "x", 0);
        const target = Math.max(
          0,
          Math.min(
            p.columns.length - 1,
            Math.floor(
              (point.x - r.left - GRID.identity + v.scrollLeft) / GRID.column,
            ),
          ),
        );
        const neighbours: Paint["neighbours"] = [];
        for (
          let at = Math.min(index, target);
          at <= Math.max(index, target);
          at++
        ) {
          if (at === index) continue;
          const x = GRID.identity + at * GRID.column - v.scrollLeft;
          if (x + GRID.column <= GRID.identity || x >= v.clientWidth) continue;
          const displacement = (target > index ? -1 : 1) * GRID.column;
          const header = [...v.querySelectorAll<HTMLElement>(".column")][at];
          if (header) neighbours.push([header, displacement, 0]);
          for (const row of visibleRows(v)) {
            const y = GRID.header + rowIndex(row) * GRID.row - v.scrollTop;
            const cell = row.querySelector<HTMLElement>(
              `[data-column="${at}"]`,
            );
            if (cell && y + GRID.row > GRID.header && y < v.clientHeight)
              neighbours.push([cell, displacement, 0]);
          }
        }
        return {
          clip,
          x: point.x - grab,
          y: r.top,
          landingX:
            r.left + GRID.identity + target * GRID.column - v.scrollLeft,
          landingY: r.top,
          target,
          valid: true,
          neighbours,
        };
      },
      drop: (target) => {
        const order = p.columns.map((c) => c.field.name),
          [field] = order.splice(index, 1);
        order.splice(target, 0, field);
        return desktop.columns(order, p);
      },
    };
  });
}
export function beginArrayDrag(
  event: ReactPointerEvent<HTMLElement>,
  view: ComplexView,
  child: EditorNode,
  apply: (item: string, index: number) => Promise<boolean>,
  focus: (path: string[]) => void,
) {
  if (!view.node.shape?.array || !view.node.editable) return;
  const original = event.currentTarget.closest<HTMLElement>(".complex-item")!,
    scroll = original.closest<HTMLElement>(".ant-drawer-body")!;
  const grab = event.clientY - original.getBoundingClientRect().top;
  begin(event, () => {
    const p = desktop.surface.projection,
      complexTarget = desktop.interaction.complex,
      parent = original.parentElement!;
    if (
      !original.isConnected ||
      !p ||
      p.source !== view.source ||
      p.sessionEpoch !== view.sessionEpoch ||
      p.revision !== view.revision ||
      p.generation !== view.generation
    )
      return null;
    const nodes = [
        ...parent.querySelectorAll<HTMLElement>(":scope > .complex-item"),
      ],
      from = nodes.indexOf(original);
    const rects = nodes.map((node) => node.getBoundingClientRect()),
      top = scroll.scrollTop,
      height = rects[from].height;
    const origin = () => ({
      x: rects[from].left,
      y: rects[from].top - scroll.scrollTop + top,
    });
    return {
      axis: "y",
      original,
      scroll,
      from,
      header: 0,
      origin,
      valid: () =>
        sameContext(p) &&
        !!desktop.interaction.complex &&
        desktop.interaction.complex.row === view.row &&
        desktop.interaction.complex.start === view.node.start &&
        JSON.stringify(desktop.interaction.complex.path) ===
          JSON.stringify(view.node.path),
      preview: () => {
        const clone = cleanClone(original);
        clone.style.width = `${rects[from].width}px`;
        return clone;
      },
      paint: (point) => {
        const r = scroll.getBoundingClientRect(),
          clip = new DOMRect(
            r.left,
            r.top,
            scroll.clientWidth,
            scroll.clientHeight,
          );
        const y = point.y + scroll.scrollTop - top;
        let target = rects.findIndex(
          (rect) => y < rect.top + rect.height * 0.5,
        );
        if (target < 0) target = nodes.length - 1;
        return {
          clip,
          x: rects[from].left,
          y: point.y - grab,
          landingX: rects[from].left,
          landingY: rects[target].top - scroll.scrollTop + top,
          target,
          valid: target >= 0,
          neighbours: nodes.flatMap((node, at): Paint["neighbours"] =>
            rects[at].bottom - scroll.scrollTop + top > clip.top &&
            rects[at].top - scroll.scrollTop + top < clip.bottom &&
            (target > from
              ? at > from && at <= target
              : at >= target && at < from)
              ? [[node, 0, target > from ? -height : height]]
              : [],
          ),
        };
      },
      drop: async (target) => {
        const item = child.path.at(-1)!;
        const ok = await apply(item, view.node.start + target);
        if (ok && desktop.interaction.complex === complexTarget)
          focus(child.path);
        return ok;
      },
    };
  });
}
