// One collapsible section: channel categories, player lists, group-chat controls, the platform's
// game lists. Uncontrolled: each section remembers its own open state.
//
// `className` replaces the frame: a sidebar row by default, a bordered card where it stands alone.
import type { ReactNode } from "react";
import { useState } from "react";

export function Section({
  label,
  defaultOpen = true,
  className = "border-b border-edge",
  children,
}: {
  label: ReactNode;
  defaultOpen?: boolean;
  className?: string;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <section className={`flex flex-col ${className}`}>
      <button
        type="button"
        className="flex items-center gap-2 bg-panel px-3 py-2 text-left text-xs font-medium uppercase tracking-wide text-ink-dim hover:bg-raised hover:text-ink"
        onClick={() => setOpen(!open)}
      >
        <span className={`inline-block w-3 text-center transition-transform ${open ? "rotate-90" : ""}`}>▸</span>
        {label}
      </button>
      {open && children}
    </section>
  );
}
