<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState } from "../../game_state.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { ActionRequest, ActionPayload } from "../../bindings";
  import { slotKeyFromString, slotKeyToString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  const flash = new Flash();
  let open = $state(false);

  // The selected channel's backing gc, if it is a group chat at all.
  const gc_key = $derived(
    ui.selected_channel
      ? game.gc_key_for_channel(ui.selected_channel)
      : undefined,
  );

  // Only the gc's owner sees the controls. Admin has no view, so it's never owner.
  const is_owner = $derived(
    !!gc_key && (game.views.get(ui.viewer)?.owned_gcs.has(gc_key) ?? false),
  );

  // Current members the owner can act on: Raw displays only (a specific player), and
  // never the owner themselves. Role/anonymous members can't be targeted by id.
  const members = $derived.by(() => {
    if (!ui.selected_channel) return [];
    const entry = game.views.get(ui.viewer)?.channel_views.get(ui.selected_channel);
    const out: { id: string; name: string }[] = [];
    for (const [, m] of entry?.members ?? []) {
      const d = m.display;
      if (!m.had_positive) continue;
      if (typeof d !== "string" && "Raw" in d) {
        const id = slotKeyToString(d.Raw);
        if (id === ui.viewer) continue;
        out.push({ id, name: game.resolve_display(d) });
      }
    }
    return out;
  });

  const member_ids = $derived(new Set(members.map((m) => m.id)));

  // Players not already members (and not the owner) — candidates to add.
  const candidates = $derived(
    [...game.players.entries()].filter(
      ([id]) => id !== ui.viewer && !member_ids.has(id),
    ),
  );

  async function send(payload: ActionPayload, ok: string) {
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload,
    };
    const err = await game.dispatch(request);
    if (err) flash.set_error(err);
    else flash.set_success(ok);
  }

  function add(player_id: string) {
    if (!gc_key) return;
    send(
      {
        AddToGroupchat: {
          groupchat_id: slotKeyFromString(gc_key),
          player_id: slotKeyFromString(player_id),
          owner: false,
        },
      },
      "Added.",
    );
  }

  function remove(player_id: string) {
    if (!gc_key) return;
    send(
      {
        RemoveFromGroupchat: {
          groupchat_id: slotKeyFromString(gc_key),
          player_id: slotKeyFromString(player_id),
        },
      },
      "Removed.",
    );
  }

  function transfer(player_id: string) {
    if (!gc_key) return;
    send(
      {
        SetGroupchatOwner: {
          groupchat_id: slotKeyFromString(gc_key),
          owner: slotKeyFromString(player_id),
        },
      },
      "Ownership transferred.",
    );
  }
</script>

{#if is_owner}
  <div class="flex flex-col gap-1 border-b border-neutral-800 p-2">
    <button
      class="flex items-center gap-1 px-2 py-1 text-xs font-medium uppercase tracking-wide text-neutral-400 hover:text-neutral-200"
      onclick={() => (open = !open)}
    >
      <span class="text-[0.6rem]">{open ? "▾" : "▸"}</span>
      Group Chat Controls
    </button>

    {#if open}
      <!-- Add a member -->
      <p class="px-2 pt-1 text-[0.65rem] uppercase tracking-wide text-neutral-600">
        Add member
      </p>
      {#if candidates.length === 0}
        <p class="px-2 py-1 text-xs text-neutral-600">No one to add</p>
      {:else}
        {#each candidates as [id, player] (id)}
          <button
            class="flex w-full items-center justify-between rounded px-2 py-1 text-sm text-neutral-300 hover:bg-neutral-800"
            onclick={() => add(id)}
          >
            <span>{player.display_name}</span>
            <span class="text-xs text-neutral-600">add</span>
          </button>
        {/each}
      {/if}

      <!-- Existing members: remove or hand ownership -->
      <p class="px-2 pt-2 text-[0.65rem] uppercase tracking-wide text-neutral-600">
        Members
      </p>
      {#if members.length === 0}
        <p class="px-2 py-1 text-xs text-neutral-600">No other members</p>
      {:else}
        {#each members as m (m.id)}
          <div
            class="flex items-center justify-between gap-1 rounded px-2 py-1 text-sm text-neutral-300"
          >
            <span class="truncate">{m.name}</span>
            <span class="flex shrink-0 gap-1">
              <button
                class="rounded px-1.5 py-0.5 text-xs text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200"
                onclick={() => transfer(m.id)}
                title="Make owner"
              >
                owner
              </button>
              <button
                class="rounded px-1.5 py-0.5 text-xs text-red-400/80 hover:bg-neutral-800 hover:text-red-300"
                onclick={() => remove(m.id)}
                title="Remove from group chat"
              >
                remove
              </button>
            </span>
          </div>
        {/each}
      {/if}

      <div class="px-2 pt-1">
        <FlashDisplay {flash} />
      </div>
    {/if}
  </div>
{/if}
