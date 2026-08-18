<script lang="ts">
  // What a client shows when no game is joined. Two jobs, kept apart because they use different
  // credentials: joining takes a per-game key, which is what an ordinary player has, while
  // administration takes a platform key that gates creating and destroying games and has nothing
  // to do with any game's own keys. Administration renders only where the host has a platform.
  import { getContext } from "svelte";
  import { CLIENT_KEY, ClientState } from "../../client.svelte";
  import { STRINGS } from "../../config/strings";

  const client = getContext<ClientState>(CLIENT_KEY);

  // One splash, picked on load.
  const splashes = STRINGS.platform_splashes;
  const splash = $state(splashes[Math.floor(Math.random() * splashes.length)]);

  type RosterEntry = { game_id: number; connections: number };

  // The platform directory, refreshed on an interval while this screen is up.
  let roster = $state<RosterEntry[]>([]);

  // Platform administration. The key "sits here" always, since ending games needs it.
  let platformKey = $state("");
  let adminBusy = $state(false);
  let adminError = $state("");
  // Shown once, here: the server never stores these in a retrievable form, so losing this admin
  // key means losing the game.
  let created = $state<{ game_id: number; admin_key: string } | null>(null);

  // The direct-connect overlay, brought up on demand.
  let connecting = $state(false);
  let gameId = $state("");
  let key = $state("");
  let keyInput = $state<HTMLInputElement | null>(null);

  // Poll the roster while we're on this screen. The App unmounts Platform the moment a join
  // starts, so this interval never outlives the platform view.
  $effect(() => {
    if (!client.host.platform) return;
    let stopped = false;
    const poll = async () => {
      const entries = await client.host.platform!.roster();
      if (!stopped) roster = entries;
    };
    poll();
    const id = setInterval(poll, 15000);
    return () => {
      stopped = true;
      clearInterval(id);
    };
  });

  // Refetch the roster immediately, e.g. right after creating a game so it shows up at once.
  async function refreshRoster() {
    if (!client.host.platform) return;
    try {
      roster = await client.host.platform.roster();
    } catch {
      // Transient; keep the last known roster rather than flash an error on every drop.
    }
  }

  const implicit = client.host.implicitGame !== undefined;
  const canJoin = $derived(gameId.trim() !== "" && key.trim() !== "");

  function openConnect(id?: number, keyValue = "") {
    gameId = id === undefined ? "" : String(id);
    key = keyValue;
    connecting = true;
    // Focus the key on the next frame, once the modal is mounted.
    queueMicrotask(() => keyInput?.focus());
  }

  function closeConnect() {
    connecting = false;
  }

  function join(event: SubmitEvent) {
    event.preventDefault();
    const id = Number(gameId);
    if (!Number.isInteger(id) || id < 0 || key.trim() === "") return;
    void client.join({ gameId: id, key: key.trim() });
  }

  async function createGame() {
    if (!client.host.platform) return;
    adminBusy = true;
    adminError = "";
    created = null;
    try {
      created = await client.host.platform.createGame(platformKey.trim());
      // Bring up the connect overlay prefilled with both the id and the minted admin key so the
      // host can immediately enter its own game, and refresh the roster so the new game shows up
      // at once.
      await refreshRoster();
      openConnect(created.game_id, created.admin_key);
    } catch (e) {
      adminError = e instanceof Error ? e.message : String(e);
    } finally {
      adminBusy = false;
    }
  }

  async function endGame(id: number) {
    if (!client.host.platform) return;
    adminBusy = true;
    adminError = "";
    try {
      await client.host.platform.endGame(id, platformKey.trim());
      // Refetch so the ended game drops off the list at once.
      await refreshRoster();
    } catch (e) {
      adminError = e instanceof Error ? e.message : String(e);
    } finally {
      adminBusy = false;
    }
  }
</script>

