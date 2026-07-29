<script lang="ts">
  // Minting the keys that let anyone else into the game.
  //
  // Creating a game mints exactly one key — the admin's — so every other participant's credential
  // has to come from here. A key is not a person and not a connection: it is a privilege set, and
  // "associating it with an actor" means naming the slots its holder may play as.
  //
  // A minted key is the one fact a response carries that arrives nowhere else. It is a secret for a
  // single holder, so it is deliberately NOT a command — broadcasting it would hand it to everyone
  // entitled to the stream. The server keeps no retrievable copy either, which is why the list
  // below exists: this dialog is the only place the key is ever shown, and it is gone when the
  // session ends.
  import { getContext } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { GAME_STATE_KEY, type GameState } from "../../game/state.svelte";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import { execErrorText, playerLabel } from "../../game/helpers.svelte";
  import { slotKeyFromString } from "../../bindings";
  import type { ActorScope, Capability } from "../../bindings";
  import Button from "../kit/Button.svelte";
  import Dialog from "../kit/Dialog.svelte";
  import Select from "../kit/Select.svelte";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const session = getContext<SessionState>(SESSION_KEY);
  const flash = new Flash();

  // What a key may do beyond acting as its actors. Supervise never appears without Administer
  // because a key holding it alone is inert — every control is gated on Administer first, and
  // Supervise only widens which OTHER keys an administrator may touch.
  type Grant = "player" | "admin" | "supervisor";

  const GRANTS: { value: Grant; label: string }[] = [
    { value: "player", label: "Player — no administration" },
    { value: "admin", label: "Administrator — manages ordinary keys" },
    { value: "supervisor", label: "Supervisor — manages administrators too" },
  ];

  const CAPABILITIES: Record<Grant, Capability[]> = {
    player: [],
    admin: ["Administer"],
    supervisor: ["Administer", "Supervise"],
  };

  // A key shown once. Held here rather than in any state layer: it is not game state, nothing else
  // may read it, and losing it on leaving the game is correct — the server cannot reissue it either.
  type Minted = { key: string; summary: string };

  let open = $state(false);
  let grant = $state<Grant>("player");
  // `All` covers actors created LATER, which an enumerated set cannot, so it is its own choice
  // rather than "tick everyone".
  let every_actor = $state(false);
  const chosen = new SvelteSet<string>();
  let busy = $state(false);
  let minted = $state<Minted[]>([]);

  // The System view, never the selected one: an admin reading a player's view must still be able to
  // mint for every slot. Orgs are excluded deliberately — no connection may act as an org (the
  // engine instantiates org actions from a member's own), so an org in a key's scope grants nothing.
  const players = $derived.by(() => {
    const all = game.system_view().players;
    return [...all.keys()]
      .map((id) => ({ id, label: playerLabel(id, all) }))
      .sort((a, b) => a.label.localeCompare(b.label));
  });

  function toggle(id: string) {
    if (chosen.has(id)) chosen.delete(id);
    else chosen.add(id);
  }

  function scope(): ActorScope {
    return every_actor ? "All" : { Only: [...chosen].map(slotKeyFromString) };
  }

  // Frozen at mint time. The scope can be changed later by another control, so this states what the
  // key was created for rather than pretending to track what it currently permits.
  function summarize(): string {
    const who = every_actor
      ? "Every actor"
      : chosen.size === 0
        ? "No actors"
        : players
            .filter((p) => chosen.has(p.id))
            .map((p) => p.label)
            .join(", ");
    return `${who} · ${GRANTS.find((g) => g.value === grant)!.label.split(" — ")[0]}`;
  }

  async function create() {
    // A key with neither actors nor administration can do nothing at all, and would only ever be
    // discovered as such by whoever was handed it.
    if (!every_actor && chosen.size === 0 && grant === "player") {
      flash.set_error("Pick at least one player, or grant administration.");
      return;
    }

    busy = true;
    const summary = summarize();
    const reply = await session.submit_control({
      CreateKey: { actors: scope(), capabilities: CAPABILITIES[grant] },
    });
    busy = false;

    if (!reply.ok) {
      flash.set_error(execErrorText(reply.error));
      return;
    }
    // Every other control answers with a bare tag, so a response without the key means the reply
    // stream is not what this client thinks it is — say so rather than showing a blank key.
    if (typeof reply.value === "string" || !("KeyCreated" in reply.value)) {
      flash.set_error("The server answered something other than a key.");
      return;
    }

    minted = [{ key: reply.value.KeyCreated.key, summary }, ...minted];
    chosen.clear();
    every_actor = false;
    grant = "player";
    flash.set_success("Key created — copy it now.");
  }

  async function copy(key: string) {
    // Absent on an insecure origin, so this is a real case rather than defensive padding.
    if (!navigator.clipboard) {
      flash.set_error("No clipboard here — select the key and copy it by hand.");
      return;
    }
    try {
      await navigator.clipboard.writeText(key);
      flash.set_success("Copied.");
    } catch {
      flash.set_error("Could not copy — select the key and copy it by hand.");
    }
  }
</script>

<Button size="sm" onclick={() => (open = true)}>Keys</Button>

<Dialog bind:open title="Keys">
  <p class="text-xs text-ink-dim">
    A key is how someone gets in. Choose the players its holder may act as, hand it over out of
    band, and they join with it.
  </p>

  <label class="flex items-center gap-2 text-sm">
    <input type="checkbox" bind:checked={every_actor} />
    Every actor, including ones added later
  </label>

  <div
    class="max-h-48 overflow-y-auto rounded-md border border-edge"
    class:opacity-50={every_actor}
  >
    {#each players as player (player.id)}
      <label class="flex items-center gap-2 px-2 py-1 text-sm hover:bg-raised">
        <input
          type="checkbox"
          disabled={every_actor}
          checked={chosen.has(player.id)}
          onchange={() => toggle(player.id)}
        />
        {player.label}
      </label>
    {:else}
      <p class="px-2 py-1 text-xs text-ink-dim">No players yet — add some first.</p>
    {/each}
  </div>

  <Select bind:value={grant} options={GRANTS} class="w-full" />

  <Button disabled={busy} onclick={create}>Create key</Button>

  {#if minted.length > 0}
    <div class="flex flex-col gap-2 border-t border-edge pt-3">
      <p class="text-xs text-ink-dim">
        Shown here and nowhere else. The server keeps no readable copy, and closing the game loses
        this list.
      </p>
      {#each minted as entry (entry.key)}
        <div class="rounded-md border border-edge p-2">
          <p class="text-xs text-ink-dim">{entry.summary}</p>
          <div class="flex items-center gap-2">
            <code class="min-w-0 flex-1 break-all font-mono text-xs">{entry.key}</code>
            <Button size="sm" variant="ghost" onclick={() => copy(entry.key)}>Copy</Button>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <FlashDisplay {flash} />
</Dialog>
