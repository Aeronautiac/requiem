<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game_state.svelte.ts";
  import type { GameState } from "../../game_state.svelte.ts";
  import type { ActionRequest } from "../../bindings";
  import Button from "$lib/components/ui/button/button.svelte";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";
  import { now, add_offset, time_offset } from "../../time.svelte.ts";
  import AddPlayers from "./AddPlayers.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const flash = new Flash();

  async function init() {
    const request: ActionRequest = {
      actor: "Admin", // later enforce who is allowed to do this on the server
      timestamp: now(),
      payload: { InitializeEngine: { seed: 0 } },
    };
    const err = await game.dispatch(request);
    if (err) flash.set_error(`Action Failed: ${err}`);
    else flash.set_success("Successfully initialized Engine");
  }

  // Timestamp base unit is milliseconds (Date.now()); the offset menu has a box per
  // unit (largest first) and sums them.
  const TIME_UNITS: { label: string; ms: number }[] = [
    { label: "day", ms: 86_400_000 },
    { label: "hr", ms: 3_600_000 },
    { label: "min", ms: 60_000 },
    { label: "sec", ms: 1000 },
    { label: "ms", ms: 1 },
  ];
  // number|null because <input type="number"> binds a number (or null when empty).
  const empty_parts = (): Record<string, number | null> =>
    Object.fromEntries(TIME_UNITS.map((u) => [u.label, null]));

  let offset_open = $state(false);
  let offset_parts = $state<Record<string, number | null>>(empty_parts());

  // Render a ms offset as its two largest non-zero units, e.g. "1day 6hr".
  function format_offset(ms: number): string {
    if (ms === 0) return "0";
    const sign = ms < 0 ? "-" : "";
    let n = Math.abs(ms);
    const parts: string[] = [];
    for (const { label, ms: factor } of TIME_UNITS) {
      const v = Math.floor(n / factor);
      if (v > 0) {
        parts.push(`${v}${label}`);
        n -= v * factor;
      }
    }
    return sign + parts.slice(0, 2).join(" ");
  }

  // Sums the per-unit offset boxes and applies them to the central clock, then sends a
  // no-op action so the engine catches up to the (possibly shifted) time. All boxes
  // empty → Update is just a plain tick/catchup.
  async function update() {
    let delta = 0;
    for (const u of TIME_UNITS) {
      const v = offset_parts[u.label];
      if (v != null && Number.isFinite(v)) delta += v * u.ms;
    }
    if (delta !== 0 && !add_offset(delta)) {
      flash.set_error("Offset would rewind before the engine's current time.");
      return;
    }
    offset_parts = empty_parts();
    offset_open = false;

    const request: ActionRequest = {
      actor: "Admin",
      timestamp: now(),
      payload: { Null: {} },
    };
    const err = await game.dispatch(request);
    if (err) flash.set_error(`Action Failed: ${err}`);
    else flash.set_success("Updated");
  }
</script>

<div class="flex items-center gap-2">
  <AddPlayers />

  <Button
    onclick={async () => {
      await init();
    }}>Initialize Engine</Button
  >

  <div class="relative inline-block align-middle">
    <button
      class="h-9 rounded-md border border-neutral-700 bg-neutral-900 px-3 text-sm text-neutral-200 hover:bg-neutral-800"
      onclick={() => (offset_open = !offset_open)}
    >
      Offset{#if time_offset() !== 0}
        <span class="text-neutral-500">· {format_offset(time_offset())}</span>
      {/if}
    </button>

    {#if offset_open}
      <button
        class="fixed inset-0 z-10 cursor-default"
        aria-hidden="true"
        onclick={() => (offset_open = false)}
      ></button>
      <div
        class="absolute bottom-full left-0 z-20 mb-1 flex flex-col gap-1 rounded-md border border-neutral-700 bg-neutral-900 p-2 shadow-lg"
      >
        {#each TIME_UNITS as u (u.label)}
          <label
            class="flex items-center justify-end gap-2 text-xs text-neutral-400"
          >
            <input
              type="number"
              placeholder="0"
              bind:value={offset_parts[u.label]}
              class="h-7 w-20 rounded bg-neutral-800 px-2 text-right text-sm text-neutral-200"
            />
            {u.label}
          </label>
        {/each}
      </div>
    {/if}
  </div>

  <Button
    onclick={async () => {
      await update();
    }}>Update</Button
  >

  <Button
    variant="destructive"
    onclick={async () => {
      const err = await game.dispatch({
        actor: "Admin",
        timestamp: now(),
        payload: { Crash: {} },
      });
      // A crash comes back as the "engine has crashed" string (the runtime is respawned
      // and resaturated behind the scenes); surface it so the crash is actually visible.
      if (err) flash.set_error(err);
      else flash.set_success("No crash?");
    }}>Crash</Button
  >

  <FlashDisplay {flash} />
</div>
