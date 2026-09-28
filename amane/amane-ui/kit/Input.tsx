// `ComponentProps<"input">` rather than the bare attribute types, so a `ref` passes straight
// through (React 19 hands `ref` to a function component as an ordinary prop).
import type { ComponentProps } from "react";

// No width here. Each field is full width unless the caller's `className` says otherwise: two
// width utilities on one element don't override each other by order, so the default has to be
// replaced, not added to.
const FIELD =
  "min-w-0 border border-edge bg-panel px-3 text-sm text-ink placeholder:text-ink-dim focus:outline-none focus:ring-1 focus:ring-edge disabled:opacity-50";

export function Input({ className = "w-full", ...rest }: ComponentProps<"input">) {
  return <input className={`h-9 ${FIELD} ${className}`} {...rest} />;
}

export function TextArea({ className = "w-full", ...rest }: ComponentProps<"textarea">) {
  return <textarea className={`py-2 ${FIELD} ${className}`} {...rest} />;
}

// A styled native <select>: keyboard navigation, type-ahead and the phone's own picker come free.
export function Select({
  options,
  className = "w-full",
  ...rest
}: ComponentProps<"select"> & { options: { value: string; label: string }[] }) {
  return (
    <select className={`h-8 ${FIELD} ${className}`} {...rest}>
      {options.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </select>
  );
}
