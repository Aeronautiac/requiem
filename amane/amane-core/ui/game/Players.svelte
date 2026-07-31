<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { permsLabel, playerLabel } from "../../game/helpers.svelte";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { ActorDisplay } from "../../bindings";
  import { slotKeyToString } from "../../bindings";
  import Player from "./Player.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  // The one view this component renders. Everything it can show is what this view was delivered.
  const view = $derived(game.view_of(ui.viewer));

  let channel_open = $state(true);
  let players_open = $state(true);

  // The channel whose members we show: the selected channel, or news's backing channel.
  const channel_id = $derived(
    ui.is_news ? view.news_channel_id : ui.selected_channel,
  );
  // Admin reads the System view, which the engine doesn't feed rosters to yet, so admin's member
  // list is empty for now.
  // TODO: direct ChannelRoster to System so admin sees channel members.
  const current_view = $derived(
    game.view_of(ui.viewer),
  );
  // Every name the room can see. A name it has not been told about is simply absent from the
  // roster, so there is nothing to filter out here.
  const members = $derived(
    channel_id ? (current_view?.channel_views.get(channel_id)?.roster ?? []) : [],
  );

  // Only Raw displays identify a specific player, so anonymous/role names stay in "other".
  const member_player_ids = $derived.by(() => {
    const ids = new Set<string>();
    for (const profile of members) {
      const d = profile.display;
      if (typeof d !== "string" && "Raw" in d) ids.add(slotKeyToString(d.Raw));
    }
    return ids;
  });
  const other_players = $derived(
    [...view.players.entries()].filter(([id]) => !member_player_ids.has(id)),
  );

  // Anonymous and role displays can't be contacted — you don't know who they are.
  function contact_target(display: ActorDisplay): string | null {
    return typeof display !== "string" && "Raw" in display
      ? slotKeyToString(display.Raw)
      : null;
  }
</script>

<div class="flex flex-col gap-2 p-2">
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
        {#each members as member (slotKeyToString(member.profile_id))}
          {@const pid = contact_target(member.display)}
          {#if pid}
            <Player
              id={pid}
              label={view.resolve_display(member.display)}
              perms={member.perms}
            />
          {:else}
            <!-- nothing to contact or inspect -->
            <div
              class="flex items-center justify-between px-2 py-1 text-sm text-neutral-300"
            >
              <span>{view.resolve_display(member.display)}</span>
              {#if permsLabel(member.perms)}
                <span class="text-xs text-neutral-600">
                  {permsLabel(member.perms)}
                </span>
              {/if}
            </div>
          {/if}
        {/each}
      {/if}
    {/if}
  </section>

  <section class="flex flex-col gap-0.5">
    <button
      class="flex items-center gap-1 px-2 py-1 text-xs font-medium uppercase tracking-wide text-neutral-500 hover:text-neutral-300"
      onclick={() => (players_open = !players_open)}
    >
      <span class="text-[0.6rem]">{players_open ? "▾" : "▸"}</span>
      Other Players
    </button>

    {#if players_open}
      {#each other_players as [id] (id)}
        <Player {id} label={playerLabel(id, view.players)} />
      {/each}

      {#if other_players.length === 0}
        <p class="px-2 py-1 text-xs text-neutral-600">No other players</p>
      {/if}
    {/if}
  </section>
</div>
