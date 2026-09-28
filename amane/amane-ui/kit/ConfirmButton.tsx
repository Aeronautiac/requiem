// A button whose action needs a second, deliberate click: it opens a small dialog saying what will
// happen, and only the dialog's button acts. The one confirm pattern in the app; nothing uses the
// browser's own confirm().
import type { ComponentProps, ReactNode } from "react";
import { useState } from "react";
import { Button } from "./Button.tsx";
import { Modal } from "./Modal.tsx";

export function ConfirmButton({
  title,
  body,
  confirm,
  onConfirm,
  variant = "danger",
  size = "sm",
  disabled,
  children,
}: {
  title: string;
  // What will happen, and anything that can't be undone.
  body: ReactNode;
  // The dialog's action button label, e.g. "End game".
  confirm: string;
  // May be async; the dialog closes once it settles.
  onConfirm: () => void | Promise<void>;
  variant?: ComponentProps<typeof Button>["variant"];
  size?: ComponentProps<typeof Button>["size"];
  disabled?: boolean;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);

  async function act() {
    setBusy(true);
    await onConfirm();
    setBusy(false);
    setOpen(false);
  }

  return (
    <>
      <Button variant={variant} size={size} disabled={disabled} onClick={() => setOpen(true)}>
        {children}
      </Button>
      <Modal
        open={open}
        onClose={() => setOpen(false)}
        title={title}
        footer={
          <>
            <Button variant="ghost" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button variant={variant} disabled={busy} onClick={() => void act()}>
              {confirm}
            </Button>
          </>
        }
      >
        <div className="text-sm text-ink">{body}</div>
      </Modal>
    </>
  );
}
