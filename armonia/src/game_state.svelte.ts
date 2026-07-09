import { SvelteMap } from "svelte/reactivity";
import type { AbilityName, ActorDisplay, ActorKey, ActionResponse, AppExecution, CommandPayload, CommandRecipient, NotebookKey, Role } from "./bindings";
import { slotKeyFromString, slotKeyToString } from "./bindings";

// store all messages and events in the top level, but give every view a copy 

export type WorldEvent = {
  Death: {
    target_id: string,
    true_name: string,
    death_message: string,
    role: Role,
    notebook_transferred: boolean,
    ability_transferred: boolean,
  }
} |
{
  PseudocideRevival: {
    target_id: string,
  }
} | {
  Kidnapping: {
    target_id: string,
    duration: number,
  }
} | {
  KidnapReveal: {
    kidnapper: string | null, // null = the kidnapper stayed anonymous
  }
} | {
  AnonymousAnnouncement: {
    content: string,
  }
}

export type WriteEvent = {
  user_id: string,
  notebook_id: string,
  message: string,
  true_name: string,
  delay: number,
  successes_remaining: number,
  attempts_remaining: number,
  success: boolean,
  target_saved: boolean,
}


export type ChannelPerms = {
  read: boolean;
  read_updated: number; // the time that read perms were last updated
  send: boolean;
  loggability_control: boolean;
  had_positive: boolean; // if any perm here has been positive, this is set permanently
}

// world is a generic world channel type for things like prison
// a role channel may be a world channel, but it is stronger than the world category
// these categories are used only for display and hold no significance outside of it
// key world channels should get their own data at a higher level and point to the lower level channel
export type ChannelKind = "Raw" | "Lounge" | "Groupchat" | "Notebook" | "Role" | "World";
export const CHANNEL_KINDS: ChannelKind[] = [
  "Raw",
  "Lounge",
  "Groupchat",
  "Notebook",
  "Role",
  "World",
];


export type Channel = {
  kind: ChannelKind;
  name: string;
  archived: boolean;
  events: GameEvent[];
}

export type Message = {
  sender_display: ActorDisplay,
  content: string,
}

export type GameEvent = {
  timestamp: number,
  data: { Message: Message } | { Write: WriteEvent } | WorldEvent,
}

export interface AbilityView {
  name: AbilityName;
  usages_remaining: number;
  iterations_to_reset: number;
}

// world events are stored individually within each view
// channels and similar are stored top level
// when rendering, you must create a list of channel events (world events + viewable messages)
// and render those rather than rendering directly
//
// anything that would be stored in a database should be stored top level
// as a rule of thumb
//
// we can do away with the concept of an admin game view state with this in mind
// admin can be treated as a viewer of everything and a valid viewing state, but it doesn't need its own local state.
// that is a player specific thing.
//
// channel permissions can be stored within the channel object itself per actor
//
// its ok to transform data during a rendering pass. this is already bloated to hell (its web dev) and its not core logic.
// its an interaction layer.
// cases where this would need to be done are cases like rendering channel events where things like messages, world events, etc... should be combined into one list and then rendered.
//
// only channels which players have had view permissions for at some point should be rendered

// what a specific player can see
// A single member of a channel, as seen by one viewer. Membership is per-view
// because the same actor can be shown under different displays to different viewers
// (deception). Keyed by display key -> member.
export type ChannelMemberView = {
  display: ActorDisplay;
  perms: number;
  // Sticky: set once this member has ever held a positive permission. A member that
  // never had one isn't an effective member and shouldn't be shown as one.
  had_positive: boolean;
};

// Everything a viewer knows about a channel they're a member of: their permissions
// and the members they can see. The presence of an entry (hence perms) IS the
// membership signal — non-members hold no entry and receive no member updates.
export type ChannelView = {
  perms: ChannelPerms;
  members: SvelteMap<string, ChannelMemberView>;
  // The displays this viewer may send as in this channel (their "send as" options).
  displays: ActorDisplay[];
};

