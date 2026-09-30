type DragStart = { pointerId: number; clientX: number; clientY: number };

type ReorderOptions = {
  root: HTMLTableElement;
  axis: "x" | "y";
  sourceIndex: number;
  itemCount: number;
  rowHeight?: number;
  onDrop: (destination: number) => void;
};

/** A visual permutation; the application receives one intent only after drop. */
export function startGridReorder(event: DragStart, options: ReorderOptions): () => void {
  const { root, axis, sourceIndex, itemCount, onDrop } = options;
  const scroll = root.closest<HTMLElement>(".grid-scroll");
  if (!scroll) return () => {};
  const selector = axis === "x" ? "thead th[data-column-index]" : "tbody tr[data-grid-row-index]";
  const indexOf = (item: HTMLElement) => Number(axis === "x" ? item.dataset.columnIndex : item.dataset.gridRowIndex);
  const items = Array.from(root.querySelectorAll<HTMLElement>(selector));
  const source = items.find(item => indexOf(item) === sourceIndex);
  if (!source) return () => {};
  const sourceRect = source.getBoundingClientRect();
  const startScrollLeft = scroll.scrollLeft, startScrollTop = scroll.scrollTop;
  const size = axis === "x" ? sourceRect.width : (options.rowHeight ?? sourceRect.height);
  const sourceCenter = axis === "x" ? sourceRect.left + size / 2 : sourceRect.top + size / 2;
  const centers = items.map(item => {
    const rect = item.getBoundingClientRect();
    return { index: indexOf(item), center: rect.left + rect.width / 2 };
  });
  const rowOrigin = sourceRect.top - sourceIndex * size;
  let pointerX = event.clientX, pointerY = event.clientY;
  let active = false, closed = false, destination = sourceIndex;
  let paintedDestination: number | null = null;
  let overlay: HTMLDivElement | null = null;
  let ghost: HTMLDivElement | null = null;
  const touched = new Map<HTMLElement, { transform: string }>();
  const sourceCells = axis === "x"
    ? []
    : Array.from(source.children).map(cell => ({ cell: cell as HTMLElement, rect: cell.getBoundingClientRect() }));

  const bounds = () => {
    const rect = scroll.getBoundingClientRect();
    return { left: rect.left + scroll.clientLeft, top: rect.top + scroll.clientTop,
      right: rect.left + scroll.clientLeft + scroll.clientWidth,
      bottom: rect.top + scroll.clientTop + scroll.clientHeight };
  };
  const inside = () => {
    const rect = bounds();
    return pointerX >= rect.left && pointerX <= rect.right && pointerY >= rect.top && pointerY <= rect.bottom;
  };
  const touch = (item: HTMLElement, offset: number, isSource: boolean) => {
    if (!touched.has(item)) {
      if (offset === 0 && !isSource) return;
      touched.set(item, { transform: item.style.transform });
      item.classList.add("grid-reorder-item");
    }
    if (item.classList.contains("grid-drag-source") !== isSource) item.classList.toggle("grid-drag-source", isSource);
    const transform = axis === "x" ? `translateX(${offset}px)` : `translateY(${offset}px)`;
    if (item.style.transform !== transform) item.style.transform = transform;
  };
  const previewOrder = () => {
    if (paintedDestination !== destination) root.dataset.dragDestination = String(destination);
    paintedDestination = destination;
    for (const item of root.querySelectorAll<HTMLElement>(axis === "x" ? "[data-column-index]" : selector)) {
      const index = indexOf(item);
      const offset = index > sourceIndex && index <= destination ? -size
        : index < sourceIndex && index >= destination ? size : 0;
      touch(item, offset, index === sourceIndex);
    }
    // Removed virtual rows need no retained references; the ghost owns its copy.
    for (const item of touched.keys()) if (!root.contains(item)) touched.delete(item);
  };
  const copyCell = (cell: HTMLElement, width: number, height: number, left: number, top: number) => {
    if (!ghost) return;
    const table = document.createElement("table");
    table.className = "record-grid grid-drag-copy";
    Object.assign(table.style, { width: `${width}px`, height: `${height}px`, left: `${left}px`, top: `${top}px`,
      zIndex: cell.closest("thead") || cell.classList.contains("row-number") ? "2" : "1" });
    const section = document.createElement(cell.closest("thead") ? "thead" : "tbody");
    const row = document.createElement("tr");
    const clone = cell.cloneNode(true) as HTMLElement;
    for (const element of [clone, ...clone.querySelectorAll<HTMLElement>("*")]) {
      for (const attr of Array.from(element.attributes)) {
        if (attr.name === "id" || attr.name.startsWith("data-")) element.removeAttribute(attr.name);
      }
      element.removeAttribute("autofocus");
      if (element.matches("input, button, select, textarea, [tabindex]")) element.tabIndex = -1;
    }
    clone.classList.remove("grid-reorder-item", "grid-drag-source");
    clone.style.transform = "";
    Object.assign(clone.style, { width: `${width}px`, minWidth: `${width}px`, maxWidth: `${width}px`, height: `${height}px` });
    row.append(clone); section.append(row); table.append(section); ghost.append(table);
  };
  const refreshGhost = () => {
    if (!overlay || !ghost) return;
    const viewport = bounds();
    const head = root.querySelector<HTMLElement>("thead th")?.getBoundingClientRect();
    const rowHeader = root.querySelector<HTMLElement>("thead .row-number")?.getBoundingClientRect();
    const clipLeft = axis === "x" ? Math.max(viewport.left, rowHeader?.right ?? viewport.left) : viewport.left;
    const clipTop = axis === "y" ? Math.max(viewport.top, head?.bottom ?? viewport.top) : viewport.top;
    Object.assign(overlay.style, { left: `${clipLeft}px`, top: `${clipTop}px`,
      width: `${Math.max(0, viewport.right - clipLeft)}px`, height: `${Math.max(0, viewport.bottom - clipTop)}px` });
    ghost.replaceChildren();
    if (axis === "x") {
      for (const cell of root.querySelectorAll<HTMLElement>(`[data-column-index="${sourceIndex}"]`)) {
        const rect = cell.getBoundingClientRect();
        if (rect.bottom < clipTop || rect.top > viewport.bottom) continue;
        copyCell(cell, rect.width, rect.height, sourceRect.left - clipLeft, rect.top - clipTop);
      }
    } else {
      for (const { cell, rect } of sourceCells) {
        const left = cell.classList.contains("row-number") ? viewport.left : rect.left - (scroll.scrollLeft - startScrollLeft);
        copyCell(cell, rect.width, rect.height, left - clipLeft, sourceRect.top - clipTop);
      }
    }
    moveGhost();
  };
  const moveGhost = () => {
    if (ghost) ghost.style.transform = axis === "x"
      ? `translateX(${pointerX - event.clientX}px)` : `translateY(${pointerY - event.clientY}px)`;
  };
  const locate = () => {
    if (!active || closed) return;
    if (!root.isConnected) { cleanup(); return; }
    // Animated rectangles move under the pointer. Frozen centers plus scroll
    // offset keep the permutation stable instead of oscillating between targets.
    // EVIDENCE: GUI-UNIFIED-008, docs/gui/data-editor/grid-authoring.md.
    const center = sourceCenter + (axis === "x" ? pointerX - event.clientX + scroll.scrollLeft - startScrollLeft
      : pointerY - event.clientY + scroll.scrollTop - startScrollTop);
    if (axis === "y") {
      destination = Math.max(0, Math.min(itemCount - 1, Math.floor((center - rowOrigin) / size)));
    } else {
      destination = sourceIndex;
      for (const item of centers) {
        if (item.index > sourceIndex && center > item.center) destination = item.index;
        else if (item.index < sourceIndex && center < item.center) { destination = item.index; break; }
      }
    }
    if (paintedDestination !== destination) previewOrder();
    moveGhost();
  };
  const observer = new MutationObserver(() => {
    if (active && !closed) { locate(); previewOrder(); refreshGhost(); }
  });
  const activate = () => {
    active = true;
    root.classList.add("grid-reordering");
    overlay = document.createElement("div");
    overlay.className = "grid-drag-overlay";
    overlay.setAttribute("aria-hidden", "true");
    overlay.inert = true;
    ghost = document.createElement("div"); ghost.className = "grid-drag-ghost";
    overlay.append(ghost); scroll.append(overlay);
    refreshGhost();
    observer.observe(root, { childList: true, subtree: true });
  };
  const scrollChanged = () => { if (active) { locate(); refreshGhost(); } };
  const edgeScroll = () => {
    if (!active || closed || !inside()) return;
    const rect = bounds();
    const headerBottom = root.querySelector<HTMLElement>("thead th")?.getBoundingClientRect().bottom ?? rect.top;
    const oldLeft = scroll.scrollLeft, oldTop = scroll.scrollTop;
    if (axis === "x") {
      if (pointerX < rect.left + 34) scroll.scrollLeft = Math.max(0, scroll.scrollLeft - 18);
      else if (pointerX > rect.right - 34) scroll.scrollLeft += 18;
    } else {
      if (pointerY < headerBottom + 34) scroll.scrollTop = Math.max(0, scroll.scrollTop - 18);
      else if (pointerY > rect.bottom - 34) scroll.scrollTop += 18;
    }
    if (scroll.scrollLeft !== oldLeft || scroll.scrollTop !== oldTop) scrollChanged();
  };
  const timer = window.setInterval(edgeScroll, 30);
  const cleanup = () => {
    if (closed) return;
    closed = true;
    window.clearInterval(timer); observer.disconnect();
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
    window.removeEventListener("pointercancel", cancel);
    window.removeEventListener("keydown", key, true);
    window.removeEventListener("blur", cleanup);
    window.removeEventListener("resize", cleanup);
    scroll.removeEventListener("scroll", scrollChanged);
    overlay?.remove();
    root.classList.remove("grid-reordering"); delete root.dataset.dragDestination;
    for (const [item, original] of touched) {
      item.classList.remove("grid-reorder-item", "grid-drag-source"); item.style.transform = original.transform;
    }
    touched.clear();
  };
  const move = (pointer: PointerEvent) => {
    if (pointer.pointerId !== event.pointerId) return;
    pointerX = pointer.clientX; pointerY = pointer.clientY;
    if (!active && Math.hypot(pointerX - event.clientX, pointerY - event.clientY) < 5) return;
    if (!active) activate();
    pointer.preventDefault(); locate();
  };
  const up = (pointer: PointerEvent) => {
    if (pointer.pointerId !== event.pointerId) return;
    pointerX = pointer.clientX; pointerY = pointer.clientY;
    if (active) { pointer.preventDefault(); locate(); }
    const result = active && !closed && inside() && destination !== sourceIndex ? destination : null;
    cleanup();
    if (result !== null) onDrop(result);
  };
  const cancel = (pointer: PointerEvent) => { if (pointer.pointerId === event.pointerId) cleanup(); };
  const key = (event: KeyboardEvent) => {
    if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); cleanup(); }
  };
  window.addEventListener("pointermove", move, { passive: false });
  window.addEventListener("pointerup", up);
  window.addEventListener("pointercancel", cancel);
  window.addEventListener("keydown", key, true);
  window.addEventListener("blur", cleanup);
  window.addEventListener("resize", cleanup);
  scroll.addEventListener("scroll", scrollChanged);
  return cleanup;
}
