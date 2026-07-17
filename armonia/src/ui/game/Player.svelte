<script lang="ts">
  // One person row in the Players panel: click to expand a dropdown. Admins get the admin
  // controls menu (inspect + set role / true name / kill / revive); everyone else gets the
  // Contact abilities against this player.
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game_state.svelte.ts";
  import { CLIENT_KEY, type ClientState } from "../../client.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState } from "../../game_state.svelte.ts";
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

  const client = getContext<ClientState>(CLIENT_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let expanded = $state(false);
  const flash = new Flash();

  const is_admin = $derived(ui.viewer === "Admin");

  const contact_abilities = $derived(
    [...(game.views.get(ui.viewer)?.abilities.entries() ?? [])].filter(
      ([, av]) => av.name === "Contact",
    ),
  );

  // send = bit 0, read = bit 1 (matches the UpdateChannelView perms parsing).
  function perms_label(p: number): string {
    const parts: string[] = [];
    if (p & 2) parts.push("read");
    if (p & 1) parts.push("send");
    return parts.join(" · ");
  }

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
    const err = await client.dispatch(request);
    if (err) {
      flash.set_error(`Contact failed: ${err}`);
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
    <span>{label}</span>
    {#if perms !== null && perms_label(perms)}
      <span class="text-xs text-neutral-600">{perms_label(perms)}</span>
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
