// World events: this view's news feed. Each is delivered on the presence viewport, so a view that
// has left presence simply stops receiving them — there is no gate here to forget.
import { slotKeyToString } from "../../bindings";
import type { CmdCtx, Handlers } from "./index";

export const worldHandlers: Handlers = {
  Death(ctx: CmdCtx, p) {
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: {
        Death: {
          target_id: slotKeyToString(p.target_id),
          true_name: p.true_name,
          death_message: p.death_message,
          role: p.role,
          notebook_transferred: p.notebook_transferred,
          ability_transferred: p.ability_transferred,
        },
      },
    });
  },

  AnonymousAnnouncement(ctx: CmdCtx, p) {
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { AnonymousAnnouncement: { content: p.content } },
    });
  },

  FailedSilentProsecution(ctx: CmdCtx, p) {
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: {
        FailedSilentProsecution: {
          accuser_id: slotKeyToString(p.accuser_id),
          true_name: p.true_name,
          org: p.org,
        },
      },
    });
  },

  PseudocideRevival(ctx: CmdCtx, p) {
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { PseudocideRevival: { target_id: slotKeyToString(p.target_id) } },
    });
  },

  NewIteration(ctx: CmdCtx, p) {
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { NewIteration: { iteration: p.iteration } },
    });
  },

  // Tracked as well as announced: the reveal below carries only the id, so the victim is a lookup
  // here, and a replay of that reveal has to resolve the same one.
  Kidnapping(ctx: CmdCtx, p) {
    const kidnapping_id = slotKeyToString(p.kidnapping_id);
    const target_id = slotKeyToString(p.target_id);
    ctx.view.kidnappings.set(kidnapping_id, {
      victim: target_id,
      duration: p.duration,
      revealed: false,
    });
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { Kidnapping: { kidnapping_id, target_id, duration: p.duration } },
    });
  },

  // The tracked entry is MARKED rather than deleted, because this handler can be replayed long
  // after the fact and the victim must still resolve.
  KidnapReveal(ctx: CmdCtx, p) {
    const kidnapping_id = slotKeyToString(p.kidnapping_id);
    const tracked = ctx.view.kidnappings.get(kidnapping_id);
    // Re-set rather than mutate: a Map tracks its own get/set, not writes into a stored value.
    if (tracked) ctx.view.kidnappings.set(kidnapping_id, { ...tracked, revealed: true });
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: {
        KidnapReveal: {
          kidnapping_id,
          victim: tracked?.victim ?? null,
          kidnapper: p.kidnapper ? slotKeyToString(p.kidnapper) : null,
        },
      },
    });
  },

  Incarceration(ctx: CmdCtx, p) {
    const incarceration_id = slotKeyToString(p.incarceration_id);
    const victim_id = slotKeyToString(p.victim_id);
    ctx.view.incarcerations.set(incarceration_id, {
      victim: victim_id,
      duration: p.duration,
      released: false,
    });
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { Incarceration: { incarceration_id, victim_id, duration: p.duration } },
    });
  },

  IncarcerationReleased(ctx: CmdCtx, p) {
    const incarceration_id = slotKeyToString(p.incarceration_id);
    const tracked = ctx.view.incarcerations.get(incarceration_id);
    if (tracked) {
      ctx.view.incarcerations.set(incarceration_id, { ...tracked, released: true });
    }
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: {
        IncarcerationReleased: { incarceration_id, victim: tracked?.victim ?? null },
      },
    });
  },
};