export class GameView {
  // channel key -> the viewer's perms + visible members. Only channels the viewer is
  // (or was) a member of appear here.
  channel_views = new SvelteMap<string, ChannelView>();
  events: GameEvent[] = $state([]); // should only store world events
  abilities = new SvelteMap<string, AbilityView>();

  // structuredClone can't clone a GameView: it drops the class prototype and
  // turns the SvelteMaps into plain Maps (losing reactivity), and can throw on
  // the maps' internal reactive state. Clone by hand into fresh SvelteMaps,
  // copying each value object so views don't share mutable state.
  clone(): GameView {
    const copy = new GameView();
    for (const [id, cv] of this.channel_views) {
      const members = new SvelteMap<string, ChannelMemberView>();
      for (const [dkey, m] of cv.members) {
        members.set(dkey, {
          display: m.display,
          perms: m.perms,
          had_positive: m.had_positive,
        });
      }
      copy.channel_views.set(id, {
        perms: { ...cv.perms },
        members,
        displays: [...cv.displays],
      });
    }
    for (const [id, ability] of this.abilities) {
      copy.abilities.set(id, $state.snapshot(ability));
    }
    for (const event of this.events) {
      copy.events.push(event);
    }
    return copy;
  }
}

// Stable map key for an ActorDisplay (the tagged union isn't usable as a key directly).
export function displayKey(d: ActorDisplay): string {
  if (typeof d === "string") return d; // "Mysterious" | "System"
  if ("Raw" in d) return `Raw:${slotKeyToString(d.Raw)}`;
  if ("Org" in d) return `Org:${slotKeyToString(d.Org)}`;
  return `Role:${d.Role}`;
}

// Channel permission bits, mirroring ChannelPermission in the engine.
export const PERM_SEND = 1;
export const PERM_VIEW = 2;
export const PERM_LOGGABILITY = 4;

// Whether a perms value grants at least one permission. Used to derive the sticky
// had_positive flag — a membership is "effective" if it has EVER been positive.
export function hasPositivePerms(perms: number): boolean {
  return (perms & (PERM_SEND | PERM_VIEW | PERM_LOGGABILITY)) !== 0;
}

export interface Player {
  display_name: string;
}

export function new_player(display_name: string): Player {
  let player: Player = $state({ display_name });
  return player;
}

// Channels must be $state proxies, not plain objects: SvelteMap only tracks its
// own get/set, not deep mutations to stored values. Without this, channel.events.push
// and channel.archived = true don't trigger reactivity, so views go stale until
// something else (e.g. switching channels) forces a recompute.
export function new_channel(kind: ChannelKind, name: string): Channel {
  let channel: Channel = $state({ kind, name, archived: false, events: [] });
  return channel;
}

// Maps a command recipient to the key of the view it targets. Player recipients
// key by their slot; the string variants map to the standing views: System is the
// admin view (world events emitted with include_system land here), BasePlayer is
// the Base template. Returns undefined only for an unhandled recipient.
function recipientToView(rec: CommandRecipient): string | undefined {
  if (typeof rec !== "string") {
    return slotKeyToString(rec.Player);
  }
  if (rec === "System") return "System";
  if (rec === "BasePlayer") return "Base";
  return undefined;
}

function recipientToPlayer(recipient: CommandRecipient): string | undefined {
  if (typeof (recipient) !== 'string') {
    const id = recipient.Player;
    return slotKeyToString(id);
  }
}

export class GameState {
  #channel_to_notebook = new SvelteMap<string, string>();
  #notebook_to_channel = new SvelteMap<string, string>();
  // The real News channel's key once one exists (set in MapWorldChannel). $state so
  // that views resolving news's backing channel recompute the moment it's assigned —
  // otherwise selecting news before the channel exists never picks up its perms.
  // Left pointing at a stale key after removal so news falls back to event-log-only.
  news_channel_id = $state<string | null>(null);
  channels = new SvelteMap<string, Channel>();
  players = new SvelteMap<string, Player>();
  views = new SvelteMap<string, GameView>();

