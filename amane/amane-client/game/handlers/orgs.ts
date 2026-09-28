import { slotKeyToString } from "../../bindings.ts";
import type { Handlers } from "./index.ts";

export const orgHandlers: Handlers = {
  // The roster, which every member sees in full. Dead members stay listed.
  AddOrgMember(ctx, p) {
    ctx.view.orgs.get(slotKeyToString(p.org_id))?.members.add(slotKeyToString(p.player_id));
  },

  RemoveOrgMember(ctx, p) {
    ctx.view.orgs.get(slotKeyToString(p.org_id))?.members.delete(slotKeyToString(p.player_id));
  },

  // The present subset that counts toward ability requirements. Whole set every time — replace it,
  // don't merge, so a member who regained presence is added and one who lost it is dropped in one
  // shot.
  OrgEffectiveMembers(ctx, p) {
    const org = ctx.view.orgs.get(slotKeyToString(p.org_id));
    if (!org) return;
    org.effective.clear();
    for (const member of p.members) org.effective.add(slotKeyToString(member));
  },

  OrgLeader(ctx, p) {
    const org = ctx.view.orgs.get(slotKeyToString(p.org_id));
    if (!org) return;
    org.leader = p.leader ? slotKeyToString(p.leader) : null;
  },

  // Directed at the one player whose leadership changed. It carries the same `org.leader` truth the
  // admin gets on OrgLeader, but from this view's vantage: gaining it means you are now the leader
  // you know of, losing it clears the field back to "unknown". A member never learns who else leads.
  LeaderStatus(ctx, p) {
    const org_key = slotKeyToString(p.org_id);
    const org = ctx.view.orgs.get(org_key);
    if (org) org.leader = p.leader ? ctx.view.own_key : null;
    ctx.view.push_notif(ctx.timestamp, { LeaderStatus: { org_id: org_key, leader: p.leader } });
  },
};
