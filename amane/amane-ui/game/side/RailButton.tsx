// One button in the rail's action row (Organization, Polls, Prosecutions), so the three read as the
// same kind of control. `active` marks the one whose panel is up; `count` is an optional badge.
import type { ReactNode } from "react";

export function RailButton({
  active = false,
  count,
  onClick,
  children,
}: {
  active?: boolean;
  count?: number;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      className={`flex min-h-8 w-full items-center gap-2 border border-edge px-3 py-1.5 text-sm font-medium ${
        active ? "bg-raised text-ink" : "bg-panel text-ink-dim hover:bg-raised hover:text-ink"
      }`}
      onClick={onClick}
    >
      {children}
      {count !== undefined && count > 0 && <span className="ml-auto bg-surface px-1.5 text-xs text-ink-dim">{count}</span>}
    </button>
  );
}
