// What a channel's event list is, and how news merges the world feed with its own channel. Plain
// functions of a View (plus the current game time, since the caller ticks that on a clock), so a
// headless harness could reuse the same merge the UI renders.
import type { GameEvent } from "../game/types.ts";
import type { View } from "../game/view.ts";

export type FeedSelection = { kind: "news" } | { kind: "channel"; id: string };

// The events a selection renders, oldest first. News concatenates the world feed — filtered to
// what the game clock has reached, since death beats may be stamped in the future — with the News
// channel's own messages; everything else is just that channel's events.
export function feedEvents(view: View, selection: FeedSelection | null, now: number): GameEvent[] {
  if (!selection) return [];
  let events: GameEvent[] = [];
  if (selection.kind === "news") {
    events = events.concat(view.events.filter((event) => event.timestamp <= now));
    const news = view.news_channel ? view.channels.get(view.news_channel) : undefined;
    if (news) events = events.concat(news.events);
  } else {
    const channel = view.channels.get(selection.id);
    if (channel) events = events.concat(channel.events);
  }
  return events.sort((a, b) => a.timestamp - b.timestamp);
}
