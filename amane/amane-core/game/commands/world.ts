// World events: this view's news feed. Most are delivered on the world-events viewport, so a view
// that has left it — by losing presence, or because the world went dark — simply stops receiving
// them, and there is no gate here to forget.
//
// NewIteration and Blackout are the exceptions: they ride world data, which nothing takes away.
// They land in the same feed because that is where a reader looks for them, not because they share
// a delivery rule.
//
// Each handler writes the event into the feed, then composes its own toast and raises it. The two
// are separate acts on purpose: the feed is a record, the toast is an alert, and only the handler
// knows the words for either.
import { slotKeyToString } from "../../bindings";
import { nameLabel, orgDisplayName, playerLabel, t } from "../helpers.svelte";
import { formatDuration } from "../../lib/utils";
import type { CmdCtx, Handlers } from "./index";

export const worldHandlers: Handlers = {
  Death(ctx: CmdCtx, p) {
    const target_id = slotKeyToString(p.target_id);
    ctx.view.events.push({
      timestamp: ctx.timestamp,
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
    });
    ctx.notify({
      title: t("toast_death_title"),
      body: t("toast_death_body", { name: playerLabel(target_id, ctx.view.players) }),
    });
  },

  AnonymousAnnouncement(ctx: CmdCtx, p) {
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { AnonymousAnnouncement: { content: p.content } },
    });
    ctx.notify({ title: t("toast_announcement_title"), body: p.content });
  },

  FailedSilentProsecution(ctx: CmdCtx, p) {
    const accuser_id = slotKeyToString(p.accuser_id);
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { FailedSilentProsecution: { accuser_id, true_name: p.true_name, org: p.org } },
    });
    ctx.notify({
      title: t("toast_false_accusation_title"),
      body: t("toast_false_accusation_body", {
        name: playerLabel(accuser_id, ctx.view.players),
        true_name: nameLabel(p.true_name),
        org: orgDisplayName(p.org),
      }),
    });
  },

  PseudocideRevival(ctx: CmdCtx, p) {
    const target_id = slotKeyToString(p.target_id);
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { PseudocideRevival: { target_id } },
    });
    ctx.notify({
      title: t("toast_revival_title"),
      body: t("toast_revival_body", { name: playerLabel(target_id, ctx.view.players) }),
    });
  },

  NewIteration(ctx: CmdCtx, p) {
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { NewIteration: { iteration: p.iteration } },
    });
    ctx.notify(
      p.iteration === 1
        ? { title: t("toast_game_begins_title"), body: t("toast_game_begins_body") }
        : { title: t("toast_new_day_title"), body: t("toast_new_day_body", { day: p.iteration }) },
    );
  },

  Blackout(ctx: CmdCtx, p) {
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { Blackout: { active: p.active } },
    });
    ctx.notify(
      p.active
        ? { title: t("blackout_begun_label"), body: t("blackout_begun") }
        : { title: t("blackout_over_label"), body: t("blackout_over") },
    );
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
    ctx.notify({
      title: t("toast_kidnapping_title"),
      body: t("toast_kidnapping_body", { name: playerLabel(target_id, ctx.view.players) }),
    });
  },

  // The tracked entry is MARKED rather than deleted, because this handler can be replayed long
  // after the fact and the victim must still resolve.
  KidnapReveal(ctx: CmdCtx, p) {
    const kidnapping_id = slotKeyToString(p.kidnapping_id);
    const tracked = ctx.view.kidnappings.get(kidnapping_id);
    // Re-set rather than mutate: a Map tracks its own get/set, not writes into a stored value.
    if (tracked) ctx.view.kidnappings.set(kidnapping_id, { ...tracked, revealed: true });
    const kidnapper = p.kidnapper ? slotKeyToString(p.kidnapper) : null;
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: { KidnapReveal: { kidnapping_id, victim: tracked?.victim ?? null, kidnapper } },
    });
    const victim = tracked ? playerLabel(tracked.victim, ctx.view.players) : t("toast_kidnap_reveal_unknown_victim");
    ctx.notify({
      title: t("toast_kidnap_reveal_title"),
      body: kidnapper
        ? t("toast_kidnap_reveal_named", { victim, kidnapper: playerLabel(kidnapper, ctx.view.players) })
        : t("toast_kidnap_reveal_anonymous", { victim }),
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
    const name = playerLabel(victim_id, ctx.view.players);
    ctx.notify({
      title: t("toast_incarceration_title"),
      body: p.duration
        ? t("toast_incarceration_timed", { name, duration: formatDuration(p.duration) })
        : t("toast_incarceration_body", { name }),
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
    ctx.notify({
      title: t("toast_release_title"),
      body: t("toast_release_body", {
        name: tracked ? playerLabel(tracked.victim, ctx.view.players) : t("toast_release_unknown"),
      }),
    });
  },
};
