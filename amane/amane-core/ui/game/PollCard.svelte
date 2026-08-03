<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import {
    channelLabel,
    execErrorText,
    orgDisplayName,
    playerLabel,
  } from "../../game/helpers.svelte";
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

  // One poll, rendered the same way whether it sits inline in its home channel or in the top
  // panel. `variant` only changes width/jump target: inline knows it is already in the channel, so
  // its jump opens the panel; the panel jumps to the home channel.
  let {
    id,
    data,
    pollView,
    frozen,
    variant,
  }: {
    id: string;
    data: PollData;
    pollView: PollView | null;
    frozen: boolean;
    variant: "inline" | "panel";
  } = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);
  const session = getContext<SessionState>(SESSION_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  const view = $derived(game.view_of(ui.viewer));

  const flash = new Flash();

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

  function parentLabel(parent: PollParent): string {
    if (parent === "World") return "Everyone";
    if ("Org" in parent) {
      const org = view.orgs.get(slotKeyToString(parent.Org));
      return org ? orgDisplayName(org.name) : "Org";
    }
    const ch = view.channels.get(slotKeyToString(parent.Channel));
    return ch ? channelLabel(ch.name) : "Channel";
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
  const args = $derived.by(() => {
    const subject = data.subject;
    if (!("OrgAbility" in subject)) return [] as { label: string; value: string }[];
    const beh = subject.OrgAbility as Record<string, unknown>;
    const name = Object.keys(beh)[0] ?? "";
    const raw = (beh[name] ?? {}) as Record<string, unknown>;
    const out: { label: string; value: string }[] = [];
    for (const [k, v] of Object.entries(raw)) {
      if (v === null || v === undefined) continue;
      out.push({ label: prettyKey(k), value: formatArgValue(v) });
    }
    return out;
  });

  // The channel (or news) this poll calls home, derived from its parent scope. World polls live in
  // News, org polls in the org's channel, channel polls in that channel.
  function home_selection(): (() => void) | null {
    const parent = data.parent;
    if (parent === "World") return () => ui.select_news();
    if ("Org" in parent) {
      const ch = view.channel_of_org(slotKeyToString(parent.Org));
      return ch ? () => ui.select_channel(ch) : null;
    }
    return () => ui.select_channel(slotKeyToString(parent.Channel));
  }

  function jump() {
    if (variant === "inline") {
      // Already in the channel — jump means "see it in the full panel with every other poll".
      ui.top_panel = "polls";
    } else {
      home_selection()?.();
    }
  }

  async function send(payload: Action, ok: string) {
    const reply = await session.submit_action({
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload,
    });
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else flash.set_success(ok);
  }

  function vote(option: PollOptionIndex, label: string) {
    send({ AddVote: { poll_id: slotKeyFromString(id), option } }, `Voted ${label}.`);
  }

  function retract() {
    send({ RemoveVote: { poll_id: slotKeyFromString(id) } }, "Vote retracted.");
  }
</script>

<div
  class="flex flex-col gap-2 border border-neutral-700 bg-gradient-to-b from-neutral-800 to-neutral-900 p-3 shadow-md {variant ===
  'panel'
    ? 'w-72 shrink-0'
    : 'w-full'}"
>
  <div class="flex items-start justify-between gap-2">
    <span class="text-sm font-medium text-neutral-100">{subjectHeading(data.subject)}</span>
    <button
      class="shrink-0 text-[0.65rem] uppercase tracking-wide text-neutral-500 hover:text-neutral-200"
      onclick={jump}
    >
      {variant === "inline" ? "open ▸" : "jump ▸"}
    </button>
  </div>

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
    <span class="bg-neutral-950 px-1.5 py-0.5 text-neutral-400">
      {parentLabel(data.parent)}
    </span>
    {#if data.opener}
      <span>started by {view.actor_name(data.opener)}</span>
    {/if}
  </div>

  <div class="flex flex-col gap-0.5 text-[0.7rem] text-neutral-500">
    {#each data.options as option, i (i)}
      <span class:text-neutral-300={pollView?.own_vote === i}>
        {optionLabel(option.label)}
        {option.weight}
      </span>
    {/each}
    <span>· of {data.potential}</span>
  </div>

  {#if frozen}
    <span class="text-[0.7rem] italic text-amber-500/70">
      no longer visible to you — last known tally
    </span>
  {:else if pollView === null}
    <span class="text-[0.7rem] italic text-neutral-600">observing</span>
  {:else if !pollView.eligible}
    <span class="text-[0.7rem] italic text-neutral-600">you can't vote in this poll</span>
  {:else if pollView.own_vote === null}
    <div class="flex flex-wrap gap-1">
      {#each data.options as option, i (i)}
        <button
          class="flex-1 bg-neutral-700/80 px-2 py-1 text-xs font-medium text-white shadow-sm hover:bg-neutral-600"
          onclick={() => vote(i, optionLabel(option.label))}
        >
          {optionLabel(option.label)}
        </button>
      {/each}
    </div>
  {:else}
    <div class="flex items-center justify-between gap-2">
      <span class="text-xs text-neutral-300">
        you voted {optionLabel(data.options[pollView.own_vote]?.label ?? { Generic: "?" })}
      </span>
      <button
        class="px-1.5 py-0.5 text-xs text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200"
        onclick={retract}
      >
        retract
      </button>
    </div>
  {/if}

  <FlashDisplay {flash} />
</div>
