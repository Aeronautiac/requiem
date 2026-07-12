<script lang="ts">
  import { getContext } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import {
    GAME_STATE_KEY,
    CHANNEL_KINDS,
  } from "../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState, ChannelKind } from "../../game_state.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { ActionRequest } from "../../bindings";
  import { slotKeyFromString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  const gc_flash = new Flash();

  // Human-readable category headings, keyed by ChannelKind.
  const KIND_LABELS: Record<ChannelKind, string> = {
    Raw: "Direct",
    Lounge: "Lounges",
    Groupchat: "Group Chats",
    Notebook: "Notebooks",
    Role: "Roles",
    World: "World",
    Info: "Info",
  };

  // World leads; News lives under it, so it always shows. The rest follow in their
  // canonical order.
  const CATEGORY_ORDER: ChannelKind[] = [
    "World",
    ...CHANNEL_KINDS.filter((k) => k !== "World"),
  ];

  // only render the channels which you have had some positive perms for at some point
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
      // News is rendered separately and always (even when it doesn't exist or the
      // viewer has no perms), so skip it here to avoid rendering it twice.
      if (ch_key === game.news_channel_id) continue;

      if (ui.viewer === "Admin") {
        push(ch_key);
      } else {
        let view = game.views.get(ui.viewer)!;
        let perms = view.channel_views.get(ch_key)?.perms;
        if (perms && perms.had_positive) {
          push(ch_key);
        }
      }
    }

    // Frontend-only info channels live per-view, not in game.channels, so pull them
    // from the viewer's own view and list them under the Info category.
    const view =
      ui.viewer === "Admin" ? game.system_view() : game.views.get(ui.viewer);
    for (const key of view?.info_channels.keys() ?? []) {
      const existing = map.get("Info");
      if (existing) existing.push(key);
      else map.set("Info", [key]);
    }

    return map;
  });

  // Categories start expanded.
  const collapsed = new SvelteSet<ChannelKind>();

  function toggle(kind: ChannelKind) {
    if (collapsed.has(kind)) {
      collapsed.delete(kind);
    } else {
      collapsed.add(kind);
    }
  }

  // Creating a group chat is a player action driven by the viewer's CreateGroupchat
  // ability, exactly like Contact. Admins have no ability view, so this is empty for
  // them and the button is hidden.
  const gc_ability_id = $derived(
    ui.viewer === "Admin"
      ? undefined
      : game.find_abilities(ui.viewer, "CreateGroupchat")[0],
  );

  async function create_gc(ability_id: string) {
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload: {
        UseAbility: {
          ability_id: slotKeyFromString(ability_id),
          ability_args: { CreateGroupchat: {} },
        },
      },
    };
    const err = await game.dispatch(request);
    if (err) {
      gc_flash.set_error(`Create failed: ${err}`);
    } else {
      gc_flash.set_success("Group chat created.");
    }
  }
</script>

<div class="flex flex-col p-2">
  <!-- Every category is always rendered, even when empty, so players can see what
       kinds of channels can appear. -->
  {#each CATEGORY_ORDER as kind}
    {@const keys = channel_categories.get(kind) ?? []}
    {@const open = !collapsed.has(kind)}
    <section class="flex flex-col mt-1">
        <button
          class="flex items-center gap-1 px-2 py-1 text-xs font-medium uppercase tracking-wide text-neutral-500 hover:text-neutral-300"
          onclick={() => toggle(kind)}
        >
          <span class="text-[0.6rem]">{open ? "▾" : "▸"}</span>
          {KIND_LABELS[kind]}
        </button>

        {#if open}
          {#if kind === "World"}
            <button
              class="w-full text-left px-3 py-1 rounded text-sm hover:bg-neutral-800 {ui.is_news
                ? 'bg-neutral-800'
                : ''} text-neutral-300"
              onclick={() => ui.select_news()}
            >
              News
            </button>
          {/if}

          {#each keys as key}
            {@const channel = game.resolve_channel(ui.viewer, key)!}
            <button
              class="w-full text-left px-3 py-1 rounded text-sm hover:bg-neutral-800 {channel.archived
                ? 'text-neutral-600'
                : 'text-neutral-300'} {ui.selected_channel === key
                ? 'bg-neutral-800'
                : ''}"
              onclick={() => ui.select_channel(key)}
            >
              {channel.name}
            </button>
          {/each}

          {#if kind === "Groupchat" && gc_ability_id}
            <button
              class="w-full text-left px-3 py-1 rounded text-sm text-neutral-500 hover:bg-neutral-800 hover:text-neutral-300"
              onclick={() => create_gc(gc_ability_id)}
            >
              + Create group chat
            </button>
            <div class="px-3">
              <FlashDisplay flash={gc_flash} />
            </div>
          {/if}
      {/if}
    </section>
  {/each}
</div>
