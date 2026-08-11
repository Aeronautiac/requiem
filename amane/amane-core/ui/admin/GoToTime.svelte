<script lang="ts">
  // The host's time-travel control: jump the game clock to an arbitrary instant, forward or back.
  // This is the game task's own mechanic (GoToTime) — it works entirely on the server's sandboxed
  // clock, so the client just names a target and the response confirms the jump was issued.
  import { getContext } from "svelte";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import type { Flash } from "../../flash.svelte.ts";
  import Button from "../kit/Button.svelte";
  import Input from "../kit/Input.svelte";

  let {
    flash,
  }: { flash: Flash } = $props();

  const session = getContext<SessionState>(SESSION_KEY);

  let text = $state("");
  let busy = $state(false);

  // Accept "H:MM", "H:MM:SS", or a plain number of seconds, and turn it into game-time ms — the
  // units the wire carries and the game clock counts from 0.
  function parse(text: string): number | null {
    const s = text.trim();
    if (!s) return null;
    if (s.includes(":")) {
      const parts = s.split(":").map((p) => Number(p));
      if (parts.some((n) => !Number.isFinite(n) || n < 0)) return null;
      if (parts.length === 2) return (parts[0] * 60 + parts[1]) * 1000;
      if (parts.length === 3) return (parts[0] * 3600 + parts[1] * 60 + parts[2]) * 1000;
      return null;
    }
    const n = Number(s);
    if (!Number.isFinite(n) || n < 0) return null;
    return n * 1000;
  }

  const target_ms = $derived(parse(text));
  const ok = $derived(target_ms !== null);

  async function go() {
    if (target_ms === null) return;
    busy = true;
    const reply = await session.submit_control({ GoToTime: { time: target_ms } });
    busy = false;
    if (!reply.ok) flash.set_error("Go to time was refused.");
    else flash.set_success("Clock moved.");
  }
</script>

<div class="flex items-center gap-1.5">
  <Input
    type="text"
    placeholder="11:03:00"
    bind:value={text}
    onkeydown={(e) => {
      if (e.key === "Enter") void go();
    }}
    class="w-24"
  />
  <Button variant="ghost" size="sm" disabled={!ok || busy} onclick={() => void go()}>
    Go to
  </Button>
</div>
