<script lang="ts">
  // What a client shows when no game is joined. Two jobs, kept apart because they use different
  // credentials: joining takes a per-game key, which is what an ordinary player has, while
  // administration takes a platform key that gates creating and destroying games and has nothing
  // to do with any game's own keys. Administration renders only where the host has a platform.
  import { getContext } from "svelte";
  import { CLIENT_KEY, ClientState } from "../../client.svelte";

  const client = getContext<ClientState>(CLIENT_KEY);

  let gameId = $state("");
  let key = $state("");

  // Platform administration
  let platformKey = $state("");
  let adminBusy = $state(false);
  let adminError = $state("");
  // Shown once, here: the server never stores these in a retrievable form, so losing this admin
  // key means losing the game.
  let created = $state<{ game_id: number; admin_key: string } | null>(null);
  let endId = $state("");

  const joining = $derived(client.phase.status === "joining");
  // A host with an implicit game has no credential to ask for.
  const implicit = client.host.implicitGame !== undefined;

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
      // Prefill the join form so the host can immediately enter its own game.
      gameId = String(created.game_id);
      key = created.admin_key;
    } catch (e) {
      adminError = e instanceof Error ? e.message : String(e);
    } finally {
      adminBusy = false;
    }
  }

  async function endGame() {
    if (!client.host.platform) return;
    const id = Number(endId);
    if (!Number.isInteger(id)) return;
    adminBusy = true;
    adminError = "";
    try {
      await client.host.platform.endGame(id, platformKey.trim());
      endId = "";
    } catch (e) {
      adminError = e instanceof Error ? e.message : String(e);
    } finally {
      adminBusy = false;
    }
  }
</script>

<div class="flex flex-1 items-center justify-center overflow-y-auto p-8">
  <div class="w-full max-w-md space-y-8">
    <header class="space-y-1">
      <h1 class="text-2xl font-semibold tracking-tight">requiem</h1>
      <p class="text-sm text-neutral-400">
        {implicit
          ? "This host runs a single engine in-process."
          : "Enter a game id and the key you were given."}
      </p>
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
        class="w-full rounded bg-neutral-100 px-3 py-2 text-sm font-medium text-neutral-950 disabled:opacity-50"
        disabled={joining}
        onclick={() => client.start()}
      >
        {joining ? "Connecting…" : "Open engine"}
      </button>
    {:else}
      <form class="space-y-3" onsubmit={join}>
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
            class="w-full rounded border border-neutral-800 bg-neutral-900 px-3 py-2 font-mono text-xs"
            bind:value={key}
            placeholder="paste your key"
            autocomplete="off"
          />
        </label>
        <button
          class="w-full rounded bg-neutral-100 px-3 py-2 text-sm font-medium text-neutral-950 disabled:opacity-50"
          disabled={joining}
          type="submit"
        >
          {joining ? "Connecting…" : "Join"}
        </button>
      </form>
    {/if}

    {#if client.canAdminister}
      <details class="rounded border border-neutral-800">
        <summary
          class="cursor-pointer px-3 py-2 text-xs uppercase tracking-wide text-neutral-500"
        >
          Platform administration
        </summary>
        <div class="space-y-3 border-t border-neutral-800 p-3">
          <label class="block space-y-1">
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
            class="w-full rounded border border-neutral-700 px-3 py-2 text-sm disabled:opacity-50"
            disabled={adminBusy}
            onclick={createGame}
          >
            Create game
          </button>

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

          <div class="flex gap-2">
            <input
              class="flex-1 rounded border border-neutral-800 bg-neutral-900 px-3 py-2 text-sm"
              bind:value={endId}
              inputmode="numeric"
              placeholder="game id"
            />
            <button
              class="rounded border border-red-900 px-3 py-2 text-sm text-red-200 disabled:opacity-50"
              disabled={adminBusy}
              onclick={endGame}
            >
              End game
            </button>
          </div>

          {#if adminError}
            <p class="text-xs text-red-300">{adminError}</p>
          {/if}
        </div>
      </details>
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
