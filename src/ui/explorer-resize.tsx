import { useEffect, useRef, useState } from "react";

export function ExplorerResize({
  value,
  update,
}: {
  value: number;
  update: (width: number) => void;
}) {
  const [dragging, setDragging] = useState(false);
  const drag = useRef<{ x: number; width: number; pointer: number } | null>(
    null,
  );
  const clamp = (width: number) =>
    Math.max(180, Math.min(480, innerWidth - 520, width));
  useEffect(
    () => () => {
      document.documentElement.style.removeProperty("cursor");
    },
    [],
  );
  return (
    <div
      role="separator"
      aria-label="エクスプローラーの幅"
      aria-orientation="vertical"
      aria-valuemin={180}
      aria-valuemax={Math.max(180, Math.min(480, innerWidth - 520))}
      aria-valuenow={clamp(value)}
      tabIndex={0}
      className={`explorer-resizer ${dragging ? "resizing" : ""}`}
      onDoubleClick={() => update(236)}
      onKeyDown={(event) => {
        const next =
          event.key === "ArrowLeft"
            ? value - 16
            : event.key === "ArrowRight"
              ? value + 16
              : event.key === "Home"
                ? 180
                : event.key === "End"
                  ? 480
                  : null;
        if (next !== null) {
          event.preventDefault();
          update(clamp(next));
        }
      }}
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        event.preventDefault();
        event.currentTarget.focus();
        event.currentTarget.setPointerCapture(event.pointerId);
        drag.current = {
          x: event.clientX,
          width: clamp(value),
          pointer: event.pointerId,
        };
        setDragging(true);
        document.documentElement.style.cursor = "col-resize";
      }}
      onPointerMove={(event) => {
        if (drag.current?.pointer === event.pointerId)
          update(clamp(drag.current.width + event.clientX - drag.current.x));
      }}
      onLostPointerCapture={() => {
        drag.current = null;
        setDragging(false);
        document.documentElement.style.removeProperty("cursor");
      }}
      onPointerUp={(event) => {
        if (event.currentTarget.hasPointerCapture(event.pointerId))
          event.currentTarget.releasePointerCapture(event.pointerId);
      }}
    />
  );
}
