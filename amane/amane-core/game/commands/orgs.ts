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
};
