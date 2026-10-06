// A short-lived line under a form: how a call came out. Every action's reply is shown this way, so
// a failure is never silent and never an alert box.
import type { Reply } from "amane-client/host.ts";
import { execErrorText } from "amane-client/text.ts";
import { useEffect, useState } from "react";

type Message = { kind: "error" | "ok"; text: string };

export type Flash = {
  message: Message | null;
  error: (text: string) => void;
  ok: (text: string) => void;
  // Shows a failed reply's error, or `success` when it went through. Returns whether it did.
  reply: <T>(reply: Reply<T>, success?: string) => reply is { ok: true; value: T };
};

export function useFlash(ms = 3000): Flash {
  const [message, setMessage] = useState<Message | null>(null);
  useEffect(() => {
    if (!message) return;
    const id = setTimeout(() => setMessage(null), ms);
    return () => clearTimeout(id);
  }, [message, ms]);

  return {
    message,
    error: (text) => setMessage({ kind: "error", text }),
    ok: (text) => setMessage({ kind: "ok", text }),
    reply: <T,>(reply: Reply<T>, success?: string): reply is { ok: true; value: T } => {
      if (!reply.ok) setMessage({ kind: "error", text: execErrorText(reply.error) });
      else if (success) setMessage({ kind: "ok", text: success });
      return reply.ok;
    },
  };
}

// `floating` is for a toolbar, where a line in the flow would push the buttons around: the line
// sits over the bottom of the screen instead, clear of the bottom bar, and takes no space. `fixed`,
// so it shows the same from the bar and from inside a dialog's scrolling body.
const FLOATING =
  "pointer-events-none fixed inset-x-4 bottom-14 z-50 mx-auto w-fit max-w-md whitespace-normal border border-edge bg-panel px-3 py-2 text-center";

export function FlashLine({ flash, floating = false }: { flash: Flash; floating?: boolean }) {
  if (!flash.message) return null;
  return (
    <p
      className={`text-sm ${flash.message.kind === "error" ? "text-danger-text" : "text-ok-text"} ${floating ? FLOATING : ""}`}
    >
      {flash.message.text}
    </p>
  );
}
