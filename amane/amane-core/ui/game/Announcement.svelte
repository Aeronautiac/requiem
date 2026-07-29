<script lang="ts">
  // Pure presentation; the parent resolves the text and picks the colour per event type.
  //
  // `color` is any CSS colour string. The translucency is applied here so callers hand in a solid
  // colour and don't have to think about Tailwind's class purging — dynamic colours cannot be
  // Tailwind classes.
  interface Props {
    color: string;
    description: string;
    content: string;
  }
  let { color, description, content }: Props = $props();
</script>

<div class="px-4 py-1">
  <div
    class="relative overflow-hidden rounded-md border-l-2 px-3 py-2"
    style="border-color: {color}"
  >
    <div
      class="pointer-events-none absolute inset-0"
      style="background-color: {color}; opacity: 0.12"
    ></div>

    <div class="relative">
      <div
        class="text-xs font-semibold uppercase tracking-wide"
        style="color: {color}"
      >
        {description}
      </div>
      <div class="mt-0.5 whitespace-pre-wrap break-words text-sm text-neutral-200">
        {content}
      </div>
    </div>
  </div>
</div>
