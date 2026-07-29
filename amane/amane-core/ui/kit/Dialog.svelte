<script lang="ts">
  // `showModal()` is what gives us the focus trap, Esc to close, inertness of the page behind and
  // ::backdrop — the four things a hand-rolled overlay gets wrong, and the reason this needs no
  // library. There is deliberately no trigger prop: a dialog is opened by setting `open`.
  import type { Snippet } from "svelte";

  let {
    open = $bindable(false),
    title,
    header,
    class: klass = "",
    onOpenChange,
    children,
  }: {
    open?: boolean;
    // The ordinary case. Use `header` instead when the heading needs its own markup.
    title?: string;
    header?: Snippet;
    class?: string;
    onOpenChange?: (open: boolean) => void;
    children: Snippet;
  } = $props();

  let el = $state<HTMLDialogElement>();

  // Drive the element from `open`, guarded both ways: calling showModal() on an already-open
  // dialog throws, and close() on a closed one fires a spurious close event.
  $effect(() => {
    if (!el) return;
    if (open && !el.open) el.showModal();
    else if (!open && el.open) el.close();
  });

  // The element closes itself on Esc and on backdrop dismissal, so `open` is synced back from the
  // event rather than assumed.
  function closed() {
    if (!open) return;
    open = false;
    onOpenChange?.(false);
  }

  // The dialog box is a child of <dialog>, so a click landing on the element ITSELF is a click on
  // the backdrop.
  function backdrop(event: MouseEvent) {
    if (event.target === el) el?.close();
  }
</script>

<dialog
  bind:this={el}
  onclose={closed}
  onclick={backdrop}
  class="m-auto w-full max-w-md rounded-lg border border-edge bg-panel p-0 text-ink
         backdrop:bg-black/60 {klass}"
>
  <div class="flex flex-col gap-3 p-5">
    {#if header}
      <div class="text-base font-semibold">{@render header()}</div>
    {:else if title}
      <h2 class="text-base font-semibold">{title}</h2>
    {/if}
    {@render children()}
  </div>
</dialog>
