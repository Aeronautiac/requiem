// What the polls panel and an inline poll card need: which polls are still live, and which channel
// a poll calls home. Grouping and lookups only — presentation stays in the component.
import { slotKeyToString } from "../bindings.ts";
import type { PollData, PollView } from "../game/types.ts";
import type { View } from "../game/view.ts";

export type LivePoll = { id: string; data: PollData; pollView: PollView | null; frozen: boolean };

// Every poll still open, in delivery order. A resolved poll keeps its entry (a late viewer replays
// it to reach the same outcome), so `outcome` is the liveness test.
export function livePolls(view: View): LivePoll[] {
  const out: LivePoll[] = [];
  for (const [id, data] of view.polls) {
    if (data.outcome) continue;
    out.push({ id, data, pollView: view.poll_views.get(id) ?? null, frozen: view.frozen(data.viewport) });
  }
  return out;
}

// The channel (or the news feed) a poll calls home, resolved from its parent scope: world polls
// live in News, org polls in the org's channel, channel polls in that channel.
export function pollHomeChannel(view: View, parent: PollData["parent"]): string | "news" | null {
  if (parent === "World") return "news";
  if ("Org" in parent) return view.orgs.get(slotKeyToString(parent.Org))?.channel ?? null;
  return slotKeyToString(parent.Channel);
}
