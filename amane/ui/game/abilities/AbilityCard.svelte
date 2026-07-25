<script lang="ts">
  // Generic top-level display for one ability: name, remaining usages, reset timing,
  // and a Use button. It knows nothing ability-specific — clicking Use hands off to
  // the ability's own configuration UI (see AbilityMenu / the registry).
  import type { AbilityName } from "../../../bindings";
  import { prettyAbility } from "./registry";

  let {
    name,
    successUsages,
    failureUsages,
    resets,
    hasUi,
    onUse,
  }: {
    name: AbilityName;
    successUsages: number;
    failureUsages: number;
    resets: number;
    hasUi: boolean;
    onUse: () => void;
  } = $props();

  // Usage counts are now split by outcome: a pool may only be spent on success,
  // only on failure, or both. The ability is usable as long as some outcome still
  // has charges left.
  const usable = $derived(hasUi && (successUsages > 0 || failureUsages > 0));

  // Collapse to a single number when both outcomes agree (the common case), else
  // break them out so the asymmetry is visible.
  const same = $derived(successUsages === failureUsages);
</script>

<div
  class="flex items-center justify-between gap-3 rounded-lg border border-neutral-800 px-3 py-2"
>
  <div class="flex flex-col">
    <span class="text-sm text-neutral-200">{prettyAbility(name)}</span>
    <span class="text-xs text-neutral-500">
      {#if same}
        {successUsages} use{successUsages === 1 ? "" : "s"} left
      {:else}
        {successUsages} on success · {failureUsages} on failure
      {/if}
      {#if resets > 0}· resets in {resets}{/if}
      {#if !hasUi}· no UI yet{/if}
    </span>
  </div>

  <button
    class="shrink-0 rounded-md bg-neutral-100 px-3 py-1 text-sm font-medium text-neutral-900 hover:bg-white disabled:cursor-not-allowed disabled:bg-neutral-800 disabled:text-neutral-600"
    disabled={!usable}
    onclick={onUse}
  >
    Use
  </button>
</div>
