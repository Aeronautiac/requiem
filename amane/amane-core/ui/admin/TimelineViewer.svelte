<script lang="ts">
  import { getContext } from "svelte";
  import type { Action, ActionActor, ActionOutcome } from "../../bindings";
  import { slotKeyToString } from "../../bindings";
  import { playerLabel } from "../../game/helpers.svelte";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import Dialog from "../kit/Dialog.svelte";
  import Button from "../kit/Button.svelte";

  const session = getContext<SessionState>(SESSION_KEY);
  // The admin's action timeline lives on the System view, which is the only one that sees
  // Admin-gated output.
  const view = $derived(session.game.system_view());
  const entries = $derived(view.action_log);

  let open = $state(false);

  // Which action variants to leave OUT of the timeline. A hidden variant is skipped forever, even
  // once the log contains it again — unchecking restores it. Reassigned wholesale (never mutated in
  // place) so Svelte reacts to the toggle unambiguously.
  let hidden = $state<Set<string>>(new Set());
  let filter_open = $state(false);

  function toggle(variant: string) {
    const next = new Set(hidden);
    if (next.has(variant)) next.delete(variant);
    else next.add(variant);
    hidden = next;
  }

  // Every distinct action variant this view has seen, so the filter never offers a name that has
  // not appeared.
  const variants = $derived(
    [...new Set(entries.map((e) => actionName(e.action.payload)))].sort(),
  );
  const hidden_count = $derived(hidden.size);

  const shown = $derived(entries.filter((e) => !hidden.has(actionName(e.action.payload))));

  // Like a chat: new entries land at the bottom, so keep the newest in view rather than forcing the
  // user to scroll down each time one arrives.
  let list = $state<HTMLOListElement>();
  $effect(() => {
    shown.length;
    if (list) list.scrollTop = list.scrollHeight;
  });

  // ---- formatting (deliberately local: this is host tooling, not shared game text) ----

  function actionName(a: Action): string {
    return Object.keys(a)[0];
  }

  function actorText(actor: ActionActor): string {
    if (actor === "Admin") return "Admin";
    if (actor === "System") return "System";
    if ("Player" in actor) return playerLabel(slotKeyToString(actor.Player), view.players);
    const o = actor.Organization;
    return `Org ${o.org_id}`;
  }

  function outcomeText(o: ActionOutcome): string {
    if (o === "Denied") return "denied";
    if (o === "EnginePanic") return "crash";
    return "Ok" in o ? "ok" : "err";
  }

  function outcomeClass(o: ActionOutcome): string {
    if (o === "Denied") return "bg-neutral-800 text-neutral-400";
    if (o === "EnginePanic") return "bg-red-900/60 text-red-200";
    return "Ok" in o ? "bg-emerald-900/60 text-emerald-200" : "bg-amber-900/60 text-amber-200";
  }

  // Game time in milliseconds since the sandbox's zero, rendered as HH:MM:SS.
  function timeText(ms: number): string {
    const s = Math.floor(ms / 1000) % 60;
    const m = Math.floor(ms / 60000) % 60;
    const h = Math.floor(ms / 3600000);
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${pad(h)}:${pad(m)}:${pad(s)}`;
  }
</script>

<Button variant="ghost" size="sm" onclick={() => (open = true)}>Timeline</Button>

<Dialog open={open} onOpenChange={(v) => (open = v)} title="Action Timeline" width="42rem">
  <p class="text-sm text-ink-dim">
    Every action requested by a connection, and how it came out — newest at the bottom, like a chat.
    Server-initiated work (ticks, time skips) is not shown.
  </p>

  <div class="flex items-center gap-2">
    <Button variant="ghost" size="sm" onclick={() => (filter_open = !filter_open)}>
      {filter_open ? "Close filter" : `Filter${hidden_count ? ` (${hidden_count})` : ""}`}
    </Button>
    {#if hidden_count > 0}
      <Button variant="ghost" size="sm" onclick={() => (hidden = new Set())}>Show all</Button>
    {/if}
  </div>

  {#if filter_open}
    <div class="grid grid-cols-2 gap-x-3 gap-y-1.5 max-h-56 overflow-y-auto rounded border border-edge p-2">
      {#if variants.length === 0}
        <p class="text-sm text-ink-dim">No variants yet.</p>
      {:else}
        {#each variants as variant}
          <label class="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={!hidden.has(variant)}
              onchange={() => toggle(variant)}
            />
            <span class="truncate">{variant}</span>
          </label>
        {/each}
      {/if}
    </div>
  {/if}

  {#if shown.length === 0}
    <p class="text-sm text-ink-dim">No actions to show.</p>
  {:else}
    <ol bind:this={list} class="max-h-[30rem] overflow-y-auto divide-y divide-edge">
      {#each shown as entry (entry.time + ":" + actionName(entry.action.payload))}
        <li class="flex items-center gap-3 py-2 text-[0.9375rem]">
          <span class="shrink-0 font-mono text-[0.8125rem] text-ink-dim tabular-nums w-16">
            {timeText(entry.time)}
          </span>
          <span class="min-w-0 shrink-0 truncate text-ink-dim w-40">{actorText(entry.action.actor)}</span>
          <span class="min-w-0 flex-1 truncate">{actionName(entry.action.payload)}</span>
          <span class={`shrink-0 rounded px-1.5 text-[0.7rem] uppercase ${outcomeClass(entry.outcome)}`}>
            {outcomeText(entry.outcome)}
          </span>
        </li>
      {/each}
    </ol>
  {/if}
</Dialog>
