// Bug feeds and contact logs: read-only records handed to a view rather than rooms it is in.
//
// Neither needs a gate of its own. A view holds one iff it was delivered the command that creates
// it, and whether it is still live is `frozen` on the viewport recorded here.
import { slotKeyToString } from "../../bindings";
import { bugChannelKey, contactLogChannelKey, new_channel } from "../helpers.svelte";
import type { CmdCtx, Handlers } from "./index";

export const feedHandlers: Handlers = {
  // Addressed to the bug's own viewport, so it is also what registers the feed against it. The
  // target is deliberately not carried — identity leaks only through relayed message displays —
  // so the feed is named by the bug's slot.
  NewBug(ctx: CmdCtx, p) {
    const key = bugChannelKey(p.bug_key);
    if (!ctx.view.bugs.has(key)) {
      ctx.view.bugs.set(
        key,
        new_channel("Bug", "Logs", `bug-${p.bug_key.idx}v${p.bug_key.version}`),
      );
    }
    ctx.view.record_viewport(ctx.viewport, key);
  },

  // The sender display is the target's own, which is what reveals them.
  AddBugMessage(ctx: CmdCtx, p) {
    ctx.view.bugs.get(bugChannelKey(p.bug_key))?.events.push({
      timestamp: ctx.timestamp,
      data: { Message: { sender_display: p.display, content: p.content } },
    });
  },

  // The bug is no longer active, but its feed stays readable.
  ArchiveBug(ctx: CmdCtx, p) {
    const bug = ctx.view.bugs.get(bugChannelKey(p.bug_key));
    if (bug) bug.archived = true;
  },

  // The feed is created on its first entry — nothing else is addressed to a contact-log viewport,
  // so there is no creation command to hang it off. The record IS its ContactLogType (Full/Even/Odd)
  // now that the entry carries it, so it names and keys the feed directly — the same feed reaches a
  // linked reader who never learns which passive fed it.
  AddContactLog(ctx: CmdCtx, p) {
    const key = contactLogChannelKey(p.kind);
    let feed = ctx.view.contact_logs.get(key);
    if (!feed) {
      feed = new_channel("ContactLog", "Logs", `Contact Log (${p.kind})`);
      ctx.view.contact_logs.set(key, feed);
    }
    ctx.view.record_viewport(ctx.viewport, key);
    feed.events.push({ timestamp: ctx.timestamp, data: { ContactLogEntry: p.log } });
  },
};
