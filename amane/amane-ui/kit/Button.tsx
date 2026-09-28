// The only button. A call site names its INTENT ("danger"), never a colour; `className` is for
// layout (width, margin) only.
import type { ButtonHTMLAttributes } from "react";

type Variant = "default" | "danger" | "ghost";
type Size = "sm" | "md";

const VARIANTS: Record<Variant, string> = {
  default: "bg-accent text-accent-ink hover:opacity-90",
  danger: "bg-danger text-danger-ink hover:opacity-90",
  ghost: "border border-edge bg-panel text-ink hover:bg-raised",
};

const SIZES: Record<Size, string> = {
  sm: "h-8 px-3 text-sm",
  md: "h-10 px-4 text-sm",
};

type Props = ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant; size?: Size };

export function Button({ variant = "default", size = "md", type = "button", className = "", ...rest }: Props) {
  return (
    <button
      type={type}
      className={`inline-flex shrink-0 items-center justify-center gap-2 font-medium transition-opacity disabled:pointer-events-none disabled:opacity-50 ${VARIANTS[variant]} ${SIZES[size]} ${className}`}
      {...rest}
    />
  );
}
