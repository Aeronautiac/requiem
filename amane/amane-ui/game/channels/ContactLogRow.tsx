// Deliberately not an Announcement — a log is read by scanning dozens of these, so the row carries
// no label of its own and leans on the accent colour instead. Pure presentation: the caller resolves
// both ends to display strings.
import type { ActorDisplay, ContactEvent } from "amane-client/bindings.ts";
import type { View } from "amane-client/game/view.ts";
import { refFromDisplay } from "amane-client/text.ts";
import { Entity, TimeStamp } from "../Name.tsx";

// The verb wraps around the second name so each row reads as a sentence rather than a record to
// decode. Colour is scanning help only — the words carry it on their own. Reuses existing event
// accents rather than inventing new tokens: LoungeOpened as a record made (tap accent), added/removed
// as a plain confirmation/warning.
const PHRASES: Record<ContactEvent, { verb: string; suffix: string; color: string }> = {
  LoungeOpened: { verb: "contacted", suffix: "", color: "var(--color-event-tap)" },
  GroupchatAdded: { verb: "added", suffix: "to groupchat", color: "var(--color-ok-text)" },
  GroupchatRemoved: { verb: "removed", suffix: "from groupchat", color: "var(--color-danger-text)" },
};

export function ContactLogRow({
  from,
  to,
  event,
  timestamp,
  view,
}: {
  from: ActorDisplay;
  to: ActorDisplay;
  event: ContactEvent;
  timestamp: number;
  view: View;
}) {
  const phrase = PHRASES[event];
  return (
    <div className="px-4 py-1">
      <div
        className="flex items-baseline gap-3 border border-l-2 border-edge bg-panel/30 px-3 py-2 text-sm hover:bg-panel/60"
        style={{ borderLeftColor: phrase.color }}
      >
        <TimeStamp timestamp={timestamp} />
        <span className="min-w-0 flex-1 text-ink">
          <Entity of={refFromDisplay(from, view.orgs)} view={view} chip />{" "}
          <span style={{ color: phrase.color }}>{phrase.verb}</span>{" "}
          <Entity of={refFromDisplay(to, view.orgs)} view={view} chip />
          {phrase.suffix && <span className="text-ink-dim">&nbsp;{phrase.suffix}</span>}
        </span>
      </div>
    </div>
  );
}
