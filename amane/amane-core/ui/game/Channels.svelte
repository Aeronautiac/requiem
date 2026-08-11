<script lang="ts">
  import { channelLabel, execErrorText } from "../../game/helpers.svelte";
  import { getContext } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import type { ChannelCategory } from "../../game/types";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { ActionRequest } from "../../bindings";
  import { slotKeyFromString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";
  import Dropdown from "../kit/Dropdown.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);

  const session = getContext<SessionState>(SESSION_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  // The one view this component renders. Everything it can show is what this view was delivered.
  const view = $derived(game.view_of(ui.viewer));
  // System is not a player, so it gets no "create your own" affordances. Acting, not visibility.
  const is_admin = $derived(ui.viewer === "Admin");

  const gc_flash = new Flash();
  const pc_flash = new Flash();

  const CATEGORY_LABELS: Record<ChannelCategory, string> = {
    Lounge: "Lounges",
    Groupchat: "Group Chats",
    Notebook: "Notebooks",
    Role: "Roles",
    World: "World",
    Kidnapping: "Kidnapping",
    Raw: "Misc",
    Prosecution: "Trials",
    Logs: "Logs",
    Personal: "Personal",
    Org: "Organizations",
  };

  const CATEGORY_ORDER: ChannelCategory[] = [
    "World",
    "Role",
    "Logs",
    "Org",
    "Personal",
    "Notebook",
    "Lounge",
    "Groupchat",
    "Prosecution",
    "Kidnapping",
    "Raw",
  ];

  // Everything this view holds, grouped for the sidebar.
  const channel_categories = $derived.by(() => {
    let map = new Map<ChannelCategory, string[]>();

    function bucket(category: ChannelCategory, ch_key: string) {
      const old = map.get(category);
      if (old) old.push(ch_key);
      else map.set(category, [ch_key]);
    }

    // Info channels are built by the view rather than delivered to it. Bucketed FIRST so
    // notifications render above the viewer's personal channels within the Personal category.
    for (const [key, ch] of view.info_channels) {
      bucket(ch.category, key);
    }

    // No permission check. Holding the channel at all means this view was delivered its Map*,
    // which means it held the membership viewport, which means it could read. Presence in the
    // view IS the gate.
    for (const [ch_key, ch] of view.channels) {
      // News is rendered separately and unconditionally, so skip it to avoid rendering it twice.
      if (ch_key === view.news_channel_id) continue;
      bucket(ch.category, ch_key);
    }

    for (const [key, ch] of view.bugs) bucket(ch.category, key);
    for (const [key, ch] of view.contact_logs) bucket(ch.category, key);
    for (const [key, ch] of view.logs) bucket(ch.category, key);

    return map;
  });

  // Categories start expanded.
  const collapsed = new SvelteSet<ChannelCategory>();

  function toggle(category: ChannelCategory) {
    if (collapsed.has(category)) {
      collapsed.delete(category);
    } else {
      collapsed.add(category);
    }
  }

  // Driven by the viewer's own ability, exactly like Contact. A view holding no such ability shows
  // no button, which covers System without asking whether it is System.
  const gc_ability_id = $derived(view.find_abilities("CreateGroupchat")[0]);

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
    const reply = await session.submit_action(request);
    if (!reply.ok) {
      gc_flash.set_error(`Create failed: ${execErrorText(reply.error)}`);
    } else {
      gc_flash.set_success("Group chat created.");
    }
  }

  // A direct player action rather than an ability, so admins get no button. The engine caps how
  // many a player may hold and rejects past the limit; we just surface the error.
  async function create_personal_channel() {
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload: { CreatePersonalChannel: {} },
    };
    const reply = await session.submit_action(request);
    if (!reply.ok)
      pc_flash.set_error(`Create failed: ${execErrorText(reply.error)}`);
    else pc_flash.set_success("Personal channel created.");
  }
</script>

<div class="flex flex-col gap-1.5 pb-2">
  {#each CATEGORY_ORDER as category}
    {@const keys = channel_categories.get(category) ?? []}
    {@const open = !collapsed.has(category)}
    <!-- Categories that show even when empty: World holds News, Personal and Group Chats keep
         their create buttons reachable, and Lounges is there purely so the sidebar reads
         consistently above Group Chats. -->
    {#if keys.length > 0 || category === "World" || category === "Lounge" || (category === "Personal" && !is_admin) || (category === "Groupchat" && gc_ability_id)}
      <Dropdown label={CATEGORY_LABELS[category]} {open} onToggle={() => toggle(category)}>
          {#if category === "World"}
            <button
              class="w-full text-left px-3 py-2 rounded text-sm leading-none hover:bg-neutral-800 {ui.is_news
                ? 'bg-neutral-800'
                : ''} text-neutral-300"
              onclick={() => ui.select_news()}
            >
              News
            </button>
          {/if}

          {#each keys as key}
            {@const channel = view.channel(key)!}
            <button
              class="w-full text-left px-3 py-2 rounded text-sm leading-none hover:bg-neutral-800 {channel.archived
                ? 'text-neutral-600'
                : 'text-neutral-300'} {ui.selected_channel === key
                ? 'bg-neutral-800'
                : ''}"
              onclick={() => ui.select_channel(key)}
            >
              {channelLabel(channel.name)}
            </button>
          {/each}

          {#if category === "Groupchat" && gc_ability_id}
            <button
              class="w-full text-left px-3 py-2 rounded text-sm leading-none text-neutral-500 hover:bg-neutral-800 hover:text-neutral-300"
              onclick={() => create_gc(gc_ability_id)}
            >
              + Create group chat
            </button>
            <div class="px-3">
              <FlashDisplay flash={gc_flash} />
            </div>
          {/if}

          {#if category === "Personal" && !is_admin}
            <button
              class="w-full text-left px-3 py-2 rounded text-sm leading-none text-neutral-500 hover:bg-neutral-800 hover:text-neutral-300"
              onclick={() => create_personal_channel()}
            >
              + Add personal channel
            </button>
            <div class="px-3">
              <FlashDisplay flash={pc_flash} />
            </div>
          {/if}
      </Dropdown>
    {/if}
  {/each}
</div>
