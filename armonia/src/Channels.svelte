<script lang="ts">
  import { SvelteSet } from "svelte/reactivity";
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "./game_state.svelte.ts";
  import { UI_STATE_KEY } from "./ui_state.svelte.ts";
  import type { GameState, ChannelKind } from "./game_state.svelte.ts";
  import type { UiState } from "./ui_state.svelte.ts";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  const KINDS: ChannelKind[] = ["Lounge", "Groupchat", "Notebook", "News", "Courtroom", "Raw"];

  const channels = $derived(game.views.get(ui.viewer)?.channels ?? new Map());

  const by_kind = $derived.by(() => {
    const map = new Map<ChannelKind, [string, typeof channels extends Map<string, infer V> ? V : never][]>();
    for (const kind of KINDS) map.set(kind, []);
    for (const [id, ch] of channels.entries()) {
      map.get(ch.kind)!.push([id, ch]);
    }
    return map;
  });

  const open = new SvelteSet<ChannelKind>(KINDS);
</script>

<div class="flex flex-col p-2">
  {#each KINDS as kind (kind)}
    {@const entries = by_kind.get(kind) ?? []}
    {#if entries.length > 0}
      <div class="mb-1">
        <button
          class="w-full text-left px-2 py-0.5 text-xs font-semibold text-neutral-500 uppercase tracking-wide hover:text-neutral-300"
          onclick={() => open.has(kind) ? open.delete(kind) : open.add(kind)}
        >
          {kind}
        </button>
        {#if open.has(kind)}
          {#each entries as [id, channel] (id)}
            <button
              class="w-full text-left px-3 py-1 rounded text-sm hover:bg-neutral-800
                     {channel.archived ? 'text-neutral-600' : 'text-neutral-300'}
                     {ui.selected_channel === id ? 'bg-neutral-800' : ''}"
              onclick={() => (ui.selected_channel = id)}
            >
              {channel.name}
            </button>
          {/each}
        {/if}
      </div>
    {/if}
  {/each}
  {#if channels.size === 0}
    <p class="px-2 py-1 text-xs text-neutral-600">No channels</p>
  {/if}
</div>
