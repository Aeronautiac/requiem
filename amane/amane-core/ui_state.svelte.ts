// News is deliberately NOT a channel: it exists on its own and carries no channel data.
export type Selection = { kind: "news" } | { kind: "channel"; id: string };

export class UiState {
  viewer = $state<string>("Admin");
  selected = $state<Selection | null>(null);

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
}

export const UI_STATE_KEY = Symbol("ui_state");
