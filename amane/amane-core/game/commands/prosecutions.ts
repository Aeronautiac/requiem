import { slotKeyToString } from "../../bindings";
import { actorLabel, phaseAnnouncement, phaseViewEqual, t } from "../helpers.svelte";
import type { CmdCtx, Handlers } from "./index";

export const prosecutionHandlers: Handlers = {
  // The snapshot, plus a news event when the phase differs from what this view last held. The diff
  // is per-view, which is what makes an absent player's backfill reproduce the ordered timeline
  // when they return.
  UpdateProsecution(ctx: CmdCtx, p) {
    const key = slotKeyToString(p.prosecution_id);
    ctx.view.record_prosecution_viewport(ctx.viewport, key);

    const prev = ctx.view.prosecutions.get(key);
    ctx.view.prosecutions.set(key, {
      prosecutor_display: p.prosecutor_display,
      defendant_display: p.defendant_display,
      lawyer_display: p.lawyer_display,
      phase: p.phase,
      trial_channel: p.trial_channel ? slotKeyToString(p.trial_channel) : null,
    });

    if (!prev || !phaseViewEqual(prev.phase, p.phase)) {
      ctx.view.events.push({
        timestamp: ctx.timestamp,
        data: {
          ProsecutionEvent: {
            prosecution_id: key,
            prosecutor_display: p.prosecutor_display,
            defendant_display: p.defendant_display,
            phase: p.phase,
            ended: false,
            verdict: null,
          },
        },
      });
      ctx.notify({
        title: t("toast_prosecution_title"),
        body: phaseAnnouncement(
          p.phase,
          actorLabel(p.prosecutor_display, ctx.view.players),
          actorLabel(p.defendant_display, ctx.view.players),
          false,
        ),
      });
    }
  },

  // Directed: you are a party to this one. Nothing is looked up — the snapshot may not have
  // arrived yet, and a view that never receives one still knows this much.
  InProsecution(ctx: CmdCtx, p) {
    ctx.view.own_prosecutions.set(slotKeyToString(p.prosecution_id), p.side);
  },

  // If this view knew the prosecution, drop a terminal news event using the displays it last held.
  // A view absent for the whole thing receives the ordered timeline on entry and reaches the same
  // place.
  CloseProsecution(ctx: CmdCtx, p) {
    const key = slotKeyToString(p.prosecution_id);
    ctx.view.own_prosecutions.delete(key);

    const prev = ctx.view.prosecutions.get(key);
    if (!prev) return;
    ctx.view.events.push({
      timestamp: ctx.timestamp,
      data: {
        ProsecutionEvent: {
          prosecution_id: key,
          prosecutor_display: prev.prosecutor_display,
          defendant_display: prev.defendant_display,
          phase: prev.phase,
          ended: true,
          verdict: p.verdict,
        },
      },
    });
    ctx.notify({
      title: t("toast_prosecution_ended_title"),
      body: phaseAnnouncement(
        prev.phase,
        actorLabel(prev.prosecutor_display, ctx.view.players),
        actorLabel(prev.defendant_display, ctx.view.players),
        true,
        p.verdict,
      ),
    });
    ctx.view.prosecutions.delete(key);
  },
};
