<script lang="ts">
  import { execErrorText } from "../../game/helpers.svelte";
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import Name from "./Name.svelte";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { ActionRequest, Action } from "../../bindings";
  import { slotKeyFromString, slotKeyToString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";
  import Dropdown from "../kit/Dropdown.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);

  const session = getContext<SessionState>(SESSION_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  // The one view this component renders. Everything it can show is what this view was delivered.
  const view = $derived(game.view_of(ui.viewer));

  const flash = new Flash();
  let open = $state(false);

  // The selected channel's backing gc, if it is a group chat at all.
  const gc_key = $derived(
    ui.selected_channel
      ? view.gc_of(ui.selected_channel)
      : undefined,
  );

  // Only the gc's owner sees the controls. Admin has no view, so it's never owner.
  const is_owner = $derived(
    !!gc_key && (view.owned_gcs.has(gc_key) ?? false),
  );

  // Current members the owner can act on: Raw displays only (a specific player), and
  // never the owner themselves. Role/anonymous members can't be targeted by id.
  const members = $derived.by(() => {
    if (!ui.selected_channel) return [];
    const entry = view.channel_views.get(ui.selected_channel);
    const out: { id: string; name: string }[] = [];
    for (const m of entry?.roster ?? []) {
      const d = m.display;
      if (typeof d !== "string" && "Raw" in d) {
        const id = slotKeyToString(d.Raw);
        if (id === ui.viewer) continue;
        out.push({ id, name: view.resolve_display(d) });
      }
    }
    return out;
  });

  const member_ids = $derived(new Set(members.map((m) => m.id)));

  // Players not already members (and not the owner) — candidates to add.
  const candidates = $derived(
    [...view.players.entries()].filter(
      ([id]) => id !== ui.viewer && !member_ids.has(id),
    ),
  );

  async function send(payload: Action, ok: string) {
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload,
    };
    const reply = await session.submit_action(request);
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
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
  <div class="flex flex-col pb-1.5">
    <Dropdown label="Group Chat Controls" {open} onToggle={() => (open = !open)}>
      <!-- Add a member -->
      <p class="px-3 pt-2 pb-0.5 text-[0.6rem] font-medium uppercase tracking-wide text-neutral-600">
        Add member
      </p>
      {#if candidates.length === 0}
        <p class="px-2 py-1 text-xs text-neutral-600">No one to add</p>
      {:else}
        {#each candidates as [id] (id)}
          <button
            class="flex w-full items-center justify-between px-3 py-1.5 text-sm text-neutral-300 hover:bg-neutral-800"
            onclick={() => add(id)}
          >
            <span class="min-w-0 truncate"><Name {id} {view} menu={false} /></span>
            <span class="shrink-0 text-xs text-neutral-600">add</span>
          </button>
        {/each}
      {/if}

      <!-- Existing members: remove or hand ownership -->
      <p class="px-3 pt-2 pb-0.5 text-[0.6rem] font-medium uppercase tracking-wide text-neutral-600">
        Members
      </p>
      {#if members.length === 0}
        <p class="px-2 py-1 text-xs text-neutral-600">No other members</p>
      {:else}
        {#each members as m (m.id)}
          <div
            class="flex items-center justify-between gap-2 px-3 py-1.5 text-sm text-neutral-300"
          >
            <span class="min-w-0 truncate">{m.name}</span>
            <span class="flex shrink-0 gap-1">
              <button
                class="px-1.5 py-0.5 text-xs text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200"
                onclick={() => transfer(m.id)}
                title="Make owner"
              >
                owner
              </button>
              <button
                class="px-1.5 py-0.5 text-xs text-red-400/80 hover:bg-neutral-800 hover:text-red-300"
                onclick={() => remove(m.id)}
                title="Remove from group chat"
              >
                remove
              </button>
            </span>
          </div>
        {/each}
      {/if}

      <div class="px-3 py-1.5">
        <FlashDisplay {flash} />
      </div>
    </Dropdown>
  </div>
{/if}
