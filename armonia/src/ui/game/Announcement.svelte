<script lang="ts">
  // A highlighted announcement row: a coloured translucent panel with a bold
  // description and a body. Used for deaths, anonymous announcements, etc. — the
  // caller picks the colour and copy per event type. Like Message, this is pure
  // presentation; the parent resolves whatever text goes in.
  //
  // `color` is any CSS colour string (hex, rgb, named). The translucency is applied
  // here — a low-opacity tint over the base plus a solid accent bar — so callers
  // just hand in a solid colour and don't have to think about opacity or Tailwind's
  // class purging (dynamic colours can't be Tailwind classes).
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
    <!-- tint layer: the colour at low opacity over the base background -->
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
