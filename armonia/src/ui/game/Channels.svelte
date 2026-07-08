<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import type { GameState, ChannelKind } from "../../game_state.svelte.ts";
  import { UiState } from "../../ui_state.svelte.ts";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  // only render the channels which you have had some positive perms for at for at some point
  // if admin, render all regardless
  const channel_categories = $derived.by(() => {
    let map = new Map<ChannelKind, string[]>();

    function push(ch_key: string) {
      const ch = game.channels.get(ch_key)!;
      const old = map.get(ch.kind);
      if (old) {
        old.push(ch_key);
      } else {
        map.set(ch.kind, [ch_key]);
      }
    }

    for (const ch_key of game.channels.keys()) {
      if (ui.viewer === "Admin") {
        push(ch_key);
      } else {
        let view = game.views.get(ui.viewer)!;
        let perms = view.channel_perms.get(ch_key);
        if (perms && perms.had_positive) {
          push(ch_key);
        }
      }
    }

    return map;
  });

  // for dropdown
  // const open = new SvelteSet<ChannelKind>(CHANNEL_KINDS);
</script>

<div class="flex flex-col p-2">
  {#if channel_categories.size === 0}
    <p class="px-2 py-1 text-xs text-neutral-600">No channels</p>
  {/if}

  {#each channel_categories.keys() as category}
    {#each channel_categories.get(category)! as key}
      {@const channel = game.channels.get(key)!}
      <div>
        <button
          class="w-full text-left px-3 py-1 rounded text-sm hover:bg-neutral-800 {channel.archived
            ? 'text-neutral-600'
            : 'text-neutral-300'} {ui.selected_channel === key
            ? 'bg-neutral-800'
            : ''}"
          onclick={() => (ui.selected_channel = key)}
        >
          {channel.name}
        </button>
      </div>
    {/each}
  {/each}
</div>
