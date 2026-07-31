<script lang="ts">
  import { execErrorText } from "../../game/helpers.svelte";
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { orgDisplayName, playerLabel } from "../../game/helpers.svelte";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import type { PollData, PollView } from "../../game/types";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type {
    Action,
    PollOptionIndex,
    PollOptionLabel,
    PollParent,
    PollSubject,
  } from "../../bindings";
  import { slotKeyFromString, slotKeyToString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);

  const session = getContext<SessionState>(SESSION_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  // The one view this component renders. Everything it can show is what this view was delivered.
  const view = $derived(game.view_of(ui.viewer));

  const flash = new Flash();
  let open = $state(true);

  // Players see the ones they were sent a view for; admin sees every poll, and votes in none.
  //
  // Resolved polls are filtered out rather than absent: view.polls keeps them so a viewer gaining
  // the poll's viewport later can replay its history, which makes `outcome` the liveness test.
  //
  // A poll whose viewport this view has left keeps its last tally and is marked stale. It is NOT
  // dropped: the vote it showed is the last thing this viewer heard, and hiding it would be a
  // quieter lie than showing it as current.
  // Every poll this view holds. A view with no poll_views entry for one is watching rather than
  // voting, which is what System does for all of them without being asked whether it is System.
  const polls = $derived.by(() => {
    const out: { id: string; data: PollData; view: PollView | null; frozen: boolean }[] = [];
    for (const [id, data] of view.polls) {
      if (data.outcome) continue;
      out.push({
        id,
        data,
        view: view.poll_views.get(id) ?? null,
        frozen: view.frozen(view.poll_viewport(id)),
      });
    }
    return out;
  });

  // The ability's arguments render separately via subjectArgs, so the voter sees exactly what is
  // being proposed.
  function subjectHeading(subject: PollSubject): string {
    if ("Generic" in subject) return subject.Generic;
    if ("CivilianArrest" in subject) {
      return `Arrest ${playerLabel(slotKeyToString(subject.CivilianArrest), view.players)}`;
    }
    const name = Object.keys(subject.OrgAbility as Record<string, unknown>)[0] ?? "";
    return name.replace(/([a-z])([A-Z])/g, "$1 $2");
  }

  // Shown because the panel lists polls regardless of which channel you are in.
  function parentLabel(parent: PollParent): string {
    if (parent === "World") return "Everyone";
    if ("Org" in parent) {
      const org = view.orgs.get(slotKeyToString(parent.Org));
      return org ? orgDisplayName(org.name) : "Org";
    }
    return view.channels.get(slotKeyToString(parent.Channel))?.name ?? "Channel";
  }

  function optionLabel(label: PollOptionLabel): string {
    return typeof label === "string" ? label : label.Generic;
  }

  // "true_name" -> "True name", "target_id" -> "Target" (the _id suffix is noise here).
  function prettyKey(k: string): string {
    const s = k.replace(/_id$/, "").replace(/_/g, " ");
    return s.charAt(0).toUpperCase() + s.slice(1);
  }

  // Actor keys are the only object-typed args, so they are what resolves to a player name.
  function formatArgValue(v: unknown): string {
    if (typeof v === "boolean") return v ? "yes" : "no";
    if (typeof v === "object" && v !== null) {
      return playerLabel(slotKeyToString(v as never), view.players);
    }
    return String(v);
  }

  // Empty for non-ability subjects; absent args are skipped so optional fields don't show blank.
  function subjectArgs(subject: PollSubject): { label: string; value: string }[] {
    if (!("OrgAbility" in subject)) return [];
    const beh = subject.OrgAbility as Record<string, unknown>;
    const name = Object.keys(beh)[0] ?? "";
    const args = (beh[name] ?? {}) as Record<string, unknown>;
    const out: { label: string; value: string }[] = [];
    for (const [k, v] of Object.entries(args)) {
      if (v === null || v === undefined) continue;
      out.push({ label: prettyKey(k), value: formatArgValue(v) });
    }
    return out;
  }

  async function send(id: string, payload: Action, ok: string) {
    const reply = await session.submit_action({
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload,
    });
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else flash.set_success(ok);
  }

  function vote(id: string, option: PollOptionIndex, label: string) {
    send(id, { AddVote: { poll_id: slotKeyFromString(id), option } }, `Voted ${label}.`);
  }

  function retract(id: string) {
    send(id, { RemoveVote: { poll_id: slotKeyFromString(id) } }, "Vote retracted.");
  }
</script>

<div class="flex flex-col gap-1 border-b border-neutral-800 p-2">
  <button
    class="flex items-center gap-1 px-2 py-1 text-xs font-medium uppercase tracking-wide text-neutral-400 hover:text-neutral-200"
    onclick={() => (open = !open)}
  >
    <span class="text-[0.6rem]">{open ? "▾" : "▸"}</span>
    Polls
    {#if polls.length > 0}
      <span class="ml-1 rounded bg-neutral-800 px-1.5 text-[0.65rem] text-neutral-400">
        {polls.length}
      </span>
    {/if}
  </button>

  {#if open}
    {#if polls.length === 0}
      <p class="px-2 py-1 text-xs text-neutral-600">No active votes</p>
    {:else}
      {#each polls as p (p.id)}
        {@const args = subjectArgs(p.data.subject)}
        <div class="flex flex-col gap-1.5 rounded border border-neutral-800 px-2 py-2">
          <span class="text-sm text-neutral-200">{subjectHeading(p.data.subject)}</span>

          {#if args.length > 0}
            <div class="flex flex-col gap-0.5">
              {#each args as arg (arg.label)}
                <span class="text-[0.7rem] text-neutral-400">
                  <span class="text-neutral-500">{arg.label}:</span>
                  {arg.value}
                </span>
              {/each}
            </div>
          {/if}

          <div class="flex flex-wrap items-center gap-x-2 text-[0.65rem] text-neutral-500">
            <span class="rounded bg-neutral-800 px-1.5 py-0.5 text-neutral-400">
              {parentLabel(p.data.parent)}
            </span>
            {#if p.data.opener}
              <span>started by {view.actor_name(p.data.opener)}</span>
            {/if}
          </div>

          <div class="flex flex-col gap-0.5 text-[0.7rem] text-neutral-500">
            {#each p.data.options as option, i (i)}
              <span class:text-neutral-300={p.view?.own_vote === i}>
                {optionLabel(option.label)}
                {option.weight}
              </span>
            {/each}
            <span>· of {p.data.potential}</span>
          </div>

          {#if p.frozen}
            <span class="text-[0.7rem] italic text-amber-500/70">
              no longer visible to you — last known tally
            </span>
          {:else if p.view === null}
            <span class="text-[0.7rem] italic text-neutral-600">observing</span>
          {:else if !p.view.eligible}
            <span class="text-[0.7rem] italic text-neutral-600">
              you can't vote in this poll
            </span>
          {:else if p.view.own_vote === null}
            <div class="flex flex-wrap gap-1">
              {#each p.data.options as option, i (i)}
                <button
                  class="flex-1 rounded bg-neutral-700/80 px-2 py-1 text-xs font-medium text-white hover:bg-neutral-600"
                  onclick={() => vote(p.id, i, optionLabel(option.label))}
                >
                  {optionLabel(option.label)}
                </button>
              {/each}
            </div>
          {:else}
            <div class="flex items-center justify-between gap-2">
              <span class="text-xs text-neutral-300">
                you voted {optionLabel(p.data.options[p.view.own_vote]?.label ?? {
                  Generic: "?",
                })}
              </span>
              <button
                class="rounded px-1.5 py-0.5 text-xs text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200"
                onclick={() => retract(p.id)}
              >
                retract
              </button>
            </div>
          {/if}
        </div>
      {/each}
    {/if}

    <div class="px-2 pt-1">
      <FlashDisplay {flash} />
    </div>
  {/if}
</div>
