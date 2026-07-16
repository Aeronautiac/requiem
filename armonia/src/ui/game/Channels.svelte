<script lang="ts">
  import { getContext } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { GAME_STATE_KEY } from "../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState, ChannelCategory } from "../../game_state.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { ActionRequest } from "../../bindings";
  import { slotKeyFromString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  const gc_flash = new Flash();
  const pc_flash = new Flash();

  // Human-readable category headings, keyed by ChannelCategory.
  const CATEGORY_LABELS: Record<ChannelCategory, string> = {
    Lounge: "Lounges",
    Groupchat: "Group Chats",
    Notebook: "Notebooks",
    Role: "Roles",
    World: "World",
    Raw: "Misc",
    Prosecution: "Trials",
    Bug: "Bugs",
    // Personal collects the read-only Notifications feed and the player's own personal channels.
    Personal: "Personal",
    // Orgs get their own membership-gated section below, not the generic category loop.
    Org: "Organizations",
  };

  // World leads; News lives under it, so it always shows. The rest follow in their
  // canonical order.
  const CATEGORY_ORDER: ChannelCategory[] = [
    "World",
    "Role",
    "Personal",
    "Notebook",
    "Lounge",
    "Groupchat",
    "Prosecution",
    "Bug",
    "Raw",
  ];

  // only render the channels which you have had some positive perms for at some point
  // if admin, render all regardless
  const channel_categories = $derived.by(() => {
    let map = new Map<ChannelCategory, string[]>();

    function bucket(category: ChannelCategory, ch_key: string) {
      const old = map.get(category);
      if (old) old.push(ch_key);
      else map.set(category, [ch_key]);
    }

    const view =
      ui.viewer === "Admin" ? game.system_view() : game.views.get(ui.viewer);

    // Frontend-only info channels (the Notifications feed) live per-view, not in
    // game.channels; list them under their own category (Personal). Bucketed FIRST so
    // notifications render above the viewer's personal channels within that category.
    for (const [key, ch] of view?.info_channels ?? []) {
      bucket(ch.category, key);
    }

    for (const ch_key of game.channels.keys()) {
      // News is rendered separately and always (even when it doesn't exist or the
      // viewer has no perms), so skip it here to avoid rendering it twice.
      if (ch_key === game.news_channel_id) continue;

      const category = game.channels.get(ch_key)!.category;
      if (ui.viewer === "Admin") {
        bucket(category, ch_key);
      } else {
        let perms = game.views.get(ui.viewer)!.channel_views.get(ch_key)?.perms;
        if (perms && perms.had_positive) {
          bucket(category, ch_key);
        }
      }
    }

    // Bug feeds are global (game.bugs, "bug:*"); a viewer sees only the ones the engine
    // made visible to them (visible_bugs). Admin sees every bug.
    for (const [key, ch] of game.bugs) {
      if (ui.viewer !== "Admin" && !view?.visible_bugs.has(key)) continue;
      bucket(ch.category, key);
    }

    return map;
  });

  // Orgs the viewer may see: gated on view of the org's backing channel (Admin sees all),
  // the same rule as every other channel. Each opens that channel; the org member/ability
  // panel lives in the right sidebar.
  const visible_orgs = $derived.by(() => {
    const out: { key: string; name: string; channel: string }[] = [];
    const view = ui.viewer === "Admin" ? undefined : game.views.get(ui.viewer);
    for (const [key, org] of game.orgs) {
      if (ui.viewer !== "Admin") {
        const perms = view?.channel_views.get(org.channel_id)?.perms;
        if (!perms?.had_positive) continue;
      }
      out.push({ key, name: game.channels.get(org.channel_id)?.name ?? "Org", channel: org.channel_id });
    }
    return out;
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

  // Creating a personal channel is a direct player action (CreatePersonalChannel). Admins
  // aren't players, so the button is hidden for them. The engine caps how many a player may
  // hold and rejects past the limit; we just surface the error like the group-chat button.
  async function create_personal_channel() {
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload: { CreatePersonalChannel: {} },
    };
    const err = await game.dispatch(request);
    if (err) pc_flash.set_error(`Create failed: ${err}`);
    else pc_flash.set_success("Personal channel created.");
  }
</script>

<div class="flex flex-col p-2">
  {#each CATEGORY_ORDER as category}
    {@const keys = channel_categories.get(category) ?? []}
    {@const open = !collapsed.has(category)}
    <!-- World always shows (News lives under it); Personal always shows for players so the
         "add personal channel" button is reachable even with nothing in it yet; Group Chats
         likewise shows whenever the viewer can create one, so the "create group chat" button
         is always reachable. Lounges always show too (empty header), purely so the sidebar
         reads consistently above Group Chats. -->
    {#if keys.length > 0 || category === "World" || category === "Lounge" || (category === "Personal" && ui.viewer !== "Admin") || (category === "Groupchat" && gc_ability_id)}
      <section class="flex flex-col mt-1">
        <button
          class="flex items-center gap-1 px-2 py-1 text-xs font-medium uppercase tracking-wide text-neutral-500 hover:text-neutral-300"
          onclick={() => toggle(category)}
        >
          <span class="text-[0.6rem]">{open ? "▾" : "▸"}</span>
          {CATEGORY_LABELS[category]}
        </button>

        {#if open}
          {#if category === "World"}
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

          {#if category === "Groupchat" && gc_ability_id}
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

          {#if category === "Personal" && ui.viewer !== "Admin"}
            <button
              class="w-full text-left px-3 py-1 rounded text-sm text-neutral-500 hover:bg-neutral-800 hover:text-neutral-300"
              onclick={() => create_personal_channel()}
            >
              + Add personal channel
            </button>
            <div class="px-3">
              <FlashDisplay flash={pc_flash} />
            </div>
          {/if}
        {/if}
      </section>
    {/if}
  {/each}

  {#if visible_orgs.length > 0}
    {@const open = !collapsed.has("Org")}
    <section class="flex flex-col mt-1">
      <button
        class="flex items-center gap-1 px-2 py-1 text-xs font-medium uppercase tracking-wide text-neutral-500 hover:text-neutral-300"
        onclick={() => toggle("Org")}
      >
        <span class="text-[0.6rem]">{open ? "▾" : "▸"}</span>
        {CATEGORY_LABELS["Org"]}
      </button>
      {#if open}
        {#each visible_orgs as org (org.key)}
          <button
            class="w-full text-left px-3 py-1 rounded text-sm hover:bg-neutral-800 {ui.selected_channel ===
            org.channel
              ? 'bg-neutral-800'
              : ''} text-neutral-300"
            onclick={() => ui.select_channel(org.channel)}
          >
            {org.name}
          </button>
        {/each}
      {/if}
    </section>
  {/if}
</div>
