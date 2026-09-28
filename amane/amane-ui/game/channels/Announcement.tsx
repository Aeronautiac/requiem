// The shared card every non-message event renders as. Pure presentation; the caller resolves the
// text and picks the colour per event type.
//
// `color` is any CSS colour string (a `var(--color-event-*)` reference). The translucency is
// applied here so callers hand in a solid colour without worrying about it.
import type { ReactNode } from "react";
import { TimeStamp } from "../Name.tsx";

export function Announcement({
  color,
  description,
  content,
  children,
  timestamp,
}: {
  color: string;
  description: string;
  // Plain text is the common case. Pass `children` instead when the body needs markup — an
  // embedded ping chip, say.
  content?: string;
  children?: ReactNode;
  timestamp?: number;
}) {
  return (
    <div className="px-3 py-0.5">
      <div className="relative overflow-hidden border-l-2 px-2.5 py-1.5" style={{ borderColor: color }}>
        <div className="pointer-events-none absolute inset-0" style={{ backgroundColor: color, opacity: 0.12 }} />
        <div className="relative">
          <div className="flex items-baseline justify-between gap-2">
            <div className="text-sm font-semibold uppercase tracking-wide" style={{ color }}>
              {description}
            </div>
            {timestamp !== undefined && <TimeStamp timestamp={timestamp} />}
          </div>
          <div className={`mt-0.5 break-words text-sm text-ink ${children ? "" : "whitespace-pre-wrap"}`}>
            {children ?? content}
          </div>
        </div>
      </div>
    </div>
  );
}
