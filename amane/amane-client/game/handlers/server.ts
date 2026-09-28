// Handlers for server commands — the state only the server can compute, delivered on the same
// wire as engine commands and handled through the same table.
import { slotKeyToString } from "../../bindings.ts";
import { commandToEvent } from "../events.ts";
import { logDumpKey } from "../keys.ts";
import { logDumpLabel } from "../../text.ts";
import type { Handlers } from "./index.ts";

export const serverHandlers: Handlers = {
  // The host's record of one action request and its outcome, gated Admin so it lands in the System
  // view only. Appended as a live timeline, preserved on a replay exactly as it happened.
  LogAction(ctx, p) {
    ctx.view.action_log.push({
      id: ctx.view.action_log.length + 1,
      time: ctx.timestamp,
      action: p.action,
    });
  },

  // A filtered channel record: an autopsy of a target's record, or a tapped channel's log. It is
  // a channel like a bug or contact log — routed to whoever it is owed, kept per-view so a later
  // entry (or a replay) appends to the same record.
  LogDump(ctx, p) {
    const key = logDumpKey(p.log_type);
    let feed = ctx.view.channels.get(key);
    if (!feed) {
      feed = {
        kind: "Log",
        category: "Logs",
        name: logDumpLabel(key, ctx.view.players, ctx.view.orgs),
        archived: false,
        loggable: false,
        viewport: null,
        link: null,
        events: [],
      };
      ctx.view.channels.set(key, feed);
    }
    for (const lc of p.data) {
      const event = commandToEvent(lc.data, lc.time);
      if (event) feed.events.push(event);
    }
  },

  // What the SERVER knows about who occupies the slots, routed by the same view gates as any
  // command. A view only names slots it already holds; a player not yet mapped is skipped.
  ProfileRoster(ctx, p) {
    for (const [id, profile] of p.profiles) {
      const player = ctx.view.players.get(slotKeyToString(id));
      if (player) player.display_name = profile.display_name;
    }
  },

  // The whole key ledger, gated Admin so it lands in the System view only. Replacement wholesale:
  // the roster is whole, so the management surface renders exactly what the server currently holds.
  KeyRoster(ctx, p) {
    ctx.view.keys.clear();
    for (const [id, privileges] of p.keys) ctx.view.keys.set(id, privileges);
  },

  // The game's clock anchor: game time as of a real-world sent_at. Rides the world-data viewport,
  // so it lands on every view that can read the world.
  GameClock(ctx, p) {
    ctx.view.game_clock = { time: ctx.timestamp, sent_at: p.sent_at };
  },
};
