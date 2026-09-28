import { slotKeyToString } from "../../bindings.ts";
import type { PollOutcome, PollParent, PollSubject } from "../../bindings.ts";
import type { View } from "../view.ts";
import type { Handlers } from "./index.ts";

// Drop a notice into whichever channel the poll's parent maps to. No-op when that channel is not
// one this view holds, which is the ordinary case for a parent it cannot see into.
function pollNotice(
  view: View,
  parent: PollParent,
  poll_id: string,
  subject: PollSubject,
  outcome: PollOutcome | null,
  timestamp: number,
  opener: string | null,
) {
  let channel_key: string | undefined;
  if (parent === "World") {
    channel_key = view.news_channel ?? undefined;
  } else if ("Channel" in parent) {
    channel_key = slotKeyToString(parent.Channel);
  } else {
    channel_key = view.orgs.get(slotKeyToString(parent.Org))?.channel ?? undefined;
  }
  if (!channel_key) return;
  view.channels.get(channel_key)?.events.push({
    timestamp,
    data: { PollNotice: { poll_id, subject, outcome, opener } },
  });
}

export const pollHandlers: Handlers = {
  UpdatePoll(ctx, p) {
    const key = slotKeyToString(p.poll_id);
    const opener = p.opener ? slotKeyToString(p.opener) : null;
    const existing = ctx.view.polls.get(key);

    // First sight = the vote just opened; every later one is a tally refresh.
    if (!existing) {
      pollNotice(ctx.view, p.parent, key, p.subject, null, ctx.timestamp, opener);
    }

    ctx.view.polls.set(key, {
      subject: p.subject,
      parent: p.parent,
      options: p.options,
      potential: p.potential,
      opener,
      // An update after the close would be odd, but it must not un-resolve the poll.
      outcome: existing?.outcome ?? null,
      // The parent's viewport, since that is what a poll is addressed to now. Several polls share
      // one, and one going stale is exactly what freezes all of them.
      viewport: ctx.viewport ?? null,
    });
  },

  // The entry is kept, not deleted: a view gaining the parent's viewport later replays this whole
  // history and must reach the same place, and the notice below reads the poll's own subject. The
  // poll_views entry is left alone for the same reason.
  ClosePoll(ctx, p) {
    const key = slotKeyToString(p.poll_id);
    const poll = ctx.view.polls.get(key);
    if (!poll) return;
    // A resolution notice has no opener: it is the outcome, not the opening.
    pollNotice(ctx.view, poll.parent, key, poll.subject, p.outcome, ctx.timestamp, null);
    poll.outcome = p.outcome;
  },

  UpdatePollView(ctx, p) {
    ctx.view.poll_views.set(slotKeyToString(p.poll_id), {
      eligible: p.eligible,
      own_vote: p.own_vote,
    });
  },
};
