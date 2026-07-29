import { slotKeyToString } from "../../bindings";
import type { CmdCtx, Handlers } from "./index";

export const viewportHandlers: Handlers = {
  // Access gained. Everything previously addressed to the viewport is handed over right here, out
  // of the log — the server only sends a backfill for a viewport the whole CONNECTION lacked, and
  // this view may not be the holder it sent it to.
  EnterViewport(ctx: CmdCtx, p) {
    const key = slotKeyToString(p.viewport);
    ctx.view.viewports.add(key);
    ctx.view.seen_viewports.add(key);
    ctx.backfill(key, ctx.pos);
  },

  // Access lost. Nothing already received is dropped — this only means no more is coming, which
  // is what `frozen` goes on to say out loud.
  ExitViewport(ctx: CmdCtx, p) {
    ctx.view.viewports.delete(slotKeyToString(p.viewport));
  },

  // What an object a viewport belongs to, said on the viewport itself as it is allocated.
  //
  // Presence is the only kind worth acting on here: it belongs to no object, so nothing else would
  // ever name it, and this is what lets news answer whether it has gone quiet. Every other
  // viewport is identified by the content that rides it — a channel by its map, a bug by NewBug —
  // and the kind adds nothing to that. The record never arrives at all; the server keeps it.
  MapViewport(ctx: CmdCtx, p) {
    if (p.kind === "Presence") ctx.view.record_presence_viewport(slotKeyToString(p.viewport));
  },
};
