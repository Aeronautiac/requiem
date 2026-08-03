<script lang="ts">
  import { execErrorText } from "../../game/helpers.svelte";
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { playerLabel } from "../../game/helpers.svelte";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { Action } from "../../bindings";
  import { slotKeyFromString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";
  import AbilityMenu from "./abilities/AbilityMenu.svelte";
  import Dialog from "../kit/Dialog.svelte";
  import Button from "../kit/Button.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);

  const session = getContext<SessionState>(SESSION_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  // The one view this component renders. Everything it can show is what this view was delivered.
  const view = $derived(game.view_of(ui.viewer));

  let open = $state(false);
  const flash = new Flash();

  // The org backing the selected channel, if it's an org channel.
  const org_key = $derived(
    ui.selected_channel
      ? view.org_of_channel(ui.selected_channel)
      : undefined,
  );
  const org = $derived(org_key ? view.orgs.get(org_key) : undefined);

  // Holding the org at all is the gate: it is here because this view was delivered its MapActor.
  const visible = $derived(!!org);

  // The org's members and abilities ride its channel's viewport, so losing that viewport stops
  // both. What is on screen is then the roster as it last stood, which is worth saying out loud —
  // an org you were thrown out of does not stop having members, you just stop hearing about them.
  const frozen = $derived(
    !!org && view.frozen(view.viewport_of(ui.selected_channel ?? "")),
  );

  // Gates ACTING, where `visible` above gates showing: a former member should still see the org,
  // just not be offered its abilities. UX, not security — the engine rejects the action regardless.
  const is_member = $derived(
    ui.viewer === "Admin" || (!!org && org.members.has(ui.viewer)),
  );

  // The full list; every member sees it. `effective` marks the present members — the ones who
  // count toward the org's ability member requirements. An absent member (kidnapped, jailed, dead)
  // stays listed and acts like a normal member, but does not count, so the org sees its real reach.
  const org_members = $derived(
    [...(org?.members ?? [])].map((id) => ({
      id,
      name: playerLabel(id, view.players),
      effective: org?.effective.has(id) ?? false,
    })),
  );
  const effective_count = $derived(org_members.filter((m) => m.effective).length);

  // As far as THIS view is entitled to know, which needs no check of who is looking: OG standing
  // reaches the member and System and nobody else, so `player_info` is populated only on System
  // and `og_orgs` holds only your own. A view asked about anyone else answers no.
  function is_og(id: string): boolean {
    if (!org_key) return false;
    if (view.player_info.get(id)?.og_orgs?.has(org_key)) return true;
    return id === ui.viewer && view.og_orgs.has(org_key);
  }

  // Admin-only: players not already in the org.
  const candidates = $derived(
    [...view.players.entries()].filter(([id]) => !org?.members.has(id)),
  );

  // Applied to the next add. `leader` requires the org to have leadership at all.
  let add_leader = $state(false);
  let add_og = $state(false);

  async function send(payload: Action, ok: string) {
    const reply = await session.submit_action({
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload,
    });
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else flash.set_success(ok);
  }

  function add_member(player_id: string) {
    if (!org_key) return;
    send(
      {
        AddToOrg: {
          actor_id: slotKeyFromString(player_id),
          org_id: slotKeyFromString(org_key),
          leader: add_leader,
          og: add_og,
        },
      },
      "Added.",
    );
  }

  function remove_member(player_id: string) {
    if (!org_key) return;
    send(
      {
        RemoveFromOrg: {
          actor_id: slotKeyFromString(player_id),
          org_id: slotKeyFromString(org_key),
        },
      },
      "Removed.",
    );
  }

  function toggle_og(player_id: string) {
    if (!org_key) return;
    send(
      {
        SetOgStatus: {
          actor_id: slotKeyFromString(player_id),
          org_id: slotKeyFromString(org_key),
          og: !is_og(player_id),
        },
      },
      "OG status updated.",
    );
  }

  // No blacklist control here on purpose: it is a low-level primitive that other machinery drives
  // (silent prosecution). An org cannot bar or unbar anyone, so offering it on the org's own
  // roster would teach the wrong model of what it is.
</script>

{#if visible && org && org_key}
  <Button variant="ghost" size="sm" class="w-full justify-start" onclick={() => (open = true)}>
    Organization
  </Button>

  <Dialog bind:open title="Organization" class="max-w-sm">
    <div class="flex flex-col gap-1">
      <!-- frozen outranks is_member, because is_member reads the global roster and the roster is
           one of the things that stopped updating. A view that has lost the viewport can still be
           listed in an org it was thrown out of. -->
      <div class="pt-1">
        {#if frozen}
          <p class="text-xs text-amber-500/70">
            This organization no longer reaches you. Everything here is what you last saw.
          </p>
        {:else if is_member}
          <AbilityMenu orgId={org_key} />
        {:else}
          <p class="text-xs text-neutral-600">
            You are no longer in this organization. Everything here is what you last saw.
          </p>
        {/if}
      </div>

      <p class="flex flex-wrap gap-x-1 px-2 pt-2 text-[0.65rem] uppercase tracking-wide text-neutral-600">
        <span>Members</span>
        <span class="whitespace-nowrap">[ {org_members.length} total | {effective_count} counted ]</span>
      </p>
      {#if org_members.length === 0}
        <p class="px-2 py-1 text-xs text-neutral-600">No members</p>
      {:else}
        {#each org_members as m (m.id)}
          <div class="flex items-center justify-between gap-1 rounded px-2 py-1 text-sm text-neutral-300">
            <span class="truncate {m.effective ? '' : 'text-neutral-500'}">{m.name}</span>
            {#if !m.effective}
              <span
                class="shrink-0 rounded bg-neutral-800 px-1.5 py-0.5 text-[0.65rem] font-medium text-neutral-500"
                title="Not present, so not counted toward this org's ability member requirements. Still a full member."
              >
                not counted
              </span>
            {/if}
            {#if is_og(m.id)}
              <span
                class="shrink-0 rounded bg-violet-600/20 px-1.5 py-0.5 text-[0.65rem] font-medium text-violet-300"
                title="An original member. Only they and the host know this."
              >
                OG
              </span>
            {/if}
            <span class="flex-1"></span>
            {#if ui.viewer === "Admin"}
              <button
                class="shrink-0 rounded px-1.5 py-0.5 text-xs text-violet-400/80 hover:bg-neutral-800 hover:text-violet-300"
                onclick={() => toggle_og(m.id)}
                title="Toggle OG status"
              >
                og
              </button>
              <button
                class="shrink-0 rounded px-1.5 py-0.5 text-xs text-red-400/80 hover:bg-neutral-800 hover:text-red-300"
                onclick={() => remove_member(m.id)}
                title="Remove from org"
              >
                remove
              </button>
            {/if}
          </div>
        {/each}
      {/if}

      {#if ui.viewer === "Admin"}
        <p class="px-2 pt-2 text-[0.65rem] uppercase tracking-wide text-neutral-600">
          Add member
        </p>
        <div class="flex gap-3 px-2 py-1 text-xs text-neutral-400">
          <label class="flex items-center gap-1">
            <input type="checkbox" bind:checked={add_leader} /> leader
          </label>
          <label class="flex items-center gap-1">
            <input type="checkbox" bind:checked={add_og} /> og
          </label>
        </div>
        {#if candidates.length === 0}
          <p class="px-2 py-1 text-xs text-neutral-600">No one to add</p>
        {:else}
          {#each candidates as [id] (id)}
            <button
              class="flex w-full items-center justify-between rounded px-2 py-1 text-sm text-neutral-300 hover:bg-neutral-800"
              onclick={() => add_member(id)}
            >
              <span>{playerLabel(id, view.players)}</span>
              <span class="text-xs text-neutral-600">add</span>
            </button>
          {/each}
        {/if}
        <div class="px-2 pt-1">
          <FlashDisplay {flash} />
        </div>
      {/if}
    </div>
  </Dialog>
{/if}
