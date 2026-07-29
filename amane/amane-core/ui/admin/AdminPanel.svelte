<script lang="ts">
  import { execErrorText } from "../../game/helpers.svelte";
  import { getContext } from "svelte";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import Button from "../kit/Button.svelte";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";
  import { now } from "../../time.svelte.ts";
  import AddPlayers from "./AddPlayers.svelte";

  const session = getContext<SessionState>(SESSION_KEY);
  const flash = new Flash();
</script>

<div class="flex items-center gap-2">
  <AddPlayers />

  <!-- Rejected once the game is running, so this is safe to leave in place rather than gate on a
       phase the client is not told. -->
  <Button
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

  <FlashDisplay {flash} />
</div>
