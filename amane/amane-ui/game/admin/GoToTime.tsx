// The host's time-travel control: jump the game clock to an arbitrary instant, forward or back.
// This is the game task's own mechanic (GoToTime) — it works entirely on the server's sandboxed
// clock, so the client just names a target and the response confirms the jump was issued.
import { useState } from "react";
import { formatTime } from "amane-client/text.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import type { Flash } from "../../kit/Flash.tsx";
import { Input } from "../../kit/Input.tsx";
import { Modal } from "../../kit/Modal.tsx";

// Parse explicit time units in any order: "2h30m", "30m10s", "500ms", "1d2h", "1h:30m" (the
// punctuation is ignored). A plain number of digits is treated as seconds. Returns game-time ms, or
// null if the input is not a shape this can read.
function parse(text: string): number | null {
  const s = text.trim().toLowerCase().replace(/[\s,:]+/g, "");
  if (!s) return null;
  if (/^\d+$/.test(s)) return Number(s) * 1000; // bare number of seconds
  let total = 0;
  let rest = s;
  const re = /(\d+(?:\.\d+)?)(ms|s|m|h|d)/;
  while (re.test(rest)) {
    const m = re.exec(rest)!;
    const val = parseFloat(m[1]);
    const mult = m[2] === "ms" ? 1 : m[2] === "s" ? 1000 : m[2] === "m" ? 60000 : m[2] === "h" ? 3600000 : 86400000;
    total += val * mult;
    rest = rest.slice(0, m.index) + rest.slice(m.index + m[0].length);
  }
  // Any leftover characters mean malformed units — reject rather than guess.
  if (rest !== "") return null;
  return total;
}

export function GoToTime({ flash }: { flash: Flash }) {
  const session = useSession();
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [confirming, setConfirming] = useState(false);

  const targetMs = parse(text);
  const ok = targetMs !== null;

  // Confirm rather than jump straight in: a backward jump rewrites the timeline and can cut
  // connections, so it deserves a deliberate second click.
  function requestConfirm() {
    if (ok) setConfirming(true);
  }

  async function confirm() {
    if (targetMs === null) return;
    setConfirming(false);
    setBusy(true);
    const reply = await session.submit_control({ Meta: { GoToTime: { time: targetMs } } });
    setBusy(false);
    flash.reply(reply, "Clock moved.");
  }

  return (
    <div className="flex items-center gap-1.5 whitespace-nowrap">
      <Input
        type="text"
        placeholder="xxh:xxm:xxs:xxms"
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") requestConfirm();
        }}
        className="w-32"
      />
      <Button variant="ghost" size="sm" disabled={!ok || busy} onClick={requestConfirm}>
        Set time
      </Button>

      <Modal
        open={confirming}
        onClose={() => setConfirming(false)}
        title="Set game time"
        footer={
          <>
            <Button variant="ghost" size="sm" onClick={() => setConfirming(false)}>
              Cancel
            </Button>
            <Button size="sm" disabled={busy} onClick={confirm}>
              Set time
            </Button>
          </>
        }
      >
        <p className="text-sm text-ink-dim">
          Move the clock to <span className="font-mono text-ink">{targetMs === null ? "" : formatTime(targetMs)}</span>. Going backward
          rewrites the timeline and can disconnect anyone whose key is no longer valid.
        </p>
      </Modal>
    </div>
  );
}
