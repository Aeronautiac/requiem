<script lang="ts">
  import { execErrorText } from "../../game/helpers.svelte";
  import { getContext } from "svelte";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import Button from "../kit/Button.svelte";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";
  import { now } from "../../time.svelte.ts";
  import { slide } from "svelte/transition";
  import AddPlayers from "./AddPlayers.svelte";
  import KeyManager from "./KeyManager.svelte";
  import GoToTime from "./GoToTime.svelte";
  import TimelineViewer from "./TimelineViewer.svelte";

  const session = getContext<SessionState>(SESSION_KEY);
  const flash = new Flash();

  // The host's tools stay folded behind one toggle so they don't crowd the bar. Opening slides the
  // row out to the side rather than dropping a menu down — there is no room below the bar anyway.
  let open = $state(false);
</script>

<div class="flex items-center gap-2">
  <Button variant="ghost" size="sm" onclick={() => (open = !open)}>
    Admin <span class="text-[0.7rem] leading-none">{open ? "◂" : "▸"}</span>
  </Button>

  {#if open}
    <div
      class="flex items-center gap-2 overflow-hidden"
      transition:slide={{ axis: "x", duration: 150 }}
    >
      <AddPlayers />
      <KeyManager />
      <GoToTime {flash} />
      <TimelineViewer />

      <!-- Rejected once the game is running, so this is safe to leave in place rather than gate on a
           phase the client is not told. -->
      <Button
        variant="ghost"
        size="sm"
        onclick={async () => {
          const reply = await session.submit_action({
            actor: "Admin",
            timestamp: now(),
            payload: { StartGame: {} },
          });
          if (!reply.ok) flash.set_error(execErrorText(reply.error));
          else flash.set_success("Day 1. Abilities and notebooks are live.");
        }}>Start Game</Button
      >

      <Button
        variant="danger"
        size="sm"
        onclick={async () => {
          const reply = await session.submit_action({
            actor: "Admin",
            timestamp: now(),
            payload: { Crash: {} },
          });
          // A crash comes back as the "engine has crashed" string; the runtime is respawned and
          // resaturated behind the scenes, so surface it to make the crash visible at all.
          if (!reply.ok) flash.set_error(execErrorText(reply.error));
          else flash.set_success("No crash?");
        }}>Crash</Button
      >

      <Button
        variant="ghost"
        size="sm"
        onclick={async () => {
          const reply = await session.submit_action({
            actor: "Admin",
            timestamp: now(),
            payload: { NextIteration: {} },
          });
          if (!reply.ok) flash.set_error(execErrorText(reply.error));
          else flash.set_success("Day progressed.");
        }}>Next Day</Button
      >

      <FlashDisplay {flash} />
    </div>
  {/if}
</div>
