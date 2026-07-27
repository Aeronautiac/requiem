<script lang="ts">
  import { setContext, untrack } from "svelte";
  import { PLATFORM_KEY, PlatformState } from "../platform.svelte";
  import GameScreen from "./game/GameScreen.svelte";
  import Platform from "./platform/Platform.svelte";

  // The host injects the platform rather than amane building one: this is the whole seam. The
  // host owns the connection lifecycle end to end, which is what lets its transport report a
  // dropped socket back into `PlatformState.dropped` — it cannot do that for a PlatformState
  // constructed out of its reach.
  //
  // Note the split is NOT desktop-vs-web. A packaged desktop client talks to yagami over this
  // same websocket protocol; there is nothing browser-specific below this line.
  const { platform }: { platform: PlatformState } = $props();

  // untrack: the platform is built once by the entry point and never swapped, so reading it here
  // is deliberate rather than an accidentally-captured initial value. (Its FIELDS are reactive and
  // the markup below reads them normally; it is the prop itself that is fixed.)
  const owned = untrack(() => platform);
  setContext(PLATFORM_KEY, owned);
  owned.start();
</script>

<div class="flex flex-col h-screen bg-neutral-950 text-white">
  {#if !platform.client && platform.phase.status === "joining"}
    <!-- Kept off the join screen while a connection is in flight: on a host that joins
         automatically this would otherwise flash an unfillable form on every startup. -->
    <div class="flex flex-1 items-center justify-center text-sm text-neutral-500">
      Connecting…
    </div>
  {:else if platform.client}
    <!-- Keyed on the client so joining a different game rebuilds the whole tree: every
         game context below is per-client, and reusing them across games would leak one
         game's selections into another. -->
    {#key platform.client}
      <GameScreen client={platform.client} />
    {/key}
  {:else}
    <Platform />
  {/if}
</div>
