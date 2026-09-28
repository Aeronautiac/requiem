// The sender resolves against the view — rendered via the shared Entity component, so the header
// matches how that name renders everywhere else. Content still carries raw mention tokens, which
// MentionText resolves against the same view.
//
// `grouped` only drops the sender header for a continuation of an uninterrupted run from the same
// sender. Every row stays its own hover target and carries its own time.
import type { ActorDisplay } from "amane-client/bindings.ts";
import { slotKeyToString } from "amane-client/bindings.ts";
import type { View } from "amane-client/game/view.ts";
import { refFromDisplay, statusLabels } from "amane-client/text.ts";
import { statusAccent, tint } from "../../style.ts";
import { Entity, TimeStamp } from "../Name.tsx";
import { MentionText } from "../mentions/MentionText.tsx";

export function Message({
  senderDisplay,
  content,
  view,
  timestamp,
  grouped = false,
  mentioned = false,
  last = false,
}: {
  senderDisplay: ActorDisplay;
  content: string;
  view: View;
  timestamp: number;
  grouped?: boolean;
  // Whether this message names the viewer. Tints the whole row and keeps the tint on hover, rather
  // than the ordinary hover highlight, so a ping stands out in a busy channel.
  mentioned?: boolean;
  // Last message of its chain — the next row is a different sender, a non-message, or nothing. Only
  // the tail of a chain carries the block's bottom spacing, so a header isn't shoved away from its
  // own continuation lines.
  last?: boolean;
}) {
  // The sender's public status, shown beside their name in the header. Only a player has one.
  const senderPlayer = senderDisplay !== "Mysterious" && senderDisplay !== "System" && "Raw" in senderDisplay
    ? slotKeyToString(senderDisplay.Raw)
    : null;
  const senderStatuses = senderPlayer ? statusLabels(view.actor_statuses.get(senderPlayer) ?? 0) : [];

  // The ping tint reuses the "stands out" accent already meant for exactly that (a bugged badge),
  // rather than the button accent, which means something else entirely.
  const pingColor = "var(--color-status-bugged)";
  return (
    <div
      className={`px-4 pt-0.5 ${grouped ? "" : "mt-2 first:mt-0"} ${last ? "pb-1.5" : "pb-0.5"} ${mentioned ? "" : "hover:bg-raised/40"}`}
      style={
        mentioned
          ? { backgroundColor: `color-mix(in srgb, ${pingColor} 10%, transparent)`, boxShadow: `inset 2px 0 0 ${pingColor}` }
          : undefined
      }
    >
      {!grouped && (
        <div className="flex items-baseline justify-between gap-2">
          <div className="flex min-w-0 items-baseline gap-2">
            <Entity of={refFromDisplay(senderDisplay, view.orgs)} view={view} chip />
            {senderStatuses.map((s) => (
              <span key={s} className="px-1 py-px text-xs uppercase tracking-wide" style={tint(statusAccent(s))}>
                {s}
              </span>
            ))}
          </div>
          <TimeStamp timestamp={timestamp} />
        </div>
      )}
      <div className="flex items-baseline gap-2">
        <div className="min-w-0 flex-1 whitespace-pre-wrap break-words text-sm text-ink">
          <MentionText content={content} view={view} />
        </div>
        {grouped && (
          // A grouped message drops the sender header, but every message still carries its own
          // moment: the timestamp stays beside the line so a continuation reads its own time.
          <TimeStamp timestamp={timestamp} />
        )}
      </div>
    </div>
  );
}
