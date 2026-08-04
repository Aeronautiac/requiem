import { getContext } from "svelte";

// A single profile menu, opened from anywhere a name renders. Name instances number in the hundreds
// (every message, announcement, roster row), so they must not each carry a dialog — they call this
// shared controller with the clicked id, and one PlayerMenu mounted at the top of the tree reads the
// target and renders. The menu resolves the id against the current viewer's view itself, so the
// controller carries only the target.
export const PLAYER_MENU_KEY = Symbol("player_menu");

export class PlayerMenuController {
  target = $state<string | null>(null);

  open(id: string) {
    this.target = id;
  }
  close() {
    this.target = null;
  }
}

// Undefined outside a GameScreen (e.g. a name rendered in an admin-only surface); Name treats that
// as "not clickable" rather than erroring.
export function getPlayerMenu(): PlayerMenuController | undefined {
  return getContext(PLAYER_MENU_KEY);
}
