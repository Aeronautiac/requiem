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

export function FlashLine({ flash }: { flash: Flash }) {
  if (!flash.message) return null;
  return (
    <p className={`text-sm ${flash.message.kind === "error" ? "text-danger-text" : "text-ok-text"}`}>
      {flash.message.text}
    </p>
  );
}
