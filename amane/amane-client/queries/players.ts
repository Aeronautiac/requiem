// What the Players panel shows: a channel's roster split by whether a name can do anything there,
// the true holders behind a name (System only), and the players not present in that roster at all.
// Grouping and lookups only — a component maps rows to JSX.
import type { ChannelProfileView } from "../bindings.ts";
import { slotKeyToString } from "../bindings.ts";
import type { View } from "../game/view.ts";
import { playerLabel } from "../text.ts";

export type MemberRow = {
  profile: ChannelProfileView;
  // The player behind a Raw display, or null for a role/org/anonymous name — only a Raw display
  // identifies someone to contact or inspect.
  playerId: string | null;
};

// Every name the given channel's roster holds, split by whether it can do anything there. A member
// who holds nothing is present but mute, so it renders in its own group rather than burying who can
// actually act. Both come back empty for a channel with no roster.
export function channelRoster(view: View, channelId: string | null): { active: MemberRow[]; silent: MemberRow[] } {
  const roster = (channelId ? view.channel_views.get(channelId) : undefined)?.roster ?? [];
  const active: MemberRow[] = [];
  const silent: MemberRow[] = [];
  for (const profile of roster) {
    const d = profile.display;
    const playerId = typeof d !== "string" && "Raw" in d ? slotKeyToString(d.Raw) : null;
    (profile.perms === 0 ? silent : active).push({ profile, playerId });
  }
  return { active, silent };
}

// System only: the real holders behind a roster name, as labels — null when there is nothing worth
// adding (no owners known, or a Raw display that already names its one true holder honestly).
export function trueOwners(view: View, channelId: string | null, member: ChannelProfileView): string[] | null {
  const owners = (channelId ? view.channel_views.get(channelId) : undefined)?.owners ?? [];
  const targetId = slotKeyToString(member.profile_id);
  const entry = owners.find((o) => slotKeyToString(o.profile_id) === targetId);
  if (!entry || entry.owners.length === 0) return null;
  const ids = entry.owners.map(slotKeyToString);
  const d = member.display;
  const claimed = typeof d !== "string" && "Raw" in d ? slotKeyToString(d.Raw) : null;
  if (claimed && ids.length === 1 && ids[0] === claimed) return null;
  return ids.map((id) => playerLabel(id, view.players));
}

// Players this view knows about who are not already present as a raw name in the given channel's
// roster — candidates for "Other Players".
export function otherPlayers(view: View, channelId: string | null): string[] {
  const roster = (channelId ? view.channel_views.get(channelId) : undefined)?.roster ?? [];
  const present = new Set<string>();
  for (const profile of roster) {
    const d = profile.display;
    if (typeof d !== "string" && "Raw" in d) present.add(slotKeyToString(d.Raw));
  }
  return [...view.players.keys()].filter((id) => !present.has(id));
}
