// What the org panel needs about membership and which channel an org's abilities ride. Grouping and
// lookups only — presentation stays in the component.
import type { View } from "../game/view.ts";
import type { Org } from "../game/types.ts";

export type OrgMemberRow = { id: string; effective: boolean; leader: boolean; og: boolean };

// The org whose backing channel this is, found off the channel's link — set once, by that org's own
// MapChannel, so this is a lookup rather than a search.
export function orgOfChannel(view: View, channelId: string | null): [string, Org] | null {
  const link = channelId ? view.channels.get(channelId)?.link : null;
  const orgId = link && "org" in link ? link.org : null;
  const org = orgId ? view.orgs.get(orgId) : undefined;
  return orgId && org ? [orgId, org] : null;
}

// As far as THIS view is entitled to know, which needs no check of who is looking: OG standing
// reaches the member and System and nobody else, so `players` carries og_orgs only on System and a
// view's own `og_orgs` holds only its own.
export function isOg(view: View, orgId: string, playerId: string): boolean {
  if (view.players.get(playerId)?.og_orgs?.has(orgId)) return true;
  return playerId === view.own_key && view.og_orgs.has(orgId);
}

// The full membership, each row carrying whether it counts toward the org's ability member
// requirements right now (an absent member — kidnapped, jailed, dead — stays a member but drops
// out of `effective`).
export function orgMembers(view: View, orgId: string, org: Org): OrgMemberRow[] {
  return [...org.members].map((id) => ({
    id,
    effective: org.effective.has(id),
    leader: org.leader === id,
    og: isOg(view, orgId, id),
  }));
}
