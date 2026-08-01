<script lang="ts">
  // One person row in the Players panel: click to expand a dropdown. Admins get the admin
  // controls menu (inspect + set role / true name / kill / revive); everyone else gets the
  // Contact abilities against this player.
  import { execErrorText, permsLabel, statusLabels } from "../../game/helpers.svelte";
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { ActionRequest } from "../../bindings";
  import { slotKeyFromString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";
  import PlayerAdminControls from "./PlayerAdminControls.svelte";

  interface Props {
    id: string;
    label: string;
    // read/send hint for channel members; omit (null) for non-members.
    perms?: number | null;
  }
  let { id, label, perms = null }: Props = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);

  const session = getContext<SessionState>(SESSION_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  // The one view this component renders. Everything it can show is what this view was delivered.
  const view = $derived(game.view_of(ui.viewer));

  let expanded = $state(false);
  const flash = new Flash();

  const is_admin = $derived(ui.viewer === "Admin");

  // The public condition of this player, as the world-data viewport last projected it.
  const statuses = $derived(statusLabels(view.actor_statuses.get(id) ?? 0));

  const contact_abilities = $derived(
    [...(view.abilities.entries() ?? [])].filter(
      ([, av]) => av.name === "Contact",
    ),
  );

  async function contact(ability_id: string) {
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload: {
        UseAbility: {
          ability_id: slotKeyFromString(ability_id),
          ability_args: { Contact: { target_id: slotKeyFromString(id) } },
        },
      },
    };
    const reply = await session.submit_action(request);
    if (!reply.ok) {
      flash.set_error(`Contact failed: ${execErrorText(reply.error)}`);
    } else {
      flash.set_success("Contact sent.");
      expanded = false;
    }
  }
</script>

<div class="rounded text-sm">
  <button
    class="flex w-full items-center justify-between rounded px-2 py-1 text-neutral-300 hover:bg-neutral-800"
    onclick={() => {
      expanded = !expanded;
      flash.error = null;
      flash.success = null;
    }}
  >
    <span class="flex items-center gap-1.5">
      {label}
      {#each statuses as s (s)}
        <span
          class="rounded bg-neutral-800 px-1 py-px text-[0.6rem] uppercase tracking-wide text-neutral-400"
        >
          {s}
        </span>
      {/each}
    </span>
    {#if perms !== null && permsLabel(perms)}
      <span class="text-xs text-neutral-600">{permsLabel(perms)}</span>
    {/if}
  </button>

  {#if expanded}
    {#if is_admin}
      <PlayerAdminControls {id} />
    {:else}
      <div class="ml-2 mt-0.5 flex flex-col gap-0.5">
        {#each contact_abilities as [ability_id, av] (ability_id)}
          <button
            class="rounded px-2 py-0.5 text-left text-xs text-neutral-400 hover:bg-neutral-800"
            onclick={() => contact(ability_id)}
          >
            Contact ({av.success_usages_remaining}, resets in {av.iterations_to_reset})
          </button>
        {/each}
        <FlashDisplay {flash} />
      </div>
    {/if}
  {/if}
</div>
