<script lang="ts">
  import { setContext, untrack } from "svelte";
  import { CLIENT_KEY, ClientState } from "../client.svelte";
  import GameScreen from "./game/GameScreen.svelte";
  import Platform from "./platform/Platform.svelte";

  // The host injects the client rather than amane building one: this is the whole seam. The host
  // owns the connection lifecycle end to end, which is what lets its transport report a dropped
  // socket back into `ClientState.dropped` — it cannot do that for a ClientState constructed out
  // of its reach.
  const { client }: { client: ClientState } = $props();

  // Built once by the entry point and never swapped, so the untrack is deliberate rather than an
  // accidentally-captured initial value. Its FIELDS stay reactive below.
  const owned = untrack(() => client);
  setContext(CLIENT_KEY, owned);
  owned.start();
</script>

<div class="flex flex-col h-screen bg-neutral-950 text-white">
  {#if !client.session && client.phase.status === "joining"}
    <!-- On a host that joins automatically, the join screen would otherwise flash an unfillable
         form on every startup. -->
    <div class="flex flex-1 items-center justify-center text-sm text-neutral-500">
      Connecting…
    </div>
  {:else if client.session}
    <!-- Keyed so joining a different game rebuilds the whole tree: every game context below is
         per-session, and reusing them across games would leak one game's selections into another. -->
    {#key client.session}
      <GameScreen session={client.session} />
    {/key}
  {:else}
    <Platform />
  {/if}
</div>
