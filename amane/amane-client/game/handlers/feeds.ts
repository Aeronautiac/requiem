// Bug feeds and contact logs: read-only records handed to a view rather than rooms it is in. Both
// live in the one channels map alongside real channels, keyed by their own prefix.
//
// Neither needs a gate of its own. A view holds one iff it was delivered the command that creates
// it, and whether it is still live is `frozen` on the viewport recorded here.
import { bugChannelKey, contactLogChannelKey } from "../keys.ts";
import type { Handlers } from "./index.ts";

export const feedHandlers: Handlers = {
  // Addressed to the bug's own viewport, so it is also what records the feed's viewport. The
  // target is deliberately not carried — identity leaks only through relayed message displays —
  // so the feed is named by the bug's slot.
  NewBug(ctx, p) {
    const key = bugChannelKey(p.bug_key);
    let channel = ctx.view.channels.get(key);
    if (!channel) {
      channel = {
        kind: "Bug",
        category: "Logs",
        name: `bug-${p.bug_key.idx}v${p.bug_key.version}`,
        archived: false,
        loggable: false,
        viewport: null,
        link: null,
        events: [],
      };
      ctx.view.channels.set(key, channel);
    }
    if (ctx.viewport !== undefined) channel.viewport = ctx.viewport;
  },

  // The sender display is the target's own, which is what reveals them.
  AddBugMessage(ctx, p) {
    ctx.view.channels.get(bugChannelKey(p.bug_key))?.events.push({
      timestamp: ctx.timestamp,
      data: { Message: { sender_display: p.display, content: p.content } },
    });
  },

  // The bug is no longer active, but its feed stays readable.
  ArchiveBug(ctx, p) {
    const bug = ctx.view.channels.get(bugChannelKey(p.bug_key));
    if (bug) bug.archived = true;
  },

  // The feed is created on its first entry — nothing else is addressed to a contact-log viewport,
  // so there is no creation command to hang it off. The record IS its ContactLogType (Full/Even/Odd)
  // now that the entry carries it, so it names and keys the feed directly — the same feed reaches a
  // linked reader who never learns which passive fed it.
  AddContactLog(ctx, p) {
    const key = contactLogChannelKey(p.kind);
    let feed = ctx.view.channels.get(key);
    if (!feed) {
      feed = {
        kind: "ContactLog",
        category: "Logs",
        name: `Contact Log (${p.kind})`,
        archived: false,
        loggable: false,
        viewport: null,
        link: null,
        events: [],
      };
      ctx.view.channels.set(key, feed);
    }
    if (ctx.viewport !== undefined) feed.viewport = ctx.viewport;
    feed.events.push({ timestamp: ctx.timestamp, data: { ContactLogEntry: p.log } });
  },
};
