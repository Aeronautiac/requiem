// Players, and what one actor holds: abilities, passives, states, and the answers addressed
// privately to it.
//
// Several of these arrive twice from the engine, addressed to two different recipients — once to
// the actor it concerns and once to System. Both copies land here, in different views, and the
// recipient is what says which of the two this is.
import { slotKeyToString } from "../../bindings.ts";
import { upsertAbility } from "../abilities.ts";
import type { Handlers } from "./index.ts";

export const actorHandlers: Handlers = {
  // A slot exists, and `kind` says what holds it.
  //
  // For a player the engine says nothing about WHO is on the slot; that arrives on the profile
  // channel, and may never arrive at all, so the entry starts unnamed and renders as
  // `player-<slot>`. An org carries its name, which is engine state — the channel that backs it
  // arrives on the MapChannel immediately after.
  MapActor(ctx, p) {
    const key = slotKeyToString(p.actor_id);
    if (p.kind === "Player") {
      if (!ctx.view.players.has(key)) ctx.view.players.set(key, { display_name: null });
      return;
    }
    ctx.view.orgs.set(key, {
      name: p.kind.Org,
      leader: null,
      members: new Set(),
      effective: new Set(),
      abilities: new Map(),
      channel: null,
      viewport: null,
    });
  },

  // An org's abilities are addressed to its channel's viewport and shared by every member; a
  // player's own are addressed to the actor. That is the only thing telling the two apart.
  UpdateAbilityView(ctx, p) {
    const org = ctx.view.org_at(ctx.viewport);
    upsertAbility(org ? org.abilities : ctx.view.abilities, p);
  },

  RemoveAbility(ctx, p) {
    const org = ctx.view.org_at(ctx.viewport);
    (org ? org.abilities : ctx.view.abilities).delete(slotKeyToString(p.ability_id));
  },

  // The static gates on an org ability, arriving just after its view on the same org viewport.
  // Patched onto the existing entry rather than replacing it, so the usage counts already there
  // survive. Only ever addressed to an org viewport, so org_at always resolves here.
  OrgAbilityRequirements(ctx, p) {
    const org = ctx.view.org_at(ctx.viewport);
    const ability = org?.abilities.get(slotKeyToString(p.ability_id));
    if (ability) ability.requirements = p.requirements;
  },

  UpdatePassiveView(ctx, p) {
    ctx.view.passives.set(slotKeyToString(p.passive_id), { type: p.passive_type });
  },

  RemovePassive(ctx, p) {
    ctx.view.passives.delete(slotKeyToString(p.passive_id));
  },

  // Carries the whole set, so it replaces rather than merges. An org's rides its channel's
  // viewport and has no home to go to yet, so it is skipped rather than written into a member's
  // own states.
  ActorState(ctx, p) {
    if (ctx.viewport === undefined) ctx.view.states = p.state;
  },

  // The public condition of one actor, whole set every time. Emitted only to the world-data
  // viewport, so any arrival is a legitimate projection — just replace what was held.
  ActorStatus(ctx, p) {
    ctx.view.actor_statuses.set(slotKeyToString(p.actor_id), p.status);
  },

  // The System copy feeds the admin inspector; the actor's own goes to their notifications. Two
  // recipients, two views, one handler.
  RoleUpdate(ctx, p) {
    if (ctx.view.own_key === "System") {
      const player = ctx.view.players.get(slotKeyToString(p.target_id));
      if (player) player.role = p.role;
      return;
    }
    ctx.view.own_role = p.role;
    ctx.view.push_notif(ctx.timestamp, { RoleUpdate: { role: p.role } });
  },

  TrueNameUpdate(ctx, p) {
    if (ctx.view.own_key === "System") {
      const player = ctx.view.players.get(slotKeyToString(p.target_id));
      if (player) player.true_name = p.true_name;
      return;
    }
    ctx.view.push_notif(ctx.timestamp, { TrueNameUpdate: { true_name: p.true_name } });
  },

  // A player's background check is private to them; an org's (background check, true-name invite,
  // shinigami sacrifice) is viewport-addressed to the org's channel, where everyone who can see the
  // org learns what it bought. Same routing split as TapInResult — orgs have no notif feed of their
  // own yet, so an org copy lands in the channel rather than in each member's personal feed.
  RevealTrueName(ctx, p) {
    const target_id = slotKeyToString(p.target_id);
    const data = { RevealTrueName: { target_id, true_name: p.true_name } };
    const org_channel = ctx.view.org_at(ctx.viewport)?.channel;
    if (org_channel) {
      ctx.view.channels.get(org_channel)?.events.push({ timestamp: ctx.timestamp, data });
      return;
    }
    ctx.view.push_notif(ctx.timestamp, data);
  },

  RevealNotebookHolding(ctx, p) {
    const target_id = slotKeyToString(p.target_id);
    ctx.view.push_notif(ctx.timestamp, { RevealNotebookHolding: { target_id, holding: p.holding } });
  },

  // Directed to the player whose eyes changed. Personal, like the reveals above: it lands in their
  // notifications.
  EyeCount(ctx, p) {
    ctx.view.push_notif(ctx.timestamp, { EyeCount: { count: p.count } });
  },

  // A tap landed on a fabricated lounge. The creator (this player) is told who read it; the System
  // copy is the admin's mirror. The tapper is never told their identity was handed over.
  FakeLoungeTapped(ctx, p) {
    ctx.view.push_notif(ctx.timestamp, { FakeLoungeTapped: { display: p.display } });
  },

  // Who planted it is deliberately not carried; `context` says only why.
  Bugged(ctx, p) {
    ctx.view.push_notif(ctx.timestamp, { Bugged: { context: p.context } });
  },

  // A player's tap-in answer is private to whoever asked. An org's is not personal: it is
  // viewport-addressed and lands once in the org's channel, where everyone who could have voted
  // for the tap sees it. Orgs have no notification feed of their own yet.
  TapInResult(ctx, p) {
    const org_channel = ctx.view.org_at(ctx.viewport)?.channel;
    if (org_channel) {
      ctx.view.channels.get(org_channel)?.events.push({
        timestamp: ctx.timestamp,
        data: { TapInResult: { contact_id: p.contact_id, outcome: p.outcome } },
      });
      return;
    }
    ctx.view.push_notif(ctx.timestamp, { TapInResult: { contact_id: p.contact_id, outcome: p.outcome } });
  },

  // OG standing is personal info: it reaches the member and System, and nobody else in the org.
  OgStatus(ctx, p) {
    const org_key = slotKeyToString(p.org_id);
    if (ctx.view.own_key === "System") {
      const player = ctx.view.players.get(slotKeyToString(p.target_id));
      if (!player) return;
      const orgs = player.og_orgs ?? new Set<string>();
      if (p.og) orgs.add(org_key);
      else orgs.delete(org_key);
      player.og_orgs = orgs;
      return;
    }
    if (p.og) ctx.view.og_orgs.add(org_key);
    else ctx.view.og_orgs.delete(org_key);
  },
};
