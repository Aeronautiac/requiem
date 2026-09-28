// World events: this view's news feed. Most are delivered on the world-events viewport, so a view
// that has left it — by losing presence, or because the world went dark — simply stops receiving
// them, and there is no gate here to forget.
//
// NewIteration and Blackout are the exceptions: they ride world data, which nothing takes away.
// They land in the same feed because that is where a reader looks for them, not because they share
// a delivery rule.
import { slotKeyToString } from "../../bindings.ts";
import type { GameEvent } from "../types.ts";
import type { Handlers } from "./index.ts";

// A death is dealt out as up to four beats, this far apart: the death itself, then who they were,
// then their org standing, then what they left behind. Beats are numbered as they are appended, so
// a skipped one (no orgs, no transfer) takes no slot and the ones after it move up rather than
// leaving a silent gap. Each is stamped at its own future timestamp; a feed query hides a beat until
// the game clock reaches it, so a fresh death plays out over these gaps while a replayed one is
// simply all there already.
const DEATH_STAGGER_MS = 5000;

export const worldHandlers: Handlers = {
  Death(ctx, p) {
    const target_id = slotKeyToString(p.target_id);
    const ts = ctx.timestamp;

    const orgs = p.orgs.map(([id, view]) => ({
      id: slotKeyToString(id),
      leader: view.leader,
      og: view.og,
    }));
    const has_transfer = p.notebook_transferred || p.ability_transferred;

    let beat = 0;
    const at = () => ts + beat++ * DEATH_STAGGER_MS;

    const beats: GameEvent[] = [
      {
        timestamp: at(),
        data: {
          Death: {
            target_id,
            true_name: p.true_name,
            death_message: p.death_message,
            role: p.role,
            notebook_transferred: p.notebook_transferred,
            ability_transferred: p.ability_transferred,
          },
        },
      },
      { timestamp: at(), data: { DeathRole: { target_id, role: p.role } } },
    ];
    if (orgs.length > 0) {
      beats.push({ timestamp: at(), data: { DeathOrgs: { target_id, orgs } } });
    }
    if (has_transfer) {
      beats.push({
        timestamp: at(),
        data: {
          DeathTransfer: {
            target_id,
            notebook_transferred: p.notebook_transferred,
            ability_transferred: p.ability_transferred,
          },
        },
      });
    }

    ctx.view.events.push(...beats);
  },

  // Public: who runs the news now (null = vacant). Rides world-events like the rest of the news.
  // The personal "you are now/no longer the anchor" is derived here from the client's own key rather
  // than delivered — the engine names the anchor, not the reader.
  NewsAnchor(ctx, p) {
    const target = p.target_id ? slotKeyToString(p.target_id) : null;
    const previous = ctx.view.news_anchor;
    ctx.view.news_anchor = target;
    ctx.view.events.push({ timestamp: ctx.timestamp, data: { NewsAnchor: { target_id: target } } });

    const own = ctx.view.own_key;
    if (target === own) {
      ctx.view.push_notif(ctx.timestamp, { NewsAnchorStatus: { holding: true } });
    } else if (previous === own) {
      ctx.view.push_notif(ctx.timestamp, { NewsAnchorStatus: { holding: false } });
    }
  },

  // A press-conference roster change: someone gained or lost the right to speak on the news.
  PressConfStatus(ctx, p) {
    const target_id = slotKeyToString(p.target_id);
    if (p.has_access) ctx.view.press_conf.add(target_id);
    else ctx.view.press_conf.delete(target_id);
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { PressConfStatus: { target_id, has_access: p.has_access } },
    });
    // Personal notif derived from the public roster change, the same way NewsAnchor does it: only
    // when the actor named is the client's own key.
    if (target_id === ctx.view.own_key) {
      ctx.view.push_notif(ctx.timestamp, { PressConfMembership: { in_conf: p.has_access } });
    }
  },

  AnonymousAnnouncement(ctx, p) {
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { AnonymousAnnouncement: { content: p.content } },
    });
  },

  EyeDealTaken(ctx, p) {
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { EyeDealTaken: { user: p.user } },
    });
  },

  FailedSilentProsecution(ctx, p) {
    const accuser_id = slotKeyToString(p.accuser_id);
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { FailedSilentProsecution: { accuser_id, true_name: p.true_name, org: p.org } },
    });
  },

  Revival(ctx, p) {
    const target_id = slotKeyToString(p.target_id);
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { Revival: { target_id, message: p.message } },
    });
  },

  NewIteration(ctx, p) {
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { NewIteration: { iteration: p.iteration } },
    });
  },

  Blackout(ctx, p) {
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { Blackout: { active: p.active } },
    });
  },

  // Tracked as well as announced: the reveal below carries only the id, so the victim is a lookup
  // here, and a replay of that reveal has to resolve the same one.
  Kidnapping(ctx, p) {
    const kidnapping_id = slotKeyToString(p.kidnapping_id);
    const target_id = slotKeyToString(p.target_id);
    ctx.view.kidnappings.set(kidnapping_id, { victim: target_id, duration: p.duration, revealed: false });
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { Kidnapping: { kidnapping_id, target_id, duration: p.duration } },
    });
  },

  // The tracked entry is MARKED rather than deleted, because this handler can be replayed long
  // after the fact and the victim must still resolve.
  KidnapReveal(ctx, p) {
    const kidnapping_id = slotKeyToString(p.kidnapping_id);
    const tracked = ctx.view.kidnappings.get(kidnapping_id);
    if (tracked) tracked.revealed = true;
    const kidnapper = p.kidnapper ? slotKeyToString(p.kidnapper) : null;
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { KidnapReveal: { kidnapping_id, victim: tracked?.victim ?? null, kidnapper } },
    });
  },

  Incarceration(ctx, p) {
    const incarceration_id = slotKeyToString(p.incarceration_id);
    const victim_id = slotKeyToString(p.victim_id);
    ctx.view.incarcerations.set(incarceration_id, { victim: victim_id, duration: p.duration, released: false });
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { Incarceration: { incarceration_id, victim_id, duration: p.duration } },
    });
  },

  IncarcerationReleased(ctx, p) {
    const incarceration_id = slotKeyToString(p.incarceration_id);
    const tracked = ctx.view.incarcerations.get(incarceration_id);
    if (tracked) tracked.released = true;
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { IncarcerationReleased: { incarceration_id, victim: tracked?.victim ?? null } },
    });
  },
};
