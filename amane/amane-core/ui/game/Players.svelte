<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { permsLabel, playerLabel } from "../../game/helpers.svelte";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { ActorDisplay, ChannelProfileView } from "../../bindings";
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
  const channel_view = $derived(
    channel_id ? view.channel_views.get(channel_id) : undefined,
  );
  // Every name the room can see. A name it has not been told about is simply absent from the
  // roster, so there is nothing to filter out here.
  const members = $derived(channel_view?.roster ?? []);

  // Who is behind each name, keyed by profile_id. Only the admin view is ever told this, so for a
  // player it stays empty — a mask stays a mask. Where it IS known, a masked row can name its
  // holder; a Raw row already is its holder, so it needs nothing added.
  const owners_by_profile = $derived.by(() => {
    const map = new Map<string, string[]>();
    for (const entry of channel_view?.owners ?? []) {
      map.set(
        slotKeyToString(entry.profile_id),
        entry.owners.map((a) => playerLabel(slotKeyToString(a), view.players)),
      );
    }
    return map;
  });

  // A member who holds nothing here is present but mute — no read, no send. They render in their
  // own group so a room full of names doesn't bury who can actually act in it.
  const active_members = $derived(members.filter((m) => m.perms !== 0));
  const silent_members = $derived(members.filter((m) => m.perms === 0));

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

{#snippet memberRow(member: ChannelProfileView)}
  {@const pid = contact_target(member.display)}
  {#if pid}
    <Player
      id={pid}
      label={view.resolve_display(member.display)}
      perms={member.perms}
    />
  {:else}
    {@const owners = owners_by_profile.get(slotKeyToString(member.profile_id))}
    <!-- nothing to contact or inspect -->
    <div
      class="flex items-center justify-between px-2 py-1 text-sm text-neutral-300"
    >
      <span class="flex items-center gap-1.5">
        {view.resolve_display(member.display)}
        <!-- Admin only: the holder behind a mask the room cannot see through. -->
        {#if owners && owners.length > 0}
          <span class="text-xs text-neutral-500">({owners.join(", ")})</span>
        {/if}
      </span>
      {#if permsLabel(member.perms)}
        <span class="text-xs text-neutral-600">
          {permsLabel(member.perms)}
        </span>
      {/if}
    </div>
  {/if}
{/snippet}

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
        {#each active_members as member (slotKeyToString(member.profile_id))}
          {@render memberRow(member)}
        {/each}

        {#if silent_members.length > 0}
          <p
            class="px-2 pt-2 pb-0.5 text-[0.6rem] font-medium uppercase tracking-wide text-neutral-600"
          >
            No permissions
          </p>
          {#each silent_members as member (slotKeyToString(member.profile_id))}
            {@render memberRow(member)}
          {/each}
        {/if}
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
