<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState } from "../../game_state.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { ActionPayload } from "../../bindings";
  import { slotKeyFromString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";
  import AbilityMenu from "./abilities/AbilityMenu.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let open = $state(true);
  const flash = new Flash();

  // The org backing the selected channel, if it's an org channel.
  const org_key = $derived(
    ui.selected_channel
      ? game.org_key_for_channel(ui.selected_channel)
      : undefined,
  );
  const org = $derived(org_key ? game.orgs.get(org_key) : undefined);

  // Only those who can view the org's channel (or admin) see the panel — matches the
  // sidebar's gating and the org-directed command delivery rule.
  const visible = $derived(
    !!org &&
      (ui.viewer === "Admin" ||
        (game.views.get(ui.viewer)?.channel_views.get(org.channel_id)?.perms
          ?.had_positive ??
          false)),
  );

  // Org members: the full list, everyone sees it. Resolved to display names.
  const org_members = $derived(
    [...(org?.members ?? [])].map((id) => ({
      id,
      name: game.players.get(id)?.display_name ?? "Unknown",
    })),
  );

  // Admin-only: players not already in the org, to add via AddToOrg.
  const candidates = $derived(
    [...game.players.entries()].filter(([id]) => !org?.members.has(id)),
  );

  // Configuration applied to the next add: leader makes them the org leader (requires the
  // org to have leadership); og marks them an original/founding member.
  let add_leader = $state(false);
  let add_og = $state(false);

  async function send(payload: ActionPayload, ok: string) {
    const err = await game.dispatch({
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload,
    });
    if (err) flash.set_error(err);
    else flash.set_success(ok);
  }

  function add_member(player_id: string) {
    if (!org_key) return;
    send(
      {
        AddToOrg: {
          actor_id: slotKeyFromString(player_id),
          org_id: slotKeyFromString(org_key),
          leader: add_leader,
          og: add_og,
        },
      },
      "Added.",
    );
  }

  function remove_member(player_id: string) {
    if (!org_key) return;
    send(
      {
        RemoveFromOrg: {
          actor_id: slotKeyFromString(player_id),
          org_id: slotKeyFromString(org_key),
        },
      },
      "Removed.",
    );
  }
</script>

{#if visible && org && org_key}
  <div class="flex flex-col gap-1 border-b border-neutral-800 p-2">
    <button
      class="flex items-center gap-1 px-2 py-1 text-xs font-medium uppercase tracking-wide text-neutral-400 hover:text-neutral-200"
      onclick={() => (open = !open)}
    >
      <span class="text-[0.6rem]">{open ? "▾" : "▸"}</span>
      Organization
    </button>

    {#if open}
      <div class="px-2 pt-1">
        <AbilityMenu orgId={org_key} />
      </div>

      <p class="px-2 pt-2 text-[0.65rem] uppercase tracking-wide text-neutral-600">
        Members ({org_members.length})
      </p>
      {#if org_members.length === 0}
        <p class="px-2 py-1 text-xs text-neutral-600">No members</p>
      {:else}
        {#each org_members as m (m.id)}
          <div class="flex items-center justify-between gap-1 rounded px-2 py-1 text-sm text-neutral-300">
            <span class="truncate">{m.name}</span>
            {#if ui.viewer === "Admin"}
              <button
                class="shrink-0 rounded px-1.5 py-0.5 text-xs text-red-400/80 hover:bg-neutral-800 hover:text-red-300"
                onclick={() => remove_member(m.id)}
                title="Remove from org"
              >
                remove
              </button>
            {/if}
          </div>
        {/each}
      {/if}

      {#if ui.viewer === "Admin"}
        <p class="px-2 pt-2 text-[0.65rem] uppercase tracking-wide text-neutral-600">
          Add member
        </p>
        <div class="flex gap-3 px-2 py-1 text-xs text-neutral-400">
          <label class="flex items-center gap-1">
            <input type="checkbox" bind:checked={add_leader} /> leader
          </label>
          <label class="flex items-center gap-1">
            <input type="checkbox" bind:checked={add_og} /> og
          </label>
        </div>
        {#if candidates.length === 0}
          <p class="px-2 py-1 text-xs text-neutral-600">No one to add</p>
        {:else}
          {#each candidates as [id, player] (id)}
            <button
              class="flex w-full items-center justify-between rounded px-2 py-1 text-sm text-neutral-300 hover:bg-neutral-800"
              onclick={() => add_member(id)}
            >
              <span>{player.display_name}</span>
              <span class="text-xs text-neutral-600">add</span>
            </button>
          {/each}
        {/if}
        <div class="px-2 pt-1">
          <FlashDisplay {flash} />
        </div>
      {/if}
    {/if}
  </div>
{/if}
