<script lang="ts">
  // A text/number field. `value` is bindable and deliberately `string | number`: the number inputs
  // in the notebook-write form bind to numbers, and forcing them through strings would put a parse
  // at every call site.
  let {
    value = $bindable(),
    type = "text",
    placeholder,
    min,
    disabled = false,
    class: klass = "",
    onkeydown,
  }: {
    value?: string | number;
    type?: "text" | "number";
    placeholder?: string;
    min?: string | number;
    disabled?: boolean;
    class?: string;
    onkeydown?: (event: KeyboardEvent) => void;
  } = $props();
</script>

<input
  {type}
  {placeholder}
  {min}
  {disabled}
  {onkeydown}
  bind:value
  class="h-9 w-full rounded-md border border-edge bg-panel px-3 text-sm text-ink
         placeholder:text-ink-dim focus:outline-none focus:ring-1 focus:ring-edge
         disabled:opacity-50 {klass}"
/>
