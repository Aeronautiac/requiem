<script lang="ts">
  // One collapsible sidebar section — the frame every sidebar "dropdown" (channel categories,
  // player lists, group-chat controls) shares. Controlled: the parent owns `open` and toggles it
  // in `onToggle`, so this works whether that state is a local boolean or a member of a set.
  import type { Snippet } from "svelte";

  let {
    label,
    open,
    onToggle,
    children,
  }: {
    label: string;
    open: boolean;
    onToggle: () => void;
    children: Snippet;
  } = $props();
</script>

<section class="flex flex-col border-y border-neutral-700">
  <button
    class="flex items-center gap-2 border-neutral-700 bg-neutral-800/40 px-3 py-2 text-xs font-medium uppercase tracking-wide text-neutral-300 hover:bg-neutral-800 hover:text-neutral-100 {open
      ? 'border-b'
      : ''}"
    onclick={onToggle}
  >
    <span
      class="inline-block w-3 text-center text-[0.7rem] leading-none transition-transform {open
        ? 'rotate-90'
        : ''}"
    >
      ▸
    </span>
    {label}
  </button>

  {#if open}
    {@render children()}
  {/if}
</section>
