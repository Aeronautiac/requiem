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
