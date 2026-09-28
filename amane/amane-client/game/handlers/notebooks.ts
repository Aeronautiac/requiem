import { slotKeyToString } from "../../bindings.ts";
import type { Handlers } from "./index.ts";

export const notebookHandlers: Handlers = {
  // A write is stored on the notebook's channel exactly like a message.
  NotebookWrite(ctx, p) {
    const notebook_id = slotKeyToString(p.notebook_id);
    const channel_key = ctx.view.notebooks.get(notebook_id)?.channel;
    if (!channel_key) return;

    ctx.view.channels.get(channel_key)?.events.push({
      timestamp: ctx.timestamp,
      data: {
        Write: {
          user_id: slotKeyToString(p.user_id),
          notebook_id,
          message: p.message ?? "",
          true_name: p.true_name,
          delay: p.delay,
          successes_remaining: p.successes_remaining,
          attempts_remaining: p.attempts_remaining,
          success: p.success,
          target_saved: p.target_saved,
        },
      },
    });
  },

  // "The book in your hands is not yours" — a fact about one holder, not something the channel
  // carried, which is why it is addressed to the actor rather than to the channel.
  NotebookBorrowingStatus(ctx, p) {
    const entry = ctx.view.notebooks.get(slotKeyToString(p.notebook_id));
    if (entry) entry.borrowed = p.borrowed;
  },

  // Whether the book is a decoy — a fake book's writes can't kill. Reaches only the original owner
  // (and admin), so an inheritor is left to deduce it; an entry that never receives this shows no
  // badge rather than claiming the book is genuine.
  NotebookFakeStatus(ctx, p) {
    const entry = ctx.view.notebooks.get(slotKeyToString(p.notebook_id));
    if (entry) entry.fake = p.fake;
  },
};
