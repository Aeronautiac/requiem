<script lang="ts">
  // Pure presentation; the parent resolves the text and picks the colour per event type.
  //
  // `color` is any CSS colour string. The translucency is applied here so callers hand in a solid
  // colour and don't have to think about Tailwind's class purging — dynamic colours cannot be
  // Tailwind classes.
  import type { Snippet } from "svelte";
  import type { GameView } from "../../game/view.svelte";
  import TimeStamp from "./TimeStamp.svelte";

  interface Props {
    color: string;
    description: string;
    // Plain text is the common case. Pass `children` instead when the body needs markup — an
    // embedded ping chip, say — and it renders in the same styled slot.
    content?: string;
    children?: Snippet;
    // The game time the announcement appeared, shown in the header (with real wall time on hover).
    timestamp?: number;
    view?: GameView;
  }
  let { color, description, content, children, timestamp, view }: Props = $props();
</script>

<div class="px-3 py-0.5">
  <div
    class="relative overflow-hidden border-l-2 px-2.5 py-1.5"
    style="border-color: {color}"
  >
    <div
      class="pointer-events-none absolute inset-0"
      style="background-color: {color}; opacity: 0.12"
    ></div>

    <div class="relative">
      <div class="flex items-baseline justify-between gap-2">
        <div
          class="text-[0.8rem] font-semibold uppercase tracking-wide"
          style="color: {color}"
        >
          {description}
        </div>
        {#if timestamp !== undefined}
          <TimeStamp {timestamp} {view} />
        {/if}
      </div>
      <!-- pre-wrap only for the plain-text case, where the copy carries its own newlines. Markup
           children lay out with their own elements, so collapsing source whitespace is what's wanted. -->
      <div
        class="mt-0.5 break-words text-sm text-neutral-200 {children
          ? ''
          : 'whitespace-pre-wrap'}"
      >
        {#if children}{@render children()}{:else}{content}{/if}
      </div>
    </div>
  </div>
</div>
