// Every dialog in the app. Built on <dialog> + showModal(), which gives the focus trap, Esc to
// close, an inert page behind and ::backdrop without a library.
//
// THE SCROLL RULE lives here: the box is a column capped at the viewport's height, the header and
// footer never scroll, and only the body does. A modal never scrolls as a whole.
//
// A body with one long list inside a fixed form (the key manager) passes `scroll="content"`: the
// body then doesn't scroll, and is a flex column the content fills. The list takes
// `min-h-0 flex-1 overflow-y-auto` and scrolls on its own while the rest stays put.
import type { ReactNode } from "react";
import { useEffect, useRef } from "react";

type Props = {
  open: boolean;
  onClose: () => void;
  title: ReactNode;
  // Pinned under the body, e.g. the submit buttons.
  footer?: ReactNode;
  // Max width. Full width on a phone either way.
  width?: string;
  // Who scrolls: the whole body (the default), or a region the content marks itself.
  scroll?: "body" | "content";
  children: ReactNode;
};

export function Modal({ open, onClose, title, footer, width = "28rem", scroll = "body", children }: Props) {
  const ref = useRef<HTMLDialogElement>(null);

  // Drive the element from `open`, guarded both ways: showModal() on an open dialog throws, and
  // close() on a closed one fires a spurious close event.
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (open && !el.open) el.showModal();
    else if (!open && el.open) el.close();
  }, [open]);

  return (
    <dialog
      ref={ref}
      // Esc and backdrop clicks close the element itself; report it so `open` follows.
      onClose={onClose}
      // The box is a child of <dialog>, so a click on the element ITSELF is on the backdrop.
      onClick={(event) => {
        if (event.target === ref.current) ref.current?.close();
      }}
      // `whitespace-normal`: a dialog renders over the page but is declared inside whatever opened it,
      // and inherits its text layout. A `whitespace-nowrap` toolbar would otherwise stop every
      // paragraph in the dialog from wrapping.
      className="m-auto max-h-[calc(100dvh-2rem)] w-[calc(100%-2rem)] whitespace-normal border border-edge bg-panel p-0 text-ink"
      style={{ maxWidth: width }}
    >
      {open && (
        <div className="flex max-h-[calc(100dvh-2rem)] flex-col">
          <div className="flex shrink-0 items-center gap-2 border-b border-edge px-4 py-3">
            <h2 className="min-w-0 flex-1 truncate text-base font-semibold">{title}</h2>
            <button
              type="button"
              aria-label="Close"
              className="px-2 text-ink-dim hover:text-ink"
              onClick={onClose}
            >
              ✕
            </button>
          </div>
          <div className={`min-h-0 flex-1 p-4 ${scroll === "body" ? "overflow-y-auto" : "flex flex-col"}`}>
            {children}
          </div>
          {footer && (
            <div className="flex shrink-0 flex-wrap justify-end gap-2 border-t border-edge px-4 py-3">
              {footer}
            </div>
          )}
        </div>
      )}
    </dialog>
  );
}
