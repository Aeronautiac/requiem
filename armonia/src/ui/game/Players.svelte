<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState } from "../../game_state.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { ActionRequest, ActorDisplay } from "../../bindings";
  import { slotKeyFromString, slotKeyToString } from "../../bindings";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";
  import { viewerToActor } from "../../types";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let expanded = $state<string | null>(null);
  let channel_open = $state(true);
  let players_open = $state(true);
  const flash = new Flash();

  // The channel whose members we show: the selected channel, or news's backing channel.
  const channel_id = $derived(
    ui.is_news ? game.news_channel_id : ui.selected_channel,
  );
  // Members live per-view. Admin reads the System view, which the engine doesn't feed
  // membership to yet, so admin's member list is empty for now.
  // TODO: emit ShowChannelMember to System so admin sees channel members.
  const current_view = $derived(
    ui.viewer === "Admin" ? game.system_view() : game.views.get(ui.viewer),
  );
  // Only effective members: those that have ever held a positive permission. A player
  // added with no perms was never a real participant and belongs in "other players".
  const members = $derived.by(() => {
    const all = channel_id
      ? current_view?.channel_views.get(channel_id)?.members
      : undefined;
    return [...(all?.entries() ?? [])].filter(([, m]) => m.had_positive);
  });

  // Players already shown as channel members (only Raw displays identify a specific
  // player; anonymous/role displays can't be matched, so those players stay in "other").
  const member_player_ids = $derived.by(() => {
    const ids = new Set<string>();
    for (const [, m] of members) {
      const d = m.display;
      if (typeof d !== "string" && "Raw" in d) ids.add(slotKeyToString(d.Raw));
    }
    return ids;
  });
  const other_players = $derived(
    [...game.players.entries()].filter(([id]) => !member_player_ids.has(id)),
  );

  async function contact(target_id_str: string, ability_id_str: string) {
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload: {
        UseAbility: {
          ability_id: slotKeyFromString(ability_id_str),
          ability_args: {
            Contact: { target_id: slotKeyFromString(target_id_str) },
          },
        },
      },
    };
    const err = await game.dispatch(request);
    if (err) {
      flash.set_error(`Contact failed: ${err}`);
    } else {
      flash.set_success("Contact sent.");
      expanded = null;
    }
  }

  function contact_abilities() {
    return [...(game.views.get(ui.viewer)?.abilities.entries() ?? [])].filter(
      ([, av]) => av.name === "Contact",
    );
  }

  // send = bit 0, read = bit 1 (matches the UpdateChannelView perms parsing).
  function perms_label(perms: number): string {
    const parts: string[] = [];
    if (perms & 2) parts.push("read");
    if (perms & 1) parts.push("send");
    return parts.join(" · ");
  }

  // The player a display identifies, if any. Only Raw displays name a specific player;
  // anonymous/role displays can't be contacted (you don't know who they are).
  function contact_target(display: ActorDisplay): string | null {
    return typeof display !== "string" && "Raw" in display
      ? slotKeyToString(display.Raw)
      : null;
  }
</script>

<!-- A clickable person row: click to expand contact options for player `id`. `perms`
     shows a read/send hint (channel members); pass null to omit it (other players). -->
{#snippet contactRow(id: string, label: string, perms: number | null)}
  <div class="rounded text-sm">
    <button
      class="flex w-full items-center justify-between px-2 py-1 rounded text-neutral-300 hover:bg-neutral-800"
      onclick={() => {
        expanded = expanded === id ? null : id;
        flash.error = null;
        flash.success = null;
      }}
    >
      <span>{label}</span>
      {#if perms !== null && perms_label(perms)}
        <span class="text-xs text-neutral-600">{perms_label(perms)}</span>
      {/if}
    </button>

    {#if expanded === id}
      <div class="ml-2 mt-0.5 flex flex-col gap-0.5">
        {#each contact_abilities() as [ability_id, av] (ability_id)}
          <button
            class="px-2 py-0.5 text-xs text-left rounded text-neutral-400 hover:bg-neutral-800"
            onclick={() => contact(id, ability_id)}
          >
            Contact ({av.success_usages_remaining}, resets in {av.iterations_to_reset})
          </button>
        {/each}
        <FlashDisplay {flash} />
      </div>
    {/if}
  </div>
{/snippet}

<div class="flex flex-col gap-2 p-2">
  <!-- Channel-specific member list -->
  <section class="flex flex-col gap-0.5">
    <button
      class="flex items-center gap-1 px-2 py-1 text-xs font-medium uppercase tracking-wide text-neutral-500 hover:text-neutral-300"
      onclick={() => (channel_open = !channel_open)}
    >
      <span class="text-[0.6rem]">{channel_open ? "▾" : "▸"}</span>
      Channel Members
    </button>

    {#if channel_open}
      {#if !channel_id}
        <p class="px-2 py-1 text-xs text-neutral-600">No channel selected</p>
      {:else if members.length === 0}
        <p class="px-2 py-1 text-xs text-neutral-600">No members</p>
      {:else}
        {#each members as [key, member] (key)}
          {@const pid = contact_target(member.display)}
          {#if pid}
            {@render contactRow(
              pid,
              game.resolve_display(member.display),
              member.perms,
            )}
          {:else}
            <!-- anonymous/role member: nothing to contact -->
            <div
              class="flex items-center justify-between px-2 py-1 text-sm text-neutral-300"
            >
              <span>{game.resolve_display(member.display)}</span>
              {#if perms_label(member.perms)}
                <span class="text-xs text-neutral-600">
                  {perms_label(member.perms)}
                </span>
              {/if}
            </div>
          {/if}
        {/each}
      {/if}
    {/if}
  </section>

  <!-- Other players (those not shown as members of the selected channel) -->
  <section class="flex flex-col gap-0.5">
    <button
      class="flex items-center gap-1 px-2 py-1 text-xs font-medium uppercase tracking-wide text-neutral-500 hover:text-neutral-300"
      onclick={() => (players_open = !players_open)}
    >
      <span class="text-[0.6rem]">{players_open ? "▾" : "▸"}</span>
      Other Players
    </button>

    {#if players_open}
      {#each other_players as [id, player] (id)}
        {@render contactRow(id, player.display_name, null)}
      {/each}

      {#if other_players.length === 0}
        <p class="px-2 py-1 text-xs text-neutral-600">No other players</p>
      {/if}
    {/if}
  </section>
</div>
