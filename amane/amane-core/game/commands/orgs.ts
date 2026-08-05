import { slotKeyToString } from "../../bindings";
import { orgDisplayName, t } from "../helpers.svelte";
import type { CmdCtx, Handlers } from "./index";

export const orgHandlers: Handlers = {
  // The roster, which every member sees in full. Dead members stay listed.
  AddOrgMember(ctx: CmdCtx, p) {
    ctx.view.orgs.get(slotKeyToString(p.org_id))?.members.add(slotKeyToString(p.player_id));
  },

  RemoveOrgMember(ctx: CmdCtx, p) {
    ctx.view.orgs.get(slotKeyToString(p.org_id))?.members.delete(slotKeyToString(p.player_id));
  },

  // The present subset that counts toward ability requirements. Whole set every time — replace it,
  // don't merge, so a member who regained presence is added and one who lost it is dropped in one
  // shot.
  OrgEffectiveMembers(ctx: CmdCtx, p) {
    const org = ctx.view.orgs.get(slotKeyToString(p.org_id));
    if (!org) return;
    org.effective.clear();
    for (const member of p.members) org.effective.add(slotKeyToString(member));
  },

  OrgLeader(ctx: CmdCtx, p) {
    const org = ctx.view.orgs.get(slotKeyToString(p.org_id));
    if (!org) return;
    const new_leader = p.leader ? slotKeyToString(p.leader) : null;
    org.leader = new_leader;
  },

  // Directed at the one player whose leadership changed. It carries the same `org.leader` truth the
  // admin gets on OrgLeader, but from this view's vantage: gaining it means you are now the leader
  // you know of, losing it clears the field back to "unknown". A member never learns who else leads.
  LeaderStatus(ctx: CmdCtx, p) {
    const org_key = slotKeyToString(p.org_id);
    const org = ctx.view.orgs.get(org_key);
    if (org) org.leader = p.leader ? ctx.view.own_key : null;

    ctx.view.push_notif(ctx.timestamp, { LeaderStatus: { org_id: org_key, leader: p.leader } });
    const name = org ? orgDisplayName(org.name) : t("display_org_unknown");
    ctx.notify({
      title: t("toast_leader_title"),
      body: p.leader ? t("toast_leader_gained", { org: name }) : t("toast_leader_lost", { org: name }),
    });
  },
};