  constructor() {
    this.views.set("Base", new GameView());
    // System (admin) is not a player: it's never cloned from Base and bypasses
    // channel perms (see is_admin), so its perms/abilities stay empty. It exists
    // to hold the state authority can't cover, like the world-event stream.
    this.views.set("System", new GameView());
  }

  private new_view(key: string) {
    this.views.set(key, this.base_view().clone());
  }

  process_response(response: AppExecution, args?: Record<string, unknown>): string | void {
    const { exec_result } = response;
    if (exec_result === "Crashed") {
      return "The engine has crashed.";
    }
    const result = exec_result.Standard;
    if ("Err" in result) {
      // Even on failure the engine returns catchup commands (job-queue world
      // progression) that must still be applied. Only then surface the error.
      const [error, context] = result.Err;
      for (const cmd of context.commands) {
        this.handle_command(cmd);
      }
      return String(error);
    }
    const [action_response, context] = result.Ok;
    this.handle_response(action_response, args);
    for (const cmd of context.commands) {
      this.handle_command(cmd);
    }
  }

  private handle_command({ recipient, cmd, timestamp }: CommandPayload) {
    if ("MapLounge" in cmd) {
      const { lounge_id, channel_id } = cmd.MapLounge;
      const name = `lounge-${lounge_id.idx}v${lounge_id.version}`;

      this.channels.set(slotKeyToString(channel_id), new_channel("Lounge", name));

      return;
    }

    if ("MapGc" in cmd) {
      return;
    }

    if ("MapNotebook" in cmd) {
      const { notebook_id, channel_id } = cmd.MapNotebook;
      const channel_key = slotKeyToString(channel_id);
      const notebook_key = slotKeyToString(notebook_id);

      this.channels.set(
        slotKeyToString(channel_id),
        new_channel("Notebook", "Death Notebook" + '-' + notebook_id.idx + 'v' + notebook_id.version),
      );
      this.#channel_to_notebook.set(channel_key, notebook_key);
      this.#notebook_to_channel.set(notebook_key, channel_key);

      return;
    }

    if ("MapWorldChannel" in cmd) {
      const { channel_name, channel_id } = cmd.MapWorldChannel;

      let kind: ChannelKind = "World";
      if (channel_name == "LAndWatari") {
        kind = "Role";
      }

      const key = slotKeyToString(channel_id);
      // News is special: it must always appear to exist even after the underlying
      // channel is removed (world events render into it regardless of the channel's
      // existence or the viewer's perms). Remember its id so the UI can find it.
      if (channel_name === "News") {
        this.news_channel_id = key;
      }

      this.channels.set(key, new_channel(kind, channel_name));

      return;
    }

    // this should only ever be targetted toward players. ignore anything else.
    if ("UpdateChannelView" in cmd) {
      const channel_id = slotKeyToString(cmd.UpdateChannelView.channel_id);

      const player_id = recipientToPlayer(recipient);
      if (player_id) {
        const view = this.views.get(player_id)!;
        const p = cmd.UpdateChannelView.perms;
        const loggability_control = (p & PERM_LOGGABILITY) !== 0;
        const read = (p & PERM_VIEW) !== 0;
        const send = (p & PERM_SEND) !== 0;
        const existing = view.channel_views.get(channel_id);
        const old_perms = existing?.perms;
        let read_updated: number = timestamp;
        let had_positive = read || send || loggability_control;
        if (old_perms) {
          if (read === old_perms.read) {
            read_updated = old_perms.read_updated;
          }
          had_positive ||= old_perms.had_positive;
        }
        const perms: ChannelPerms = {
          had_positive,
          read_updated,
          loggability_control,
          read,
          send,
        };
        // Perms is the membership signal: an UpdateChannelView creates the channel
        // entry if absent, preserving members if it already existed. Re-set the map key
        // (rather than mutating in place) so perms/displays updates trigger reactivity.
        view.channel_views.set(channel_id, {
          perms,
          members: existing?.members ?? new SvelteMap(),
          displays: cmd.UpdateChannelView.displays,
        });
      }
      return;
    }

    // can only be directed to system
    if ("ArchiveChannel" in cmd) {
      const channel_id = slotKeyToString(cmd.ArchiveChannel.channel_id);
      const ch = this.channels.get(channel_id);
      if (ch) ch.archived = true;
      return;
    }

    // RemoveChannel is per-player: the target player is no longer a member (e.g. a
    // notebook that transferred away), so drop the channel from THAT player's view
    // only. The channel still exists globally for whoever else holds it — deleting it
    // globally here would wipe it for the new owner who was just granted access.
    if ("RemoveChannel" in cmd) {
      const player_id = recipientToPlayer(recipient);
      if (player_id) {
        // Drops both perms and members for that player — they're no longer a member.
        this.views
          .get(player_id)
          ?.channel_views.delete(slotKeyToString(cmd.RemoveChannel.channel_id));
      }
      return;
    }

    // DeleteChannel removes the channel globally (system-directed); the channel
    // ceases to exist for everyone.
    if ("DeleteChannel" in cmd) {
      this.channels.delete(slotKeyToString(cmd.DeleteChannel.channel_id));
      return;
    }

    // A member (identified by a display) was made visible in a channel. Only members
    // of the channel receive these, so the channel entry already exists (perms first).
    if ("ShowChannelMember" in cmd) {
      const { channel_id, display, channel_perms } = cmd.ShowChannelMember;
      const view = this.views.get(recipientToView(recipient) ?? "");
      const entry = view?.channel_views.get(slotKeyToString(channel_id));
      if (entry) {
        const key = displayKey(display);
        const had_positive =
          (entry.members.get(key)?.had_positive ?? false) ||
          hasPositivePerms(channel_perms);
        entry.members.set(key, { display, perms: channel_perms, had_positive });
      }
      return;
    }

    if ("RemoveChannelMember" in cmd) {
      const { channel_id, display } = cmd.RemoveChannelMember;
      const view = this.views.get(recipientToView(recipient) ?? "");
      view?.channel_views
        .get(slotKeyToString(channel_id))
        ?.members.delete(displayKey(display));
      return;
    }

    if ("AddMessage" in cmd) {
      const { channel_id, content, sender_display } = cmd.AddMessage;
      const key = slotKeyToString(channel_id);
      // its not possible to add a message to a channel that doesnt exist
      const channel = this.channels.get(key)!;
      channel.events.push({
        timestamp,
        data: {
          Message: {
            content,
            sender_display,
          }
        },
      });
      return;
    }

    // writes are treated the exact same as messages, so they should be stored using the same mechanism 
    if ("NotebookWrite" in cmd) {
      const { notebook_id, user_id, message, true_name, delay, successes_remaining, attempts_remaining, success, target_saved } = cmd.NotebookWrite;
      const channel_key = this.#notebook_to_channel.get(slotKeyToString(notebook_id));
      if (channel_key) {
        const channel = this.channels.get(channel_key);
        if (channel) {
          channel.events.push({
            timestamp,
            data: {
              Write: {
                user_id: slotKeyToString(user_id),
                notebook_id: slotKeyToString(notebook_id),
                message: message ?? "",
                target_saved,
                success,
                successes_remaining,
                attempts_remaining,
                delay,
                true_name,
              }
            }
          });
        }
      }
      return;
    }

    if ("UpdateAbilityView" in cmd) {
      const { ability_id, ability_name, usages_remaining, iterations_to_reset } = cmd.UpdateAbilityView;
      const view = this.views.get(recipientToView(recipient) ?? "");
      if (view) {
        const id = slotKeyToString(ability_id);
        const existing = view.abilities.get(id);
        if (existing) {
          existing.usages_remaining = usages_remaining;
          existing.iterations_to_reset = iterations_to_reset;
        } else {
          let av: AbilityView = $state({ name: ability_name, usages_remaining, iterations_to_reset });
          view.abilities.set(id, av);
        }
      }
      return;
    }

    if ("RemoveAbility" in cmd) {
      const view = this.views.get(recipientToView(recipient) ?? "");
      view?.abilities.delete(slotKeyToString(cmd.RemoveAbility.ability_id));
      return;
    }

    if ("Death" in cmd) {
      const death = cmd.Death;
      const view = this.views.get(recipientToView(recipient) ?? "");
      if (view) {
        view.events.push({
          timestamp,
          data: {
            Death: {
              target_id: slotKeyToString(death.target_id),
              true_name: death.true_name,
              death_message: death.death_message,
              role: death.role,
              notebook_transferred: death.notebook_transferred,
              ability_transferred: death.ability_transferred,
            }
          }
        });
      }
    }

    if ("AnonymousAnnouncement" in cmd) {
      const annc = cmd.AnonymousAnnouncement;
      const view = this.views.get(recipientToView(recipient) ?? "");
      if (view) {
        view.events.push({
          timestamp,
          data: {
            AnonymousAnnouncement: {
              content: annc.content
            }
          }
        });
      }
    }

    if ("Kidnapping" in cmd) {
      const kidnapping = cmd.Kidnapping;
      const view = this.views.get(recipientToView(recipient) ?? "");
      if (view) {
        view.events.push({
          timestamp,
          data: {
            Kidnapping: {
              target_id: slotKeyToString(kidnapping.target_id),
              duration: kidnapping.duration,
            }
          }
        });
      }
    }

    if ("KidnapReveal" in cmd) {
      const reveal = cmd.KidnapReveal;
      const view = this.views.get(recipientToView(recipient) ?? "");
      if (view) {
        view.events.push({
          timestamp,
          data: {
            KidnapReveal: {
              kidnapper: reveal.kidnapper
                ? slotKeyToString(reveal.kidnapper)
                : null,
            }
          }
        });
      }
    }

    if ("PseudocideRevival" in cmd) {
      const revival = cmd.PseudocideRevival;
      const view = this.views.get(recipientToView(recipient) ?? "");
      if (view) {
        view.events.push({
          timestamp,
          data: {
            PseudocideRevival: {
              target_id: slotKeyToString(revival.target_id),
            }
          }
        });
      }
    }
  }

