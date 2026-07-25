<script lang="ts">
  import { setContext, untrack } from "svelte";
  import type { HostContext } from "../lib/protocol";
  import { PLATFORM_KEY, PlatformState } from "../platform.svelte";
  import GameScreen from "./game/GameScreen.svelte";
  import Platform from "./platform/Platform.svelte";

  // The host injects its context rather than amane reaching for one: this is the whole
  // seam. Note the split is NOT desktop-vs-web — a real Tauri client talks to yagami over
  // the websocket protocol exactly like a browser does. Direct IPC is armonia's alone,
  // because armonia is a dev tool hosting an engine in-process with no server in between.
  const { host }: { host: HostContext } = $props();

  // untrack: the host is built once by the entry point and never swapped, so reading it here
  // is deliberate rather than an accidentally-captured initial value.
  const platform = new PlatformState(untrack(() => host));
  setContext(PLATFORM_KEY, platform);
  platform.start();
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
