<script lang="ts">
  import { getContext } from "svelte";
  import type { Action, ActionActor } from "../../bindings";
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

  const shown = $derived(
    entries.filter((e) => !hidden.has(actionName(e.action.payload))),
  );

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

  // ---- arguments ----
  //
  // An action payload variant rides a struct (e.g. `Kill: Kill`), so beyond the variant name there
  // is a row per field. The timeline is host tooling, so these format locally; actor keys resolve to
  // player names, Options unwrap, and anything more exotic falls back to a compact read.

  function prettyArg(k: string): string {
    const s = k.replace(/_id$/, "").replace(/_/g, " ");
    return s.charAt(0).toUpperCase() + s.slice(1);
  }

  function isSlotKey(v: unknown): v is { idx: number; version: number } {
    return (
      !!v &&
      typeof v === "object" &&
      typeof (v as { idx: unknown }).idx === "number" &&
      typeof (v as { version: unknown }).version === "number"
    );
  }

  // Action payload fields whose slot key is a PLAYER, resolving to a name in view.players. Every
  // other slot key (channel_id, bug_id, lounge_id, notebook_id, poll_id, prosecution_id, a bare
  // `id`, ...) is a different object and must NOT be shown as a player name.
  const PLAYER_KEY_FIELDS = new Set([
    "target",
    "target_id",
    "actor_id",
    "player_id",
    "creator_id",
    "contactor_id",
    "contacted_id",
    "performer",
    "sacrifice",
    "name_target",
    "kidnapper",
    "victim_id",
    "accuser_id",
    "user",
    "prosecutor",
    "defendant",
  ]);

  function fmtArg(key: string, v: unknown): string {
    if (v === null || v === undefined) return "—";
    if (isSlotKey(v)) {
      const k = slotKeyToString(v);
      return PLAYER_KEY_FIELDS.has(key) ? playerLabel(k, view.players) : k;
    }
    if (typeof v === "string") return v;
    if (typeof v === "boolean") return String(v);
    if (typeof v === "number") return isFinite(v) ? String(v) : `${v}`;
    if (Array.isArray(v)) {
      const s = v.map((x) => fmtArg(key, x));
      return s.length > 3
        ? `${s.slice(0, 3).join(", ")} +${s.length - 3}`
        : s.join(", ");
    }
    if (typeof v === "object") {
      const obj = v as Record<string, unknown>;
      if ("Some" in obj) return fmtArg(key, obj.Some);
      if ("None" in obj) return "—";
      const entries = Object.entries(obj);
      if (entries.length === 0) return "∅";
      return entries
        .map(([k, x]) => `${prettyArg(k)}: ${fmtArg(k, x)}`)
        .join(", ");
    }
    return String(v);
  }

  function payloadArgs(a: Action): { key: string; value: string }[] {
    const name = Object.keys(a)[0];
    const data = (a as unknown as Record<string, unknown>)[name];
    if (!data || typeof data !== "object") return [];
    return Object.entries(data as Record<string, unknown>).map(([k, v]) => ({
      key: prettyArg(k),
      value: fmtArg(k, v),
    }));
  }

  function actorText(actor: ActionActor): string {
    if (actor === "Admin") return "Admin";
    if (actor === "System") return "System";
    if ("Player" in actor)
      return playerLabel(slotKeyToString(actor.Player), view.players);
    const o = actor.Organization;
    return `Org ${o.org_id}`;
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

<Button variant="ghost" size="sm" onclick={() => (open = true)}>Timeline</Button
>

<Dialog
  {open}
  onOpenChange={(v) => (open = v)}
  title="Action Timeline"
  width="42rem"
>
  <p class="text-sm text-ink-dim">
    Every action requested by a connection, and how it came out — newest at the
    bottom, like a chat. Server-initiated work (ticks, time skips) is not shown.
  </p>

  <div class="flex items-center gap-2">
    <Button
      variant="ghost"
      size="sm"
      onclick={() => (filter_open = !filter_open)}
    >
      {filter_open
        ? "Close filter"
        : `Filter${hidden_count ? ` (${hidden_count})` : ""}`}
    </Button>
    {#if hidden_count > 0}
      <Button variant="ghost" size="sm" onclick={() => (hidden = new Set())}
        >Show all</Button
      >
    {/if}
  </div>

  {#if filter_open}
    <div
      class="grid grid-cols-2 gap-x-3 gap-y-1.5 max-h-56 overflow-y-auto rounded border border-edge p-2"
    >
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
    <ol
      bind:this={list}
      class="max-h-[30rem] overflow-y-auto divide-y divide-edge"
    >
      {#each shown as entry (entry.id)}
        <li class="flex items-start gap-3 py-1.5 text-[0.9375rem]">
          <span
            class="shrink-0 font-mono text-[0.8125rem] text-ink-dim tabular-nums w-16 pt-0.5"
          >
            {timeText(entry.time)}
          </span>
          <span class="min-w-0 shrink-0 truncate text-ink-dim w-40 pt-0.5"
            >{actorText(entry.action.actor)}</span
          >
          <div class="min-w-0 flex-1">
            <div class="font-medium">{actionName(entry.action.payload)}</div>
            {#each payloadArgs(entry.action.payload) as r (r.key)}
              <div class="text-xs text-neutral-500">
                <span class="text-neutral-600">{r.key}:</span>
                {r.value}
              </div>
            {/each}
          </div>
        </li>
      {/each}
    </ol>
  {/if}
</Dialog>
