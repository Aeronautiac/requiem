<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState, PollData, PollView } from "../../game_state.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { ActionPayload, PollSubject } from "../../bindings";
  import { slotKeyFromString, slotKeyToString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);
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

  // Render a poll's subject. Generic is pre-rendered text; an org-ability vote shows the
  // ability name and, when the behaviour carries a target actor, that player's name.
  function subjectText(subject: PollSubject): string {
    if ("Generic" in subject) return subject.Generic;
    if ("CivilianArrest" in subject) {
      const nm = game.players.get(slotKeyToString(subject.CivilianArrest))?.display_name;
      return nm ? `Arrest ${nm}` : "Civilian arrest";
    }
    const beh = subject.OrgAbility as Record<string, unknown>;
    const name = Object.keys(beh)[0] ?? "";
    const pretty = name.replace(/([a-z])([A-Z])/g, "$1 $2");
    const args = beh[name] as Record<string, unknown> | undefined;
    const target = args && (args.target ?? args.invitee ?? args.defendant);
    if (target && typeof target === "object") {
      const nm = game.players.get(slotKeyToString(target as never))?.display_name;
      if (nm) return `${pretty}: ${nm}`;
    }
    return pretty;
  }

  async function send(id: string, payload: ActionPayload, ok: string) {
    const err = await game.dispatch({
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
        <div class="flex flex-col gap-1.5 rounded border border-neutral-800 px-2 py-2">
          <span class="text-sm text-neutral-200">{subjectText(p.data.subject)}</span>

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
