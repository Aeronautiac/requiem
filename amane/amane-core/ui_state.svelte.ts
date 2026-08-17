// News is deliberately NOT a channel: it exists on its own and carries no channel data.
export type Selection = { kind: "news" } | { kind: "channel"; id: string };

// The horizontal widget strip that drops under the channel header. Only one is up at a time, and
// the same buttons that open it (in the right rail) close it — so this is a single mode, not a
// per-widget open flag.
export type TopPanel = "polls" | "prosecutions";

export class UiState {
  viewer = $state<string>("Admin");
  selected = $state<Selection | null>(null);
  top_panel = $state<TopPanel | null>(null);
  // Set by a poll-panel "jump" to ask the open channel to scroll that poll's inline card into
  // view; the channel view consumes and clears it. Outlives the channel switch it rides in on.
  jump_poll = $state<string | null>(null);
  // Whether desktop/toast popups are raised. Off mutes the host popups but the in-app
  // Notifications channel is untouched. Session-level, so it survives a view switch.
  notifications_enabled = $state(true);

  get is_news(): boolean {
    return this.selected?.kind === "news";
  }

  // Null when nothing or news is selected — news's backing channel, if any, is resolved
  // separately via news_channel_id.
  get selected_channel(): string | null {
    return this.selected?.kind === "channel" ? this.selected.id : null;
  }

  select_news() {
    this.selected = { kind: "news" };
  }

  select_channel(id: string) {
    this.selected = { kind: "channel", id };
  }

  toggle_panel(panel: TopPanel) {
    this.top_panel = this.top_panel === panel ? null : panel;
  }
}

export const UI_STATE_KEY = Symbol("ui_state");
