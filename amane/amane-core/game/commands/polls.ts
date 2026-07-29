import { slotKeyToString } from "../../bindings";
import type { PollOutcome, PollSubject, PollVisibility } from "../../bindings";
import type { GameView } from "../view.svelte";
import type { CmdCtx, Handlers } from "./index";

// Drop a notice into whichever channel the poll's scope maps to. No-op when that channel is not
// one this view holds, which is the ordinary case for a scope it cannot see into.
function pollNotice(
  view: GameView,
  scope: PollVisibility,
  poll_id: string,
  subject: PollSubject,
  outcome: PollOutcome | null,
  timestamp: number,
  opener: string | null,
) {
  let channel_key: string | undefined;
  if (scope === "AllPresent") {
    channel_key = view.news_channel_id ?? undefined;
  } else if ("Channel" in scope) {
    channel_key = slotKeyToString(scope.Channel);
  } else {
    channel_key = view.channel_of_org(slotKeyToString(scope.Org));
  }
  if (!channel_key) return;
  view.channels.get(channel_key)?.events.push({
    timestamp,
    data: { PollNotice: { poll_id, subject, outcome, opener } },
  });
}

export const pollHandlers: Handlers = {
  UpdatePoll(ctx: CmdCtx, p) {
    const key = slotKeyToString(p.poll_id);
    const opener = p.opener ? slotKeyToString(p.opener) : null;
    const existing = ctx.view.polls.get(key);
    ctx.view.record_poll_viewport(ctx.viewport, key);

    // First sight = the vote just opened; every later one is a tally refresh.
    if (!existing) {
      pollNotice(ctx.view, p.scope, key, p.subject, null, ctx.timestamp, opener);
    }

    ctx.view.polls.set(key, {
      subject: p.subject,
      scope: p.scope,
      accept: p.accept,
      reject: p.reject,
      potential: p.potential,
      opener,
      // An update after the close would be odd, but it must not un-resolve the poll.
      outcome: existing?.outcome ?? null,
    });
  },

  // The entry is kept, not deleted: a view gaining the poll's viewport later replays this whole
  // history and must reach the same place, and the notice below reads the poll's own subject. The
  // poll_views entry is left alone for the same reason.
  ClosePoll(ctx: CmdCtx, p) {
    const key = slotKeyToString(p.poll_id);
    const poll = ctx.view.polls.get(key);
    if (!poll) return;
    // A resolution notice has no opener: it is the outcome, not the opening.
    pollNotice(ctx.view, poll.scope, key, poll.subject, p.outcome, ctx.timestamp, null);
    // Re-set rather than mutate, so the polls panel stops showing a live vote.
    ctx.view.polls.set(key, { ...poll, outcome: p.outcome });
  },

  UpdatePollView(ctx: CmdCtx, p) {
    ctx.view.poll_views.set(slotKeyToString(p.poll_id), {
      eligible: p.eligible,
      own_vote: p.own_vote,
    });
  },
};
