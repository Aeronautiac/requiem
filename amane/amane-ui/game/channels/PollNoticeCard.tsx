// The channel-stream card for a vote that opened or closed — the counterpart to the interactive
// PollCard in the panel, rendered with the same Announcement base as every other event so it reads
// like the rest of the feed. It carries the subject and the proposed ability's arguments so a notice
// says what was actually on the table rather than just "Vote started", and for a closed poll it
// states how it ended. Read-only: voting is the panel's job.
import type { PollOutcome, PollSubject } from "amane-client/bindings.ts";
import { pollSubjectArgs, pollSubjectHeading, refFromKey, refLabel } from "amane-client/text.ts";
import type { View } from "amane-client/game/view.ts";
import { Announcement } from "./Announcement.tsx";

export function PollNoticeCard({
  poll_id,
  subject,
  outcome,
  opener,
  timestamp,
  view,
}: {
  poll_id: string;
  subject: PollSubject;
  outcome: PollOutcome | null;
  opener: string | null;
  timestamp: number;
  view: View;
}) {
  const heading = pollSubjectHeading(subject, view.players);
  const args = pollSubjectArgs(subject, view.players);

  // The description and accent read like every other announcement. Accept and Reject are the two
  // fixed accept/reject options, so a resolved vote reads as passed or rejected; a generic poll
  // names its winning option instead, and a poll that ended without one stays on the neutral accent.
  let label: string;
  let color: string;
  if (outcome === null) {
    label = "Vote Started";
    color = "var(--color-event-vote)";
  } else if (outcome === "Cancelled") {
    label = "Vote Cancelled";
    color = "var(--color-event-nothing)";
  } else if (outcome === "Inconclusive") {
    label = "Vote Closed — No Decision";
    color = "var(--color-event-nothing)";
  } else {
    const won = view.polls.get(poll_id)?.options[outcome.Resolved];
    const wonLabel = won === undefined ? "" : typeof won.label === "string" ? won.label : won.label.Generic;
    if (wonLabel === "Accept") {
      label = "Vote Passed";
      color = "var(--color-event-revival)";
    } else if (wonLabel === "Reject") {
      label = "Vote Rejected";
      color = "var(--color-event-death)";
    } else {
      label = wonLabel ? `Result: ${wonLabel}` : "Vote Resolved";
      color = "var(--color-event-vote)";
    }
  }

  return (
    <Announcement timestamp={timestamp} color={color} description={label}>
      <span>{heading}</span>
      {opener && (
        <span className="ml-1 text-xs text-ink-dim">by {refLabel(refFromKey(opener, view.players, view.orgs), view.players)}</span>
      )}
      {args.length > 0 && (
        <div className="mt-1 flex flex-col gap-0.5 border-l border-edge pl-2">
          {args.map((arg) => (
            <span key={arg.label} className="text-xs text-ink-dim">
              <span className="text-ink-dim">{arg.label}:</span> {arg.value}
            </span>
          ))}
        </div>
      )}
    </Announcement>
  );
}
