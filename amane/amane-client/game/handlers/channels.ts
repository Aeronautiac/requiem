// Channels: their registration, their content, and this actor's standing in them.
import { slotKeyToString } from "../../bindings.ts";
import { orgDisplayName, t } from "../../text.ts";
import { ownPerms } from "../perms.ts";
import type { ChannelCategory, ChannelLink } from "../types.ts";
import type { CmdCtx, Handlers } from "./index.ts";

function mapChannel(
  ctx: CmdCtx,
  key: string,
  category: ChannelCategory,
  name: string,
  link: ChannelLink,
) {
  ctx.view.channels.set(key, {
    kind: "Standard",
    category,
    name,
    archived: false,
    // Stated by the SetChannelLoggable the engine emits right after every MapChannel.
    loggable: false,
    viewport: ctx.viewport ?? null,
    link,
    events: [],
  });
}

export const channelHandlers: Handlers = {
  // Every engine channel arrives here; `kind` is what it belongs to, and only how it is filed,
  // named and linked varies. Unnamed objects read as "lounge-3", "trial-1v0".
  MapChannel(ctx, p) {
    const key = slotKeyToString(p.channel_id);
    const kind = p.kind;

    // Private to its owner; visibility falls out of perms, since only the owner is a member.
    if (kind === "Personal") {
      mapChannel(ctx, key, "Personal", `personal-${p.channel_id.idx}v${p.channel_id.version}`, null);
      return;
    }

    if ("World" in kind) {
      const category: ChannelCategory = kind.World === "LAndWatari" ? "Role" : "World";
      if (kind.World === "News") ctx.view.news_channel = key;
      mapChannel(ctx, key, category, kind.World, null);
      return;
    }

    // Named by contact id, which is the number a tap-in guesses at.
    if ("Lounge" in kind) {
      const lounge = slotKeyToString(kind.Lounge.lounge_id);
      mapChannel(ctx, key, "Lounge", `lounge-${kind.Lounge.contact_id}`, { lounge });
      return;
    }

    if ("Groupchat" in kind) {
      const gc = slotKeyToString(kind.Groupchat.gc_id);
      mapChannel(ctx, key, "Groupchat", `groupchat-${kind.Groupchat.contact_id}`, { gc });
      return;
    }

    if ("Notebook" in kind) {
      const notebook = slotKeyToString(kind.Notebook);
      mapChannel(
        ctx,
        key,
        "Notebook",
        `Death Notebook-${kind.Notebook.idx}v${kind.Notebook.version}`,
        { notebook },
      );
      ctx.view.notebooks.set(notebook, { channel: key, borrowed: false, fake: undefined });
      return;
    }

    // The org was registered by the MapActor just before this. This is where its viewport becomes
    // known: its abilities and roster arrive through it afterwards.
    if ("Org" in kind) {
      const org_key = slotKeyToString(kind.Org);
      const org = ctx.view.orgs.get(org_key);
      mapChannel(
        ctx,
        key,
        "Org",
        org ? orgDisplayName(org.name) : t("display_org_unknown"),
        { org: org_key },
      );
      if (org) {
        org.channel = key;
        org.viewport = ctx.viewport ?? null;
      }
      return;
    }

    if ("Kidnapping" in kind) {
      const kidnapping = slotKeyToString(kind.Kidnapping);
      mapChannel(
        ctx,
        key,
        "Kidnapping",
        `kidnapping-${kind.Kidnapping.idx}v${kind.Kidnapping.version}`,
        { kidnapping },
      );
      return;
    }

    // A defendant's private line to their lawyer, or the trial's own public channel. Both are
    // registered here rather than from the UpdateProsecution that names them: that rides
    // presence, and the channel's content rides the channel's own viewport.
    const prosecution_id = "Lawyer" in kind ? kind.Lawyer : kind.Trial;
    const label = "Lawyer" in kind ? "lawyer" : "trial";
    mapChannel(
      ctx,
      key,
      "Prosecution",
      `${label}-${prosecution_id.idx}v${prosecution_id.version}`,
      { prosecution: slotKeyToString(prosecution_id) },
    );
  },

  // Tearing a channel down is always archival — nothing said in it can be un-said. For a notebook
  // this is its destruction: the channel is the book's only home.
  ArchiveChannel(ctx, p) {
    const channel = ctx.view.channels.get(slotKeyToString(p.channel_id));
    if (channel) channel.archived = true;
  },

  SetChannelLoggable(ctx, p) {
    const channel = ctx.view.channels.get(slotKeyToString(p.channel_id));
    if (channel) channel.loggable = p.loggable;
  },

  AddMessage(ctx, p) {
    ctx.view.channels.get(slotKeyToString(p.channel_id))?.events.push({
      timestamp: ctx.timestamp,
      data: { Message: { content: p.content, sender_display: p.sender_display } },
    });
  },

  // Unlike a message this carries no display: the user is named raw, which is the whole cost of
  // the ability.
  KiraConnectionAttempt(ctx, p) {
    ctx.view.channels.get(slotKeyToString(p.channel_id))?.events.push({
      timestamp: ctx.timestamp,
      data: { KiraConnectionAttempt: { user: slotKeyToString(p.user), success: p.success } },
    });
  },

  // Carries no reader: members learn they were tapped, never by whom.
  ChannelTapped(ctx, p) {
    ctx.view.channels.get(slotKeyToString(p.channel_id))?.events.push({
      timestamp: ctx.timestamp,
      data: { ChannelTapped: {} },
    });
  },

  // The names in a channel that are THIS view's to speak as. Holding one is what membership is,
  // so this creates the entry, and an empty set is a member who holds nothing.
  ProfileAccess(ctx, p) {
    const channel_id = slotKeyToString(p.channel_id);
    let standing = ctx.view.channel_views.get(channel_id);
    if (!standing) {
      standing = { roster: [], own: [], owners: [] };
      ctx.view.channel_views.set(channel_id, standing);
    }
    const could_read = ownPerms(standing.own).read;
    standing.own = p.profiles;

    // A notebook channel going from no-read to read means the book is now in this view's hands.
    // Derived rather than delivered; once per gain, not on refreshes while it is held.
    const link = ctx.view.channels.get(channel_id)?.link;
    if (link && "notebook" in link && ownPerms(p.profiles).read && !could_read) {
      ctx.view.push_notif(ctx.timestamp, { NotebookReceived: {} });
    }
  },

  // Every name the room can see, whole, every time. Replaced rather than merged.
  ChannelRoster(ctx, p) {
    const channel_id = slotKeyToString(p.channel_id);
    const standing = ctx.view.channel_views.get(channel_id);
    if (standing) standing.roster = p.profiles;
    else ctx.view.channel_views.set(channel_id, { roster: p.profiles, own: [], owners: [] });
  },

  // Who is behind each name in the roster. Addressed to Admin only, so it only ever lands in the
  // System view.
  ProfileOwnership(ctx, p) {
    const channel_id = slotKeyToString(p.channel_id);
    const standing = ctx.view.channel_views.get(channel_id);
    if (standing) standing.owners = p.owners;
    else ctx.view.channel_views.set(channel_id, { roster: [], own: [], owners: p.owners });
  },

  GcOwnerStatus(ctx, p) {
    const gc_key = slotKeyToString(p.gc_id);
    if (p.owner) ctx.view.owned_gcs.add(gc_key);
    else ctx.view.owned_gcs.delete(gc_key);
  },
};
