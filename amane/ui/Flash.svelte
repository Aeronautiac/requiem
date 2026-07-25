<script lang="ts">
  import type { Flash } from "../flash.svelte.ts";
  let { flash }: { flash: Flash } = $props();
</script>

{#if flash.error !== null}
  <p class="text-red-500">{flash.error}</p>
{/if}
{#if flash.success !== null}
  <p class="text-green-500">{flash.success}</p>
{/if}
