<script lang="ts">
  // The only button in the kit. Variants exist so a call site names its INTENT ("danger") rather
  // than a colour — a per-site `class="bg-red-600 hover:bg-red-700"` is exactly what makes a theme
  // change a hunt. `class` is still forwarded for layout (width, margin), not for colour.
  import type { Snippet } from "svelte";

  type Variant = "default" | "danger" | "ghost";
  type Size = "sm" | "md";

  let {
    variant = "default",
    size = "md",
    type = "button",
    disabled = false,
    class: klass = "",
    onclick,
    children,
  }: {
    variant?: Variant;
    size?: Size;
    type?: "button" | "submit";
    disabled?: boolean;
    class?: string;
    onclick?: (event: MouseEvent) => void;
    children: Snippet;
  } = $props();

  const VARIANTS: Record<Variant, string> = {
    default: "bg-accent text-accent-ink hover:opacity-90",
    danger: "bg-danger text-danger-ink hover:opacity-90",
    ghost: "border border-edge bg-panel text-ink hover:bg-raised",
  };

  const SIZES: Record<Size, string> = {
    sm: "h-8 px-3 text-sm",
    md: "h-9 px-4 text-sm",
  };
</script>

<button
  {type}
  {disabled}
  {onclick}
  class="inline-flex items-center justify-center gap-2 rounded-md font-medium
         transition-opacity disabled:pointer-events-none disabled:opacity-50
         {VARIANTS[variant]} {SIZES[size]} {klass}"
>
  {@render children()}
</button>
