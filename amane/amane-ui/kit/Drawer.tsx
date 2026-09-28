// A side panel that slides over the page on a narrow screen. On a wide screen the same content
// sits in a rail instead, so the drawer is only mounted below the breakpoint (see GameScreen).
import type { ReactNode } from "react";

export function Drawer({
  side,
  open,
  onClose,
  children,
}: {
  side: "left" | "right";
  open: boolean;
  onClose: () => void;
  children: ReactNode;
}) {
  if (!open) return null;
  return (
    <div className={`fixed inset-0 z-40 flex ${side === "left" ? "justify-start" : "justify-end"}`}>
      <button
        type="button"
        aria-label="Close panel"
        className="absolute inset-0 bg-backdrop"
        onClick={onClose}
      />
      <div
        className={`relative flex h-full w-[85%] max-w-sm flex-col border-edge bg-surface ${side === "left" ? "border-r" : "border-l"}`}
      >
        {children}
      </div>
    </div>
  );
}
