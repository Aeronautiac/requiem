<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY, orgDisplayName } from "../../game_state.svelte.ts";
  import { CLIENT_KEY, type ClientState } from "../../client.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState, PollData, PollView } from "../../game_state.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { ActionPayload, PollSubject, PollVisibility } from "../../bindings";
  import { slotKeyFromString, slotKeyToString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);

  const client = getContext<ClientState>(CLIENT_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  const flash = new Flash();
  let open = $state(true);

  // Polls the current viewer can see. Players see the ones they were sent a view for
  // (poll_views); Admin sees every poll (no vote). Shared data comes from game.polls.
  const polls = $derived.by(() => {
    const out: { id: string; data: PollData; view: PollView | null }[] = [];
    if (ui.viewer === "Admin") {
      for (const [id, data] of game.polls) out.push({ id, data, view: null });
      return out;
    }
    const view = game.views.get(ui.viewer);
    for (const [id, pv] of view?.poll_views ?? []) {
      const data = game.polls.get(id);
      if (data) out.push({ id, data, view: pv });
    }
    return out;
  });

  // A poll's headline: Generic is pre-rendered text; CivilianArrest names the target; an
  // org-ability vote is the (prettified) ability name. The ability's arguments render
  // separately via subjectArgs so the voter sees exactly what's being proposed.
  function subjectHeading(subject: PollSubject): string {
    if ("Generic" in subject) return subject.Generic;
    if ("CivilianArrest" in subject) {
      const nm = game.players.get(slotKeyToString(subject.CivilianArrest))?.display_name;
      return nm ? `Arrest ${nm}` : "Civilian arrest";
    }
    const name = Object.keys(subject.OrgAbility as Record<string, unknown>)[0] ?? "";
    return name.replace(/([a-z])([A-Z])/g, "$1 $2");
  }

  // Where the vote is scoped — shown because the panel lists polls regardless of which
  // channel/view you're in. AllPresent is public; Org/Channel resolve to their name.
  function scopeLabel(scope: PollVisibility): string {
    if (scope === "AllPresent") return "Everyone";
    if ("Org" in scope) {
      const org = game.orgs.get(slotKeyToString(scope.Org));
      return org ? orgDisplayName(org.name) : "Org";
    }
    return game.channels.get(slotKeyToString(scope.Channel))?.name ?? "Channel";
  }

  // "true_name" -> "True name", "target_id" -> "Target" (the _id suffix is noise here).
  function prettyKey(k: string): string {
    const s = k.replace(/_id$/, "").replace(/_/g, " ");
    return s.charAt(0).toUpperCase() + s.slice(1);
  }

  // One arg value: actor keys (the only object-typed args) resolve to a player name, booleans
  // read as yes/no, everything else (roles, names, messages) is shown as-is.
  function formatArgValue(v: unknown): string {
    if (typeof v === "boolean") return v ? "yes" : "no";
    if (typeof v === "object" && v !== null) {
      return game.players.get(slotKeyToString(v as never))?.display_name ?? "Unknown";
    }
    return String(v);
  }

  // The argument lines for an org-ability vote (e.g. "Target: Alice", "Performer: Bob"). Empty
  // for non-ability subjects; null/absent args are skipped so optional fields don't show as blanks.
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

  async function send(id: string, payload: ActionPayload, ok: string) {
    const err = await client.dispatch({
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload,
    });
    if (err) flash.set_error(err);
    else flash.set_success(ok);
  }

  function vote(id: string, accept: boolean) {
    send(
      id,
      { AddVote: { poll_id: slotKeyFromString(id), accept } },
      accept ? "Voted yes." : "Voted no.",
    );
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
              {scopeLabel(p.data.scope)}
            </span>
            {#if p.data.opener}
              <span>started by {p.data.opener}</span>
            {/if}
          </div>

          <div class="flex gap-2 text-[0.7rem] text-neutral-500">
            <span class="text-emerald-400/80">yes {p.data.accept}</span>
            <span class="text-red-400/80">no {p.data.reject}</span>
            <span>· of {p.data.potential}</span>
          </div>

          {#if p.view === null}
            <span class="text-[0.7rem] italic text-neutral-600">observing</span>
          {:else if !p.view.eligible}
            <span class="text-[0.7rem] italic text-neutral-600">
              you can't vote in this poll
            </span>
          {:else if p.view.own_vote === null}
            <div class="flex gap-1">
              <button
                class="flex-1 rounded bg-emerald-700/80 px-2 py-1 text-xs font-medium text-white hover:bg-emerald-600"
                onclick={() => vote(p.id, true)}
              >
                Yes
              </button>
              <button
                class="flex-1 rounded bg-red-700/80 px-2 py-1 text-xs font-medium text-white hover:bg-red-600"
                onclick={() => vote(p.id, false)}
              >
                No
              </button>
            </div>
          {:else}
            <div class="flex items-center justify-between gap-2">
              <span class="text-xs text-neutral-300">
                you voted {p.view.own_vote ? "Yes" : "No"}
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
