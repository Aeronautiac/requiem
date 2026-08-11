// Handlers for server commands — the state only the server can compute, delivered on the same
// wire as engine commands and handled through the same table.
import type { Handlers } from "./index";

export const serverHandlers: Handlers = {
  // A filtered channel record: an autopsy of a target's record, or a tapped channel's log. It is
  // a channel display like a bug or contact log — routed to whoever it is owed, kept per-view so
  // a later entry (or a replay) appends to the same record. Rendering is not wired yet: see the
  // TODO on view.svelte.ts's log record.
  LogDump(ctx, p) {
    ctx.view.apply_log_dump(p.log_type, p.data);
  },

  // What the SERVER knows about who occupies the slots, routed by the same view gates as any
  // command. A view only names slots it already holds; the name for one it does not hold yet is
  // kept so a later MapActor can pick it up.
  ProfileRoster(ctx, p) {
    ctx.view.apply_profiles(p.profiles);
  },

  // The whole key ledger, gated Admin so it lands in the System view only. Replacement wholesale:
  // the roster is whole, so the management surface renders exactly what the server currently holds.
  KeyRoster(ctx, p) {
    ctx.view.apply_keys(p.keys);
  },

  // The game's clock anchor: game time as of a real-world sent_at. Rides the world-data viewport,
  // so it lands on every view that can read the world — held per-view, read where game time is
  // rendered.
  GameClock(ctx, p) {
    ctx.view.set_game_clock(p.sent_at, ctx.timestamp);
  },
};
