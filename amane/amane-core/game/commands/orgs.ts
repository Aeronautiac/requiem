import { slotKeyToString } from "../../bindings";
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
};
