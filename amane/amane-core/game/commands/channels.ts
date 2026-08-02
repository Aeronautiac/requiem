// Channels: their registration, their content, and this actor's standing in them.
//
// Every Map* here does the same two things — put the Channel in the view, and record the viewport
// it arrived on. The second half is easy to forget, and a channel registered without it can never
// answer whether it has gone quiet, so both go through `mapChannel`.
import { slotKeyToString } from "../../bindings";
import type { ChannelCategory } from "../types";
import { new_channel, orgDisplayName, ownPerms, t } from "../helpers.svelte";
import { type CmdCtx, type Handlers } from "./index";

function mapChannel(ctx: CmdCtx, key: string, category: ChannelCategory, name: string) {
  ctx.view.channels.set(key, new_channel("Standard", category, name));
  ctx.view.record_viewport(ctx.viewport, key);
}

export const channelHandlers: Handlers = {
  // Every channel in the game arrives here; `kind` is what it belongs to, and the only thing that
  // varies is how it is filed and named.
  MapChannel(ctx: CmdCtx, p) {
    const key = slotKeyToString(p.channel_id);
    const kind = p.kind;

    // A real, sendable channel private to its owner. Visibility falls out of channel perms, since
    // only the owner is ever a member.
    if (kind === "Personal") {
      if (ctx.view.channels.has(key)) return;
      mapChannel(ctx, key, "Personal", `personal-${p.channel_id.idx}v${p.channel_id.version}`);
      return;
    }

    // Ordinary sendable-per-perms channels; only the sidebar grouping varies.
    if ("World" in kind) {
      const category: ChannelCategory = kind.World === "LAndWatari" ? "Role" : "World";
      // News must appear to exist even after the underlying channel is removed — world events
      // render into it regardless of the channel or this view's perms.
      if (kind.World === "News") ctx.view.news_channel_id = key;
      mapChannel(ctx, key, category, kind.World);
      return;
    }

    // Lounges and group chats are named by their contact-channel id, which is the number a tap-in
    // guesses at.
    if ("Lounge" in kind) {
      mapChannel(ctx, key, "Lounge", `lounge-${kind.Lounge.contact_id}`);
      ctx.view.map_lounge(key, slotKeyToString(kind.Lounge.lounge_id));
      return;
    }

    if ("Groupchat" in kind) {
      // Custom gc names arrive with the server.
      mapChannel(ctx, key, "Groupchat", `groupchat-${kind.Groupchat.contact_id}`);
      ctx.view.map_gc(key, slotKeyToString(kind.Groupchat.gc_id));
      return;
    }

    if ("Notebook" in kind) {
      mapChannel(ctx, key, "Notebook", `Death Notebook-${kind.Notebook.idx}v${kind.Notebook.version}`);
      ctx.view.map_notebook(key, slotKeyToString(kind.Notebook));
      return;
    }

    // The org itself was registered by the MapActor immediately before this, which is what the
    // channel is named after. This is also where the org's viewport becomes identifiable — its
    // abilities and roster arrive through it afterwards.
    if ("Org" in kind) {
      const org_key = slotKeyToString(kind.Org);
      const org = ctx.view.orgs.get(org_key);
      mapChannel(ctx, key, "Org", org ? orgDisplayName(org.name) : t("display_org_unknown"));
      ctx.view.map_org_channel(key, org_key);
      ctx.view.record_org_viewport(ctx.viewport, org_key);
      return;
    }

    // TODO:
    // proper handling for kidnap channel names
    // The names will be things like:
    // "public-${victim-name}"
    // "anonymous-${victim-name}"
    if ("Kidnapping" in kind) {
      const kidnapping_key = kind.Kidnapping;
      mapChannel(ctx, key, "Kidnapping", `kidnapping-${kidnapping_key.idx}v${kidnapping_key.version}`);
    }

    // The defendant's private line to their lawyer rides its own viewport, so only those two ever
    // learn it exists. The trial's own channel is registered here too rather than from the
    // UpdateProsecution that names it — that rides presence, and filing the channel against THAT
    // viewport would mean everything said in the trial arrives on a viewport it was never mapped
    // to.
    if ("Lawyer" in kind || "Trial" in kind) {
      const prosecution_id = "Lawyer" in kind ? kind.Lawyer : kind.Trial;
      const label = "Lawyer" in kind ? "lawyer" : "trial";
      mapChannel(
        ctx,
        key,
        "Prosecution",
        `${label}-${prosecution_id.idx}v${prosecution_id.version}`,
      );
      ctx.view.map_prosecution_channel(key, slotKeyToString(prosecution_id));

    }
  },

  // Tearing a channel down is always archival — nothing said in it can be un-said.
  ArchiveChannel(ctx: CmdCtx, p) {
    const channel = ctx.view.channels.get(slotKeyToString(p.channel_id));
    if (channel) channel.archived = true;
  },

  SetChannelLoggable(ctx: CmdCtx, p) {
    ctx.view.set_loggable(slotKeyToString(p.channel_id), p.loggable);
  },

  AddMessage(ctx: CmdCtx, p) {
    const key = slotKeyToString(p.channel_id);
    ctx.view.channels.get(key)?.events.push({
      timestamp: ctx.timestamp,
      data: { Message: { content: p.content, sender_display: p.sender_display } },
    });
  },

  // Unlike a message this carries no display: the user is named raw, which is the whole cost of
  // the ability.
  KiraConnectionAttempt(ctx: CmdCtx, p) {
    const key = slotKeyToString(p.channel_id);
    ctx.view.channels.get(key)?.events.push({
      timestamp: ctx.timestamp,
      data: { KiraConnectionAttempt: { user: slotKeyToString(p.user), success: p.success } },
    });
  },

  // Carries no reader — the members learn they were tapped, never by whom. That gap is what makes
  // tapping a line you are on yourself a move rather than a waste.
  ChannelTapped(ctx: CmdCtx, p) {
    const key = slotKeyToString(p.channel_id);
    ctx.view.channels.get(key)?.events.push({
      timestamp: ctx.timestamp,
      data: { ChannelTapped: {} },
    });
  },

  // The names in a channel that are THIS view's to speak as. Holding one is what membership is, so
  // this is what creates the entry — and an empty set is a member who holds nothing, not a member
  // who was never told.
  ProfileAccess(ctx: CmdCtx, p) {
    const channel_id = slotKeyToString(p.channel_id);
    const existing = ctx.view.channel_views.get(channel_id);
    const could_read = ownPerms(existing?.own ?? []).read;
    const can_read = ownPerms(p.profiles).read;

    // Re-set the key rather than mutating in place, so the swap triggers reactivity.
    ctx.view.channel_views.set(channel_id, {
      roster: existing?.roster ?? [],
      own: p.profiles,
      owners: existing?.owners ?? [],
    });

    // A notebook channel going from no-read to read means the book is now in this view's hands.
    // Derived rather than delivered; fires once per gain, not on refreshes while it is held.
    if (can_read && !could_read && ctx.view.is_notebook_channel(channel_id)) {
      ctx.view.push_notif(ctx.timestamp, { NotebookReceived: {} });
      ctx.notify({ title: t("toast_notebook_received_title"), body: t("toast_notebook_received_body") });
    }
  },

  // Every name the room can see, whole, every time. Replaced rather than merged: a roster is the
  // current answer, and a name that has left it has stopped being in the room.
  ChannelRoster(ctx: CmdCtx, p) {
    const channel_id = slotKeyToString(p.channel_id);
    const existing = ctx.view.channel_views.get(channel_id);
    ctx.view.channel_views.set(channel_id, {
      roster: p.profiles,
      own: existing?.own ?? [],
      owners: existing?.owners ?? [],
    });
  },

  // SYSTEM only, so this only ever lands in the admin view. Who is behind each name in the roster,
  // whole set every time — replaced like the roster it accompanies. Ordinary viewers never receive
  // it, so their `owners` stays empty and nothing behind a mask is exposed.
  ProfileOwnership(ctx: CmdCtx, p) {
    const channel_id = slotKeyToString(p.channel_id);
    const existing = ctx.view.channel_views.get(channel_id);
    ctx.view.channel_views.set(channel_id, {
      roster: existing?.roster ?? [],
      own: existing?.own ?? [],
      owners: p.owners,
    });
  },

  GcOwnerStatus(ctx: CmdCtx, p) {
    const gc_key = slotKeyToString(p.gc_id);
    if (p.owner) ctx.view.owned_gcs.add(gc_key);
    else ctx.view.owned_gcs.delete(gc_key);
  },
};
