<script lang="ts">
  // Admin-only controls for a single player, surfaced inside the player dropdown: inspect
  // the player's known facts and act on them (set role, change true name, kill, revive).
  // Every action dispatches as the Admin actor, which the engine accepts for these.
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState } from "../../game_state.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { Action, ActionRequest, Role } from "../../bindings";
  import { slotKeyFromString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";

  interface Props {
    id: string;
  }
  let { id }: Props = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  const flash = new Flash();

  // Hand-maintained to mirror the Role union in bindings (no runtime enum to derive from).
  const ROLES: Role[] = [
    "Kira", "SecondKira", "L", "Watari", "BeyondBirthday", "PrivateInvestigator",
    "NewsAnchor", "Civilian", "RogueCivilian", "Poser", "ConArtist", "WantedCivilian",
    "Near", "Mello",
  ];

  const info = $derived(game.player_info.get(id));
  const target = $derived(slotKeyFromString(id));

  let role = $state<Role>(info?.role ?? "Civilian");
  let true_name = $state("");

  async function run(payload: Action, ok: string) {
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload,
    };
    const err = await game.dispatch(request);
    if (err) flash.set_error(err);
    else flash.set_success(ok);
  }

  function set_role() {
    run({ GiveRole: { target_id: target, role } }, `Role set to ${role}.`);
  }
  function set_true_name() {
    const name = true_name.trim();
    if (!name) {
      flash.set_error("Enter a name.");
      return;
    }
    run({ SetTrueName: { target_id: target, true_name: name } }, "True name set.");
  }
  function kill() {
    run(
      {
        Kill: {
          target_id: target,
          killer_id: null,
          death_message: null,
          silent: false,
          allow_link_chaining: true,
          sever_links: true,
          set_books_dormant: false,
        },
      },
      "Player killed.",
    );
  }
  function revive() {
    run({ Revive: { target_id: target, ignore_links: false } }, "Player revived.");
  }
</script>

<div class="flex flex-col gap-1.5 px-2 py-1 text-xs">
  <div class="text-neutral-500">
    <div><span class="text-neutral-600">Role:</span> {info?.role ?? "—"}</div>
    <div><span class="text-neutral-600">True name:</span> {info?.true_name ?? "—"}</div>
  </div>

  <div class="flex items-center gap-1">
    <select
      bind:value={role}
      class="min-w-0 flex-1 rounded bg-neutral-800 px-1 py-0.5 text-neutral-200"
    >
      {#each ROLES as r (r)}
        <option value={r}>{r}</option>
      {/each}
    </select>
    <button
      class="shrink-0 rounded bg-neutral-700 px-2 py-0.5 text-neutral-200 hover:bg-neutral-600"
      onclick={set_role}
    >
      Set role
    </button>
  </div>

  <div class="flex items-center gap-1">
    <input
      bind:value={true_name}
      placeholder="New true name"
      class="min-w-0 flex-1 rounded bg-neutral-800 px-1 py-0.5 text-neutral-200"
    />
    <button
      class="shrink-0 rounded bg-neutral-700 px-2 py-0.5 text-neutral-200 hover:bg-neutral-600"
      onclick={set_true_name}
    >
      Set name
    </button>
  </div>

  <div class="flex items-center gap-1">
    <button
      class="flex-1 rounded bg-red-900/60 px-2 py-0.5 text-red-200 hover:bg-red-900"
      onclick={kill}
    >
      Kill
    </button>
    <button
      class="flex-1 rounded bg-emerald-900/60 px-2 py-0.5 text-emerald-200 hover:bg-emerald-900"
      onclick={revive}
    >
      Revive
    </button>
  </div>

  <FlashDisplay {flash} />
</div>
