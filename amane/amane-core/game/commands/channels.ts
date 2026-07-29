// Channels: their registration, their content, and this actor's standing in them.
//
// Every Map* here does the same two things — put the Channel in the view, and record the viewport
// it arrived on. The second half is easy to forget, and a channel registered without it can never
// answer whether it has gone quiet, so both go through `mapChannel`.
import { SvelteMap } from "svelte/reactivity";
import { slotKeyToString } from "../../bindings";
import type { ChannelCategory } from "../types";
import {
  PERM_LOGGABILITY,
  PERM_SEND,
  PERM_VIEW,
  displayKey,
  hasPositivePerms,
  new_channel,
  orgDisplayName,
  t,
} from "../helpers.svelte";
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

    // The defendant's private line to their lawyer rides its own viewport, so only those two ever
    // learn it exists. The trial's own channel is registered here too rather than from the
    // UpdateProsecution that names it — that rides presence, and filing the channel against THAT
    // viewport would mean everything said in the trial arrives on a viewport it was never mapped
    // to.
    const prosecution_id = "Lawyer" in kind ? kind.Lawyer : kind.Trial;
    const label = "Lawyer" in kind ? "lawyer" : "trial";
    mapChannel(
      ctx,
      key,
      "Prosecution",
      `${label}-${prosecution_id.idx}v${prosecution_id.version}`,
    );
    ctx.view.map_prosecution_channel(key, slotKeyToString(prosecution_id));
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

  // This view's own permissions in a channel. Perms is the membership signal, so this is what
  // creates the entry.
  UpdateChannelView(ctx: CmdCtx, p) {
    const channel_id = slotKeyToString(p.channel_id);
    const read = (p.perms & PERM_VIEW) !== 0;
    const existing = ctx.view.channel_views.get(channel_id);
    const had_read = existing?.perms.read ?? false;

    // Re-set the key rather than mutating in place so perms/displays updates trigger reactivity.
    ctx.view.channel_views.set(channel_id, {
      perms: {
        read,
        send: (p.perms & PERM_SEND) !== 0,
        loggability_control: (p.perms & PERM_LOGGABILITY) !== 0,
      },
      members: existing?.members ?? new SvelteMap(),
      displays: p.displays,
    });

    // A notebook channel going from no-read to read means the book is now in this view's hands.
    // Derived rather than delivered; fires once per gain, not on refreshes while it is held.
    if (read && !had_read && ctx.view.is_notebook_channel(channel_id)) {
      ctx.view.push_notif(ctx.timestamp, { NotebookReceived: {} });
    }
  },

  // A view with no channel entry yet is skipped: the entry is created by UpdateChannelView, which
  // the engine emits first.
  ShowChannelMember(ctx: CmdCtx, p) {
    const entry = ctx.view.channel_views.get(slotKeyToString(p.channel_id));
    if (!entry) return;
    const key = displayKey(p.display);
    // Sticky, and about the OTHER member rather than this view: a member seeded with no perms is
    // not an effective one, and should not be listed as though they were.
    const had_positive =
      (entry.members.get(key)?.had_positive ?? false) || hasPositivePerms(p.channel_perms);
    entry.members.set(key, { display: p.display, perms: p.channel_perms, had_positive });
  },

  RemoveChannelMember(ctx: CmdCtx, p) {
    ctx.view.channel_views
      .get(slotKeyToString(p.channel_id))
      ?.members.delete(displayKey(p.display));
  },

  GcOwnerStatus(ctx: CmdCtx, p) {
    const gc_key = slotKeyToString(p.gc_id);
    if (p.owner) ctx.view.owned_gcs.add(gc_key);
    else ctx.view.owned_gcs.delete(gc_key);
  },
};
