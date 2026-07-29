// Players, and what one actor holds: abilities, passives, states, and the answers addressed
// privately to it.
//
// Several of these arrive twice from the engine, addressed to two different recipients — once to
// the actor it concerns and once to System. Both copies land here, in different views, and the
// recipient is what says which of the two this is.
import { SvelteSet } from "svelte/reactivity";
import { slotKeyToString } from "../../bindings";
import { new_org, new_player, upsert_ability } from "../helpers.svelte";
import type { CmdCtx, Handlers } from "./index";

export const actorHandlers: Handlers = {
  // A slot exists, and `kind` says what holds it.
  //
  // For a player the engine says nothing about WHO is on the slot; that arrives on the profile
  // channel, and may never arrive at all, so the entry starts unnamed and renders as
  // `player-<slot>`. An org carries its name, which is engine state — the channel that backs it
  // arrives on the MapChannel immediately after.
  MapActor(ctx: CmdCtx, p) {
    const key = slotKeyToString(p.actor_id);
    if (p.kind === "Player") {
      if (!ctx.view.players.has(key)) ctx.view.players.set(key, new_player(null));
      return;
    }
    ctx.view.orgs.set(key, new_org(p.kind.Org));
  },

  // An org's abilities are addressed to its channel's viewport and shared by every member; a
  // player's own are addressed to the actor. That is the only thing telling the two apart.
  UpdateAbilityView(ctx: CmdCtx, p) {
    const org = ctx.view.org_at(ctx.viewport);
    upsert_ability(org ? org.abilities : ctx.view.abilities, p);
  },

  RemoveAbility(ctx: CmdCtx, p) {
    const org = ctx.view.org_at(ctx.viewport);
    (org ? org.abilities : ctx.view.abilities).delete(slotKeyToString(p.ability_id));
  },

  UpdatePassiveView(ctx: CmdCtx, p) {
    ctx.view.passives.set(slotKeyToString(p.passive_id), { type: p.passive_type });
  },

  RemovePassive(ctx: CmdCtx, p) {
    ctx.view.passives.delete(slotKeyToString(p.passive_id));
  },

  // Carries the whole set, so it replaces rather than merges. An org's rides its channel's
  // viewport and has no home to go to yet, so it is skipped rather than written into a member's
  // own states.
  ActorState(ctx: CmdCtx, p) {
    if (ctx.viewport === undefined) ctx.view.states = p.state;
  },

  // The System copy feeds the admin inspector; the actor's own goes to their notifications. Two
  // recipients, two views, one handler.
  RoleUpdate(ctx: CmdCtx, p) {
    if (ctx.recipient === "System") {
      const key = slotKeyToString(p.target_id);
      ctx.view.player_info.set(key, { ...ctx.view.player_info.get(key), role: p.role });
      return;
    }
    ctx.view.push_notif(ctx.timestamp, { RoleUpdate: { role: p.role } });
  },

  TrueNameUpdate(ctx: CmdCtx, p) {
    if (ctx.recipient === "System") {
      const key = slotKeyToString(p.target_id);
      ctx.view.player_info.set(key, {
        ...ctx.view.player_info.get(key),
        true_name: p.true_name,
      });
      return;
    }
    ctx.view.push_notif(ctx.timestamp, { TrueNameUpdate: { true_name: p.true_name } });
  },

  RevealTrueName(ctx: CmdCtx, p) {
    ctx.view.push_notif(ctx.timestamp, {
      RevealTrueName: { target_id: slotKeyToString(p.target_id), true_name: p.true_name },
    });
  },

  RevealNotebookHolding(ctx: CmdCtx, p) {
    ctx.view.push_notif(ctx.timestamp, {
      RevealNotebookHolding: { target_id: slotKeyToString(p.target_id), holding: p.holding },
    });
  },

  // Who planted it is deliberately not carried; `context` says only why.
  Bugged(ctx: CmdCtx, p) {
    ctx.view.push_notif(ctx.timestamp, { Bugged: { context: p.context } });
  },

  // A player's tap-in answer is private to whoever asked. An org's is not personal: it is
  // viewport-addressed and lands once in the org's channel, where everyone who could have voted
  // for the tap sees it. Orgs have no notification feed of their own yet.
  TapInResult(ctx: CmdCtx, p) {
    const org_channel = ctx.view.org_channel_at(ctx.viewport);
    if (org_channel) {
      ctx.view.channels.get(org_channel)?.events.push({
        timestamp: ctx.timestamp,
        data: { TapInResult: { contact_id: p.contact_id, outcome: p.outcome } },
      });
      return;
    }
    ctx.view.push_notif(ctx.timestamp, {
      TapInResult: { contact_id: p.contact_id, outcome: p.outcome },
    });
  },

  // OG standing is personal info: it reaches the member and System, and nobody else in the org.
  OgStatus(ctx: CmdCtx, p) {
    const org_key = slotKeyToString(p.org_id);
    if (ctx.recipient === "System") {
      const key = slotKeyToString(p.target_id);
      const info = ctx.view.player_info.get(key) ?? {};
      const orgs = info.og_orgs ?? new SvelteSet<string>();
      if (p.og) orgs.add(org_key);
      else orgs.delete(org_key);
      ctx.view.player_info.set(key, { ...info, og_orgs: orgs });
      return;
    }
    if (p.og) ctx.view.og_orgs.add(org_key);
    else ctx.view.og_orgs.delete(org_key);
  },
};
