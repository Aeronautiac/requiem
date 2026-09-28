// A side column the user sizes by dragging its inner edge. Double-click the edge to go back to the
// default; with the edge focused, the arrow keys nudge it. The width is remembered per browser, as a
// convenience only: storage that is missing or refused just means starting at the default.
//
// The handle sits on the wrapper, not the scrolling region, so it spans the full height instead of
// scrolling away with the content.
import type { ReactNode } from "react";
import { useRef, useState } from "react";

const STEP = 16;
// No rail takes more than this share of the window, so two wide rails on a narrow screen can't
// squeeze the content between them away. Applied while dragging (so the handle never runs ahead of
// the edge) and in CSS (so a window shrunk afterwards still holds to it).
const SHARE = 0.3;

function load(key: string, fallback: number): number {
  try {
    const stored = Number(localStorage.getItem(key));
    return stored > 0 ? stored : fallback;
  } catch {
    return fallback;
  }
}

function save(key: string, width: number) {
  try {
    localStorage.setItem(key, String(width));
  } catch {
    // Not remembered; nothing else depends on it.
  }
}

export function ResizableRail({
  side,
  name,
  initial,
  min,
  max,
  className = "",
  children,
}: {
  // Which edge of the screen the rail is on; the handle is on the other edge, facing the content.
  side: "left" | "right";
  // Names the remembered width, so each rail keeps its own.
  name: string;
  // Widths in px.
  initial: number;
  min: number;
  max: number;
  className?: string;
  children: ReactNode;
}) {
  const key = `amane.rail.${name}`;
  const clamp = (w: number) => Math.max(min, Math.min(max, window.innerWidth * SHARE, Math.round(w)));
  const [width, setWidth] = useState(() => clamp(load(key, initial)));
  const [dragging, setDragging] = useState(false);
  const drag = useRef({ x: 0, width: 0 });

  function resize(w: number) {
    const next = clamp(w);
    setWidth(next);
    save(key, next);
  }

  function onPointerDown(e: React.PointerEvent) {
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { x: e.clientX, width };
    setDragging(true);
    // A drag across the message list would otherwise select its text.
    document.body.style.userSelect = "none";
  }
  function onPointerMove(e: React.PointerEvent) {
    if (!dragging) return;
    const delta = e.clientX - drag.current.x;
    setWidth(clamp(drag.current.width + (side === "left" ? delta : -delta)));
  }
  function onPointerUp() {
    if (!dragging) return;
    setDragging(false);
    document.body.style.userSelect = "";
    save(key, width);
  }
  function onKeyDown(e: React.KeyboardEvent) {
    // The arrows move the edge itself: toward the content widens a left rail and narrows a right one.
    const grow = side === "left" ? "ArrowRight" : "ArrowLeft";
    const shrink = side === "left" ? "ArrowLeft" : "ArrowRight";
    if (e.key === grow) resize(width + STEP);
    else if (e.key === shrink) resize(width - STEP);
    else return;
    e.preventDefault();
  }

  return (
    <aside
      className={`relative shrink-0 ${side === "left" ? "border-r" : "border-l"} border-edge ${className}`}
      style={{ width, maxWidth: `max(${min}px, ${SHARE * 100}vw)` }}
    >
      <div className="h-full overflow-y-auto">{children}</div>
      <div
        role="separator"
        aria-orientation="vertical"
        aria-valuenow={width}
        aria-valuemin={min}
        aria-valuemax={max}
        tabIndex={0}
        title="Drag to resize · double-click to reset"
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        onDoubleClick={() => resize(initial)}
        onKeyDown={onKeyDown}
        className={`absolute inset-y-0 z-10 w-1.5 cursor-col-resize touch-none focus:outline-none ${side === "left" ? "-right-1" : "-left-1"} ${dragging ? "bg-ink-dim" : "hover:bg-edge focus-visible:bg-edge"}`}
      />
    </aside>
  );
}
