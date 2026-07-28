<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY, playerLabel } from "../../game_state.svelte.ts";
  import { CLIENT_KEY, type ClientState } from "../../client.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState } from "../../game_state.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { Action } from "../../bindings";
  import { slotKeyFromString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";
  import AbilityMenu from "./abilities/AbilityMenu.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);

  const client = getContext<ClientState>(CLIENT_KEY);
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

  // Whether the viewer is in the org RIGHT NOW, which is a different question from `visible`
  // above. That one asks "were you ever here" and keeps the panel around as an archive of what you
  // knew; this asks "are you here", and it is what gates acting. A former member should still see
  // the org — just not be offered its abilities as though they were still in it.
  //
  // UX, not security: the engine rejects the action regardless. This only stops the client
  // presenting something it has no business offering.
  const is_member = $derived(
    ui.viewer === "Admin" || (!!org && org.members.has(ui.viewer)),
  );

  // Org members: the full list, everyone sees it. Resolved to display names.
  const org_members = $derived(
    [...(org?.members ?? [])].map((id) => ({
      id,
      name: playerLabel(id, game.players),
    })),
  );

  // Whether a member is an OG, as far as THIS viewer is entitled to know.
  //
  // OG standing is personal info: it goes to the member and to System, never to the rest of the
  // org. So admin reads anyone's out of player_info, a player can only ever answer for themselves,
  // and everyone else is shown nothing rather than a guess.
  function is_og(id: string): boolean {
    if (!org_key) return false;
    if (ui.viewer === "Admin") {
      return game.player_info.get(id)?.og_orgs?.has(org_key) ?? false;
    }
    return id === ui.viewer && (game.views.get(ui.viewer)?.og_orgs.has(org_key) ?? false);
  }

  // Admin-only: players not already in the org, to add via AddToOrg.
  const candidates = $derived(
    [...game.players.entries()].filter(([id]) => !org?.members.has(id)),
  );

  // Configuration applied to the next add: leader makes them the org leader (requires the
  // org to have leadership); og marks them an original/founding member.
  let add_leader = $state(false);
  let add_og = $state(false);

  async function send(payload: Action, ok: string) {
    const err = await client.dispatch({
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

  function toggle_og(player_id: string) {
    if (!org_key) return;
    send(
      {
        SetOgStatus: {
          actor_id: slotKeyFromString(player_id),
          org_id: slotKeyFromString(org_key),
          og: !is_og(player_id),
        },
      },
      "OG status updated.",
    );
  }

  // No blacklist control here on purpose. Blacklisting is a low-level primitive that other
  // machinery drives (silent prosecution); an org cannot bar or unbar anyone, so offering it in the
  // org's own roster would teach the wrong model of what it is.
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
        {#if is_member}
          <AbilityMenu orgId={org_key} />
        {:else}
          <p class="text-xs text-neutral-600">
            You are no longer in this organization. Everything here is what you last saw.
          </p>
        {/if}
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
            {#if is_og(m.id)}
              <span
                class="shrink-0 rounded bg-violet-600/20 px-1.5 py-0.5 text-[0.65rem] font-medium text-violet-300"
                title="An original member. Only they and the host know this."
              >
                OG
              </span>
            {/if}
            <span class="flex-1"></span>
            {#if ui.viewer === "Admin"}
              <button
                class="shrink-0 rounded px-1.5 py-0.5 text-xs text-violet-400/80 hover:bg-neutral-800 hover:text-violet-300"
                onclick={() => toggle_og(m.id)}
                title="Toggle OG status"
              >
                og
              </button>
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
          {#each candidates as [id] (id)}
            <button
              class="flex w-full items-center justify-between rounded px-2 py-1 text-sm text-neutral-300 hover:bg-neutral-800"
              onclick={() => add_member(id)}
            >
              <span>{playerLabel(id, game.players)}</span>
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
