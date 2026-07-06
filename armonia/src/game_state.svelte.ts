import { SvelteMap } from "svelte/reactivity";
import type { AbilityName, ActorDisplay, ActorKey, ActionResponse, AppExecution, CommandPayload, CommandRecipient } from "./bindings";
import { slotKeyToString } from "./bindings";

export type ChannelKind = "Lounge" | "Groupchat" | "Notebook" | "News" | "Courtroom" | "Prison" | "Collaboration" | "Raw";

export interface Channel {
  kind: ChannelKind;
  name: string;
  read: boolean;
  send: boolean;
  archived: boolean;
}

export type Message = {
  timestamp: number,
  sender_display: ActorDisplay,
  content: string,
}

export interface AbilityView {
  name: AbilityName;
  usages_remaining: number;
  iterations_to_reset: number;
}

// What a specific player can see.
export class GameView {
  channels = new SvelteMap<string, Channel>();
  // ability_id string → ability view
  abilities = new SvelteMap<string, AbilityView>();
}

export interface Player {
  display_name: string;
}

export function new_player(display_name: string): Player {
  let player: Player = $state({ display_name });
  return player;
}

function recipientToView(rec: CommandRecipient) {
  if (typeof (rec) === 'string') {
    if (rec == "System")
      return "Admin";
    return rec;
  } else {
    const id = rec.Player;
    return slotKeyToString(id);
  }
}

export class GameState {
  players = new SvelteMap<string, Player>();
  views = new SvelteMap<string, GameView>();
  channel_messages = new SvelteMap<string, Message[]>();
  #pending = new Map<string, Channel>();

  constructor() {
    this.views.set("Admin", new GameView());
    this.views.set("Base", new GameView());
  }

  private new_view(key: string) {
    this.views.set(key, new GameView());
  }

  process_response(response: AppExecution, args?: Record<string, unknown>): string | void {
    const { exec_result } = response;
    if (exec_result === "Crashed") {
      return "The engine has crashed.";
    }
    const result = exec_result.Standard;
    if ("Err" in result) {
      return String(result.Err);
    }
    const [action_response, context] = result.Ok;
    this.handle_response(action_response, args);
    for (const cmd of context.commands) {
      this.handle_command(cmd);
    }
  }

  private pending_channel(channel_id: string, kind: ChannelKind, name: string) {
    let ch = this.#pending.get(channel_id);
    if (!ch) {
      ch = { kind, name, read: false, send: false, archived: false };
      this.#pending.set(channel_id, ch);
    } else {
      ch.kind = kind;
      ch.name = name;
    }
    // Retroactively fix any views that already stored this channel as Raw.
    for (const view of this.views.values()) {
      const existing = view.channels.get(channel_id);
      if (existing) {
        existing.kind = kind;
        existing.name = name;
      }
    }
  }

  private handle_command({ recipient, cmd, timestamp }: CommandPayload) {
    if ("MapLounge" in cmd) {
      const { lounge_id, channel_id } = cmd.MapLounge;
      const name = `lounge-${lounge_id.idx}v${lounge_id.version}`;
      this.pending_channel(slotKeyToString(channel_id), "Lounge", name);
      return;
    }

    if ("MapGc" in cmd) {
      this.pending_channel(slotKeyToString(cmd.MapGc.channel_id), "Groupchat", "groupchat");
      return;
    }

    if ("MapNotebook" in cmd) {
      const { notebook_id, channel_id } = cmd.MapNotebook;
      const channel_key = slotKeyToString(channel_id);
      this.pending_channel(channel_key, "Notebook", `death-notebook-${notebook_id.idx}v${notebook_id.version}`);
      return;
    }

    if ("MapWorldChannel" in cmd) {
      const { channel_name, channel_id } = cmd.MapWorldChannel;
      const channel_key = slotKeyToString(channel_id);

      if (channel_name == "News") {
        this.pending_channel(channel_key, "Raw", "News");
      } else if (channel_name == "General") {
        this.pending_channel(channel_key, "Raw", "General");
      } else if (channel_name == "Prison") {
        this.pending_channel(channel_key, "Raw", "Prison");
      } else if (channel_name == "LAndWatari") {
        this.pending_channel(channel_key, "Raw", "L & Watari");
      }

      return;
    }

    if ("UpdateChannelView" in cmd) {
      const channel_id = slotKeyToString(cmd.UpdateChannelView.channel_id);
      const pending = this.#pending.get(channel_id);

      const view = this.views.get(recipientToView(recipient))!;
      const read = (cmd.UpdateChannelView.perms & 2) !== 0;
      const send = (cmd.UpdateChannelView.perms & 1) !== 0;
      let channel = view.channels.get(channel_id);
      if (channel) {
        channel.read = read;
        channel.send = send;
      } else {
        let ch: Channel = $state({
          kind: pending?.kind ?? "Raw",
          name: pending?.name ?? "raw",
          read,
          send,
          archived: false,
        });
        view.channels.set(channel_id, ch);
      }

      return;
    }

    if ("ArchiveChannel" in cmd) {
      const channel_id = slotKeyToString(cmd.ArchiveChannel.channel_id);
      for (const view of this.views.values()) {
        const ch = view.channels.get(channel_id);
        if (ch) ch.archived = true;
      }
      return;
    }

    if ("RemoveChannel" in cmd) {
      this.views.get(recipientToView(recipient))
        ?.channels.delete(slotKeyToString(cmd.RemoveChannel.channel_id));
      return;
    }

    if ("AddMessage" in cmd) {
      const { channel_id, content, sender_display } = cmd.AddMessage;
      const key = slotKeyToString(channel_id);
      const arr = this.channel_messages.get(key) ?? [];
      this.channel_messages.set(key, [...arr, { sender_display, content, timestamp }]);
      return;
    }

    if ("NotebookWrite" in cmd) {
      // const { notebook_id, user_id, message, true_name, delay, successes_remaining, attempts_remaining, success, target_saved } = cmd.NotebookWrite;
      // const channel_key = this.#notebook_to_channel.get(slotKeyToString(notebook_id));
      // if (channel_key) {
      //   const arr = this.events.get(channel_key) ?? [];
      //   this.events.set(channel_key, [...arr, { type: "notebook_write", user_id, true_name, success, target_saved, delay, message, successes_remaining, attempts_remaining, timestamp }]);
      // }
      // return;
    }

    if ("UpdateAbilityView" in cmd) {
      if (typeof (recipient) !== "string") {
        const player = recipient.Player;
        const { ability_id, ability_name, usages_remaining, iterations_to_reset } = cmd.UpdateAbilityView;
        const view = this.views.get(slotKeyToString(player));
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
      }
      return;
    }

    if ("RemoveAbility" in cmd) {
      this.views.get(recipientToView(recipient))
        ?.abilities.delete(slotKeyToString(cmd.RemoveAbility.ability_id));
      return;
    }
  }

  private handle_response(response: ActionResponse, args?: Record<string, unknown>) {
    if ("AddPlayer" in response) {
      this.add_player(response.AddPlayer.id, args?.display_name as string);
      return;
    }
  }

  admin_view(): GameView {
    return this.views.get("Admin")!;
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
}

export const GAME_STATE_KEY = Symbol("game_state");
