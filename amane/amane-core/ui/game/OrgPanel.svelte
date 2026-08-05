<script lang="ts">
  import { execErrorText } from "../../game/helpers.svelte";
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import {
    orgColorVar,
    orgDisplayName,
    playerLabel,
  } from "../../game/helpers.svelte";
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
  import Name from "./Name.svelte";

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

  // `org.leader` is the leader as far as this view is entitled to know: the whole answer on the
  // admin's System copy, and only "yourself, or unknown" on a member's, since leadership is not
  // announced to the wider org. Either way the question is the same one field.
  function is_leader(id: string): boolean {
    return !!org && org.leader === id;
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

  // Clearing (new_leader null) when the member is already leader; promoting them otherwise. The
  // engine rejects this on an org with no leadership at all — UX, not security — so the flash carries
  // the reason rather than this hiding the control.
  function toggle_leader(player_id: string) {
    if (!org_key) return;
    send(
      {
        ChangeOrgLeader: {
          org_id: slotKeyFromString(org_key),
          new_leader: is_leader(player_id) ? null : slotKeyFromString(player_id),
        },
      },
      "Leader updated.",
    );
  }

  // No blacklist control here on purpose: it is a low-level primitive that other machinery drives
  // (silent prosecution). An org cannot bar or unbar anyone, so offering it on the org's own
  // roster would teach the wrong model of what it is.
</script>

{#if visible && org && org_key}
  {@const accent = orgColorVar(org.name)}

  <Button variant="ghost" size="sm" class="w-full justify-start" onclick={() => (open = true)}>
    Organization
  </Button>

  {#snippet head()}
    <span class="flex items-center gap-2.5">
      <span class="h-3 w-3 shrink-0" style="background-color:{accent}"></span>
      <span class="text-lg font-semibold" style="color:{accent}">{orgDisplayName(org.name)}</span>
      <span class="text-[0.7rem] font-normal uppercase tracking-widest text-ink-dim">Organization</span>
    </span>
  {/snippet}

  <Dialog bind:open header={head} class="max-w-lg">
    <div class="flex flex-col divide-y divide-edge">
      <!-- frozen outranks is_member, because is_member reads the global roster and the roster is
           one of the things that stopped updating. A view that has lost the viewport can still be
           listed in an org it was thrown out of. -->
      <div class="pb-4">
        {#if frozen}
          <p class="border-l-2 border-event-alarm bg-event-alarm/10 px-3 py-2 text-xs text-event-alarm">
            This organization no longer reaches you. Everything here is what you last saw.
          </p>
        {:else if is_member}
          <AbilityMenu orgId={org_key} />
        {:else}
          <p class="border-l-2 border-edge bg-raised px-3 py-2 text-xs text-ink-dim">
            You are no longer in this organization. Everything here is what you last saw.
          </p>
        {/if}
      </div>

      <div class="flex flex-col gap-2 py-4">
        <div class="flex items-baseline justify-between gap-2">
          <span class="text-[0.7rem] font-semibold uppercase tracking-widest text-neutral-400">Members</span>
          <span class="text-[0.7rem] tabular-nums text-ink-dim">
            <span class="text-neutral-300">{org_members.length}</span> total
            ·
            <span style="color:{accent}">{effective_count}</span> counted
          </span>
        </div>

        {#if org_members.length === 0}
          <p class="py-1 text-xs text-ink-dim">No members.</p>
        {:else}
          <div class="border border-edge">
            {#each org_members as m (m.id)}
              <div class="flex items-center gap-2 border-b border-edge px-3 py-2 last:border-b-0">
                <span class="min-w-0 flex-1 truncate text-sm {m.effective ? 'text-ink' : 'text-neutral-500'}">
                  {m.name}
                </span>
                {#if !m.effective}
                  <span
                    class="shrink-0 border border-edge px-1.5 py-0.5 text-[0.6rem] font-medium uppercase tracking-wide text-neutral-400"
                    title="Not present, so not counted toward this org's ability member requirements. Still a full member."
                  >
                    not counted
                  </span>
                {/if}
                {#if is_leader(m.id)}
                  <span
                    class="shrink-0 border px-1.5 py-0.5 text-[0.6rem] font-semibold uppercase tracking-wide"
                    style="color:{accent};border-color:{accent}"
                    title="The org's leader. Only they and the host are told who this is."
                  >
                    Leader
                  </span>
                {/if}
                {#if is_og(m.id)}
                  <span
                    class="shrink-0 border px-1.5 py-0.5 text-[0.6rem] font-semibold uppercase tracking-wide"
                    style="color:{accent};border-color:{accent}"
                    title="An original member. Only they and the host know this."
                  >
                    OG
                  </span>
                {/if}
                {#if ui.viewer === "Admin"}
                  <button
                    class="shrink-0 border border-edge px-2 py-0.5 text-[0.65rem] uppercase tracking-wide text-neutral-400 hover:border-neutral-500 hover:text-ink"
                    onclick={() => toggle_leader(m.id)}
                    title="Toggle leader"
                  >
                    leader
                  </button>
                  <button
                    class="shrink-0 border border-edge px-2 py-0.5 text-[0.65rem] uppercase tracking-wide text-neutral-400 hover:border-neutral-500 hover:text-ink"
                    onclick={() => toggle_og(m.id)}
                    title="Toggle OG status"
                  >
                    og
                  </button>
                  <button
                    class="shrink-0 border border-edge px-2 py-0.5 text-[0.65rem] uppercase tracking-wide text-danger hover:border-danger hover:bg-danger/10"
                    onclick={() => remove_member(m.id)}
                    title="Remove from org"
                  >
                    remove
                  </button>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>

      {#if ui.viewer === "Admin"}
        <div class="flex flex-col gap-2 pt-4">
          <div class="flex items-center justify-between gap-2">
            <span class="text-[0.7rem] font-semibold uppercase tracking-widest text-neutral-400">Add member</span>
            <div class="flex gap-3 text-xs text-neutral-300">
              <label class="flex items-center gap-1.5">
                <input type="checkbox" bind:checked={add_leader} /> leader
              </label>
              <label class="flex items-center gap-1.5">
                <input type="checkbox" bind:checked={add_og} /> og
              </label>
            </div>
          </div>

          {#if candidates.length === 0}
            <p class="py-1 text-xs text-ink-dim">No one to add.</p>
          {:else}
            <div class="border border-edge">
              {#each candidates as [id] (id)}
                <button
                  class="flex w-full items-center justify-between border-b border-edge px-3 py-2 text-sm text-ink last:border-b-0 hover:bg-raised"
                  onclick={() => add_member(id)}
                >
                  <span class="truncate"><Name {id} {view} menu={false} /></span>
                  <span class="shrink-0 text-[0.65rem] uppercase tracking-wide text-ink-dim">add</span>
                </button>
              {/each}
            </div>
          {/if}
          <FlashDisplay {flash} />
        </div>
      {/if}
    </div>
  </Dialog>
{/if}
