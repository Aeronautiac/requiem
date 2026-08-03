<script lang="ts">
  // Generic top-level display for one ability: name, remaining usages, reset timing,
  // and a Use button. It knows nothing ability-specific — clicking Use hands off to
  // the ability's own configuration UI (see AbilityMenu / the registry).
  import type { AbilityName, OrgAbility } from "../../../bindings";
  import { OrgAbilityPolicyFlag } from "../../../bindings";
  import { abilityDescription, abilityWarning } from "../../../game/helpers.svelte";
  import { prettyAbility } from "./registry";

  let {
    name,
    successUsages,
    failureUsages,
    resets,
    hasUi,
    requirements,
    onUse,
  }: {
    name: AbilityName;
    successUsages: number;
    failureUsages: number;
    resets: number;
    hasUi: boolean;
    // Org abilities only: the static gates on firing this. Undefined for a personal ability.
    requirements?: OrgAbility;
    onUse: () => void;
  } = $props();

  // Usage counts are now split by outcome: a pool may only be spent on success,
  // only on failure, or both. The ability is usable as long as some outcome still
  // has charges left.
  const usable = $derived(hasUi && (successUsages > 0 || failureUsages > 0));

  // Collapse to a single number when both outcomes agree (the common case), else
  // break them out so the asymmetry is visible.
  const same = $derived(successUsages === failureUsages);

  // The gates as short chips. These state what firing needs — they do NOT reflect whether it is
  // met right now (the org can't see members' secret roles or presence), so a use can still be
  // refused; the engine says why when it is. A role shows raw until role display strings land.
  const gates = $derived.by(() => {
    const out: string[] = [];
    if (!requirements) return out;
    if (requirements.require_members > 0) {
      out.push(`${requirements.require_members} members`);
    }
    for (const role of requirements.require_roles) out.push(`needs ${role}`);
    if (requirements.usage_policies & OrgAbilityPolicyFlag.RequireLeader) out.push("leader only");
    if (requirements.usage_policies & OrgAbilityPolicyFlag.RequireVote) out.push("vote");
    return out;
  });

  const description = $derived(abilityDescription(name));
  const warning = $derived(abilityWarning(name));
  let show_desc = $state(false);
</script>

<div
  class="flex flex-col gap-2 border border-neutral-700 bg-gradient-to-b from-neutral-800 to-neutral-900 px-3 py-2 shadow-md"
>
  <div class="flex items-center justify-between gap-3">
    <div class="flex flex-col gap-1">
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
      {#if gates.length > 0}
        <span class="flex flex-wrap gap-1">
          {#each gates as gate (gate)}
            <span
              class="bg-neutral-800 px-1.5 py-px text-[0.65rem] text-neutral-400"
            >
              {gate}
            </span>
          {/each}
        </span>
      {/if}
    </div>

    <div class="flex shrink-0 items-center gap-1.5">
      {#if description || warning}
        <button
          class="h-6 w-6 border text-xs font-medium {show_desc
            ? 'border-neutral-500 bg-neutral-600 text-white'
            : 'border-neutral-600 bg-neutral-700 text-neutral-200 hover:bg-neutral-600 hover:text-white'}"
          aria-label="Toggle description"
          aria-pressed={show_desc}
          title="What this does"
          onclick={() => (show_desc = !show_desc)}
        >
          ?
        </button>
      {/if}
      <button
        class="bg-neutral-100 px-3 py-1 text-sm font-medium text-neutral-900 shadow-sm hover:bg-white disabled:cursor-not-allowed disabled:bg-neutral-800 disabled:text-neutral-600"
        disabled={!usable}
        onclick={onUse}
      >
        Use
      </button>
    </div>
  </div>

  {#if show_desc && (description || warning)}
    <div class="flex flex-col gap-1.5 border-t border-neutral-800 pt-2">
      {#if description}
        <p class="text-xs text-neutral-400">{description}</p>
      {/if}
      {#if warning}
        <p class="text-xs text-danger">{warning}</p>
      {/if}
    </div>
  {/if}
</div>