<div class="flex h-full w-full items-center overflow-y-auto p-8">
  <div class="mx-auto flex w-full max-w-3xl flex-col gap-4">
    <header class="space-y-1">
      <h1 class="text-2xl font-semibold tracking-tight">requiem-dn</h1>
      <p class="text-sm text-neutral-500">{splash}</p>
    </header>

    {#if client.phase.status === "failed"}
      <p
        class="rounded border border-red-900 bg-red-950/50 px-3 py-2 text-sm text-red-200"
      >
        {client.phase.reason}
      </p>
    {/if}

    {#if implicit}
      <!-- No credential can select anything here, so asking for one would be a form with no
           correct answer. Reachable only after leaving the game, since startup joins. -->
      <button
        class="w-full rounded bg-neutral-100 px-3 py-2 text-sm font-medium text-neutral-950"
        onclick={() => client.start()}
      >
        Open engine
      </button>
    {:else}
      <!-- The platform key is always visible: ending games needs it, creating one does too. It is
           not the join key. -->
      {#if client.canAdminister}
        <div class="flex items-end gap-3">
          <label class="block flex-1 space-y-1">
            <span class="text-xs uppercase tracking-wide text-neutral-500">
              Platform key
            </span>
            <input
              class="w-full rounded border border-neutral-800 bg-neutral-900 px-3 py-2 font-mono text-xs"
              bind:value={platformKey}
              autocomplete="off"
            />
          </label>
          <button
            class="h-9 shrink-0 rounded border border-neutral-700 px-3 text-sm disabled:opacity-50"
            disabled={adminBusy || platformKey.trim() === ""}
            onclick={createGame}
          >
            Create game
          </button>
        </div>
      {/if}

      <!-- The games directory: the bulk of the screen. Bounded so a long roster scrolls in place
           instead of stretching the whole screen. -->
      <section class="flex h-[70vh] min-h-0 flex-col rounded border border-neutral-800 bg-neutral-900/40">
        <div class="flex items-center justify-between border-b border-neutral-800 px-4 py-2">
          <span class="text-xs uppercase tracking-wide text-neutral-500">
            Active games
          </span>
          <div class="flex items-center gap-3">
            <span class="text-xs text-neutral-600">{roster.length} live</span>
            <button
              class="rounded border border-neutral-700 px-2 py-1 text-xs"
              onclick={() => openConnect()}
            >
              Direct connect
            </button>
          </div>
        </div>

        {#if roster.length === 0}
          <div class="flex flex-1 items-center justify-center p-6 text-sm text-neutral-500">
            No games running.
          </div>
        {:else}
          <ul class="min-h-0 flex-1 divide-y divide-neutral-800 overflow-y-auto">
            {#each roster as entry (entry.game_id)}
              <li class="flex items-center gap-4 px-4 py-3">
                <div class="min-w-0 flex-1">
                  <p class="text-base font-medium text-neutral-100">
                    Game {entry.game_id}
                  </p>
                  <p class="text-xs text-neutral-500">
                    {entry.connections} connected
                  </p>
                </div>
                {#if client.canAdminister}
                  <button
                    class="rounded border border-red-900/70 px-3 py-1.5 text-xs text-red-200 disabled:opacity-50"
                    disabled={adminBusy || platformKey.trim() === ""}
                    onclick={() => endGame(entry.game_id)}
                  >
                    END
                  </button>
                {/if}
                <button
                  class="rounded bg-neutral-100 px-3 py-1.5 text-xs font-medium text-neutral-950"
                  onclick={() => openConnect(entry.game_id)}
                >
                  JOIN
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </section>

      {#if created}
        <div
          class="space-y-1 rounded border border-neutral-800 bg-neutral-900 p-2 text-xs"
        >
          <p class="text-neutral-400">
            Game {created.game_id} created. This admin key is shown once — save it now.
          </p>
          <p class="break-all font-mono text-neutral-200">{created.admin_key}</p>
        </div>
      {/if}

      {#if adminError}
        <p class="text-xs text-red-300">{adminError}</p>
      {/if}
    {/if}

    {#if client.host.canQuit}
      <button
        class="text-xs text-neutral-500 underline"
        onclick={() => client.host.quit()}
      >
        Quit
      </button>
    {/if}
  </div>
</div>

{#if connecting}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-8"
    onclick={closeConnect}
  >
    <form
      class="w-full max-w-md space-y-3 rounded border border-neutral-700 bg-neutral-950 p-4"
      onclick={(e) => e.stopPropagation()}
      onsubmit={join}
    >
      <div class="flex items-center justify-between">
        <h2 class="text-sm font-medium text-neutral-100">Direct connect</h2>
        <button
          type="button"
          class="text-xs text-neutral-500 underline"
          onclick={closeConnect}
        >
          Cancel
        </button>
      </div>

      <label class="block space-y-1">
        <span class="text-xs uppercase tracking-wide text-neutral-500">Game</span>
        <input
          class="w-full rounded border border-neutral-800 bg-neutral-900 px-3 py-2 text-sm"
          bind:value={gameId}
          inputmode="numeric"
          placeholder="0"
        />
      </label>
      <label class="block space-y-1">
        <span class="text-xs uppercase tracking-wide text-neutral-500">Key</span>
        <input
          bind:this={keyInput}
          class="w-full rounded border border-neutral-800 bg-neutral-900 px-3 py-2 font-mono text-xs"
          bind:value={key}
          placeholder="paste your key"
          autocomplete="off"
        />
      </label>
      <button
        class="w-full rounded bg-neutral-100 px-3 py-2 text-sm font-medium text-neutral-950 disabled:opacity-50"
        disabled={!canJoin}
        type="submit"
      >
        Join
      </button>
    </form>
  </div>
{/if}