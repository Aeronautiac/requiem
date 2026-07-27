<script lang="ts" generics="T extends string">
  // A styled native <select>. Both pickers in the app are "choose one of a list of strings", which
  // is exactly what the native element is, and taking it means keyboard navigation, type-ahead,
  // mobile pickers and screen-reader support come free instead of being reimplemented.
  //
  // Options are data rather than child components: neither call site needs per-option markup, and a
  // list is far easier to feed from derived state than a slot is.
  let {
    value = $bindable(),
    options,
    disabled = false,
    class: klass = "",
  }: {
    value: T;
    options: { value: T; label: string }[];
    disabled?: boolean;
    class?: string;
  } = $props();
</script>

<select
  bind:value
  {disabled}
  class="h-9 rounded-md border border-edge bg-panel px-3 text-sm text-ink
         focus:outline-none focus:ring-1 focus:ring-edge disabled:opacity-50 {klass}"
>
  {#each options as option (option.value)}
    <option value={option.value}>{option.label}</option>
  {/each}
</select>