  private handle_response(response: ActionResponse, args?: Record<string, unknown>) {
    if ("AddPlayer" in response) {
      this.add_player(response.AddPlayer.id, args?.display_name as string);
      return;
    }
  }

  base_view(): GameView {
    return this.views.get("Base")!;
  }

  system_view(): GameView {
    return this.views.get("System")!;
  }

  add_player(id: ActorKey, display_name: string) {
    const key = slotKeyToString(id);
    this.new_view(key);
    this.players.set(key, new_player(display_name));
  }

  find_abilities(viewer_key: string, name: string): string[] {
    const result: string[] = [];
    for (const [id, av] of this.views.get(viewer_key)?.abilities ?? []) {
      if (av.name === name) result.push(id);
    }
    return result;
  }

  // The notebook backing a notebook channel, if any. Used by the write menu to
  // target the correct notebook.
  notebook_for_channel(channel_key: string): NotebookKey | undefined {
    const notebook_key = this.#channel_to_notebook.get(channel_key);
    return notebook_key ? slotKeyFromString(notebook_key) : undefined;
  }

  // Resolve an ActorDisplay to the name to show. Raw displays look up the player's
  // name; the rest are self-describing or intentionally opaque.
  resolve_display(display: ActorDisplay): string {
    if (display === "Mysterious") return "???";
    if (display === "System") return "System";
    if ("Raw" in display)
      return this.players.get(slotKeyToString(display.Raw))?.display_name ?? "Unknown";
    if ("Role" in display) return display.Role;
    // TODO: orgs aren't modeled on the frontend yet. Once they are, resolve Org to
    // the org's name (display.Org is its actor key) the way Role resolves to its name.
    return "Org";
  }
}

export const GAME_STATE_KEY = Symbol("game_state");
