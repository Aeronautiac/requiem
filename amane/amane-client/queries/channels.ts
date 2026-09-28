// What the channel sidebar shows. Plain functions of a View, so a headless harness can reuse the
// same grouping the UI renders.
import type { ChannelCategory } from "../game/types.ts";
import type { View } from "../game/view.ts";

// Everything this view holds, grouped for the sidebar. No permission check: holding a channel at
// all means it was delivered, which is the read gate. News rides its own selection rather than a
// channel row, so its backing channel (if any) is skipped here to avoid a duplicate entry.
export function channelCategories(view: View): Map<ChannelCategory, string[]> {
  const map = new Map<ChannelCategory, string[]>();
  for (const [key, channel] of view.channels) {
    if (key === view.news_channel) continue;
    const bucket = map.get(channel.category);
    if (bucket) bucket.push(key);
    else map.set(channel.category, [key]);
  }
  return map;
}

// The ability id this view may fire to create a group chat, or undefined if it holds none. Driven
// by possession of the ability rather than an admin/player check, so System (which holds no
// abilities) is excluded the same way a player without the ability is.
export function createGroupchatAbility(view: View): string | undefined {
  for (const [id, ability] of view.abilities) {
    if (ability.name === "CreateGroupchat") return id;
  }
  return undefined;
}
