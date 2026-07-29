import { slotKeyToString } from "../../bindings";
import { type CmdCtx, type Handlers } from "./index";

export const notebookHandlers: Handlers = {
  // A write is stored on the notebook's channel exactly like a message.
  NotebookWrite(ctx: CmdCtx, p) {
    const notebook_id = slotKeyToString(p.notebook_id);
    const channel_key = ctx.view.channel_of_notebook(notebook_id);
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
  NotebookBorrowingStatus(ctx: CmdCtx, p) {
    ctx.view.set_notebook_borrowed(slotKeyToString(p.notebook_id), p.borrowed);
  },
};
