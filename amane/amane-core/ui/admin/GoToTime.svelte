<script lang="ts">
  // The host's time-travel control: jump the game clock to an arbitrary instant, forward or back.
  // This is the game task's own mechanic (GoToTime) — it works entirely on the server's sandboxed
  // clock, so the client just names a target and the response confirms the jump was issued.
  import { getContext } from "svelte";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import type { Flash } from "../../flash.svelte.ts";
  import { formatTime } from "../../lib/utils";
  import Button from "../kit/Button.svelte";
  import Dialog from "../kit/Dialog.svelte";
  import Input from "../kit/Input.svelte";

  let {
    flash,
  }: { flash: Flash } = $props();

  const session = getContext<SessionState>(SESSION_KEY);

  let text = $state("");
  let busy = $state(false);
  let confirming = $state(false);

  // Parse explicit time units in any order: "2h30m", "30m10s", "500ms", "1d2h", "1h:30m" (the
  // punctuation is ignored). A plain number of digits is treated as seconds. Returns game-time ms,
  // or null if the input is not a shape this can read.
  function parse(text: string): number | null {
    const s = text.trim().toLowerCase().replace(/[\s,:]+/g, "");
    if (!s) return null;
    if (/^\d+$/.test(s)) return Number(s) * 1000; // bare number of seconds
    let total = 0;
    let rest = s;
    const re = /(\d+(?:\.\d+)?)(ms|s|m|h|d)/;
    while (re.test(rest)) {
      const m = re.exec(rest)!;
      const val = parseFloat(m[1]);
      const mult =
        m[2] === "ms" ? 1 : m[2] === "s" ? 1000 : m[2] === "m" ? 60000 : m[2] === "h" ? 3600000 : 86400000;
      total += val * mult;
      rest = rest.slice(0, m.index) + rest.slice(m.index + m[0].length);
    }
    // any leftover characters mean malformed units -- reject rather than guess.
    if (rest !== "") return null;
    return total;
  }

  const target_ms = $derived(parse(text));
  const ok = $derived(target_ms !== null);
  const preview = $derived(target_ms === null ? "" : formatTime(target_ms));

  // Confirm rather than jump straight in: a backward jump rewrites the timeline and can cut
  // connections, so it deserves a deliberate second click.
  function request_confirm() {
    if (!ok) return;
    confirming = true;
  }

  async function confirm() {
    if (target_ms === null) return;
    confirming = false;
    busy = true;
    const reply = await session.submit_control({ Meta: { GoToTime: { time: target_ms } } });
    busy = false;
    if (!reply.ok) flash.set_error("Set time was refused.");
    else flash.set_success("Clock moved.");
  }
</script>

<div class="flex items-center gap-1.5 whitespace-nowrap">
  <Input
    type="text"
    placeholder="xxh:xxm:xxs:xxms"
    bind:value={text}
    onkeydown={(e) => {
      if (e.key === "Enter") request_confirm();
    }}
    class="w-32"
  />
  <Button variant="ghost" size="sm" disabled={!ok || busy} onclick={request_confirm}>
    Set time
  </Button>
</div>

<Dialog bind:open={confirming} title="Set game time">
  <div class="text-sm text-neutral-300">
    Move the clock to <span class="font-mono text-neutral-100">{preview}</span>.
    Going backward rewrites the timeline and can disconnect anyone whose key is no longer valid.
  </div>
  <div class="mt-4 flex justify-end gap-2">
    <Button variant="ghost" size="sm" onclick={() => (confirming = false)}>Cancel</Button>
    <Button variant="default" size="sm" disabled={busy} onclick={confirm}>Set time</Button>
  </div>
</Dialog>
