export class UiState {
  viewer = $state<string>("Admin");
  selected_channel = $state<string | null>(null);
}

export const UI_STATE_KEY = Symbol("ui_state");
