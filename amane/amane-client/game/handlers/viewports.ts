import { slotKeyToString } from "../../bindings.ts";
import type { Handlers } from "./index.ts";

export const viewportHandlers: Handlers = {
  // Access gained. Everything previously addressed to the viewport is handed over right here, out
  // of the log: the server only backfills a viewport the whole CONNECTION lacked, and this view
  // may not be the holder it sent it to.
  EnterViewport(ctx, p) {
    const key = slotKeyToString(p.viewport);
    ctx.view.viewports.add(key);
    ctx.view.seen_viewports.add(key);
    ctx.backfill(key, ctx.pos);
  },

  // Access lost. Nothing already received is dropped; this only means no more is coming, which is
  // what `frozen` says out loud.
  ExitViewport(ctx, p) {
    ctx.view.viewports.delete(slotKeyToString(p.viewport));
  },

  // What a viewport belongs to, said on the viewport as it is allocated. Only WorldEvents needs
  // recording: it belongs to no object, so nothing else would ever name it. Every other viewport
  // is identified by the content that rides it.
  MapViewport(ctx, p) {
    if (p.kind === "WorldEvents") ctx.view.world_events_viewport = slotKeyToString(p.viewport);
  },
};
