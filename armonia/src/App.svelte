<script lang="ts">
  import { setContext } from "svelte";
  import { createTransport } from "$lib/transport";
  import { ROUTER_KEY } from "$lib/router";
  import AddPlayers from "./AddPlayers.svelte";

  // Instantiate and inject the transport. All child components access it via:
  //   const router = getContext<Router>(ROUTER_KEY);
  // They never import from transport.ts directly — only through this context.
  setContext(ROUTER_KEY, createTransport());
</script>

<div class="flex flex-col h-screen bg-neutral-950 text-white">
  <!-- build your UI here -->
  <AddPlayers />
</div>
