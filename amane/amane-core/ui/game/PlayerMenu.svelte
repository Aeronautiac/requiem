<script lang="ts">
  // The one profile menu, opened from any name via the shared controller (see player_menu.svelte.ts).
  // Everything it offers acts as the current viewer against the clicked player: contact, press-
  // conference management (only with the NewsControl passive — the engine enforces it too), and the
  // full admin controls when viewing as Admin. Mounted once by GameScreen.
  import { getContext } from "svelte";
  import Dialog from "../kit/Dialog.svelte";
  import Name from "./Name.svelte";
  import PlayerAdminControls from "./PlayerAdminControls.svelte";
  import FlashDisplay from "../Flash.svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { Action, ActionRequest } from "../../bindings";
  import { slotKeyFromString } from "../../bindings";
  import { execErrorText, statusBadgeStyle, statusLabels } from "../../game/helpers.svelte";
  import { viewerToActor } from "../../types";
  import { now } from "../../time.svelte.ts";
  import { Flash } from "../../flash.svelte.ts";
  import { getPlayerMenu } from "./player_menu.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  const session = getContext<SessionState>(SESSION_KEY);
  const menu = getPlayerMenu();
  const view = $derived(game.view_of(ui.viewer));
  const flash = new Flash();

  // Bridge the controller's target to the Dialog's own open state, and clear the target when the
  // dialog dismisses itself (Esc / backdrop).
  let open = $state(false);
  $effect(() => {
    open = menu?.target != null;
  });
  // Reset feedback whenever a different name is opened.
  $effect(() => {
    menu?.target;
    flash.error = null;
    flash.success = null;
  });

  const id = $derived(menu?.target ?? null);
  const statuses = $derived(id ? statusLabels(view.actor_statuses.get(id) ?? 0) : []);
  const is_admin = $derived(ui.viewer === "Admin");

  const contact_abilities = $derived(
    [...view.abilities.entries()].filter(([, av]) => av.name === "Contact"),
  );
  // The engine gates press-conference management on the NewsControl passive; mirror that so the
  // action only shows to someone who could actually use it.
  const can_manage_conf = $derived(
    [...view.passives.values()].some((p) => p.type === "NewsControl"),
  );
  const in_conf = $derived(id != null && view.press_conf.has(id));

  const has_actions = $derived(
    contact_abilities.length > 0 || can_manage_conf || is_admin,
  );

  async function run(payload: Action, ok: string) {
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload,
    };
    const reply = await session.submit_action(request);
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else flash.set_success(ok);
  }

  function contact() {
    const entry = contact_abilities[0];
    if (!entry || id == null) return;
    run(
      {
        UseAbility: {
          ability_id: slotKeyFromString(entry[0]),
          ability_args: { Contact: { target_id: slotKeyFromString(id) } },
        },
      },
      "Contact sent.",
    );
  }

  function toggle_conf() {
    if (id == null) return;
    run(
      { PressConfAccess: { target_id: slotKeyFromString(id), has_access: !in_conf } },
      in_conf ? "Removed from press conference." : "Added to press conference.",
    );
  }
</script>

{#snippet header()}
  <span class="flex flex-wrap items-center gap-1.5">
    {#if id}<Name {id} {view} chip menu={false} />{/if}
    {#each statuses as s (s)}
      <span
        class="rounded px-1 py-px text-[0.6rem] uppercase tracking-wide"
        style={statusBadgeStyle(s)}
      >
        {s}
      </span>
    {/each}
  </span>
{/snippet}

<Dialog
  bind:open
  onOpenChange={(o) => !o && menu?.close()}
  class="max-w-sm"
  {header}
>
  {#if id}
    <div class="flex flex-col gap-3 text-sm">
      {#if contact_abilities.length > 0 || can_manage_conf}
        <div class="flex flex-col gap-2">
          {#if contact_abilities.length > 0}
            <button
              class="rounded bg-neutral-700 px-3 py-2 text-left text-neutral-200 hover:bg-neutral-600"
              onclick={contact}
            >
              Contact
            </button>
          {/if}
          {#if can_manage_conf}
            <button
              class="rounded bg-neutral-700 px-3 py-2 text-left text-neutral-200 hover:bg-neutral-600"
              onclick={toggle_conf}
            >
              {in_conf ? "Remove from press conference" : "Add to press conference"}
            </button>
          {/if}
        </div>
      {/if}

      {#if is_admin}
        <div class="border-t border-edge pt-1">
          <PlayerAdminControls {id} />
        </div>
      {/if}

      {#if !has_actions}
        <p class="py-1 text-ink-dim">No actions available.</p>
      {/if}

      <FlashDisplay {flash} />
    </div>
  {/if}
</Dialog>
