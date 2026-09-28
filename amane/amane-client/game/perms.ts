import type { ChannelProfileView } from "../bindings.ts";
import type { ChannelPerms } from "./types.ts";

// Channel permission bits, mirroring ChannelPerm in the engine.
export const PERM_SEND = 1;
export const PERM_VIEW = 2;
export const PERM_LOGGABILITY = 4;

// What this view may do in a channel, folded over every name it holds there. Permissions belong to
// the NAME, not the person: this answers "can I do X here at all", while sending asks the chosen
// name.
export function ownPerms(own: ChannelProfileView[]): ChannelPerms {
  const all = own.reduce((acc, profile) => acc | profile.perms, 0);
  return {
    read: (all & PERM_VIEW) !== 0,
    send: (all & PERM_SEND) !== 0,
    loggability_control: (all & PERM_LOGGABILITY) !== 0,
  };
}
