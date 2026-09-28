import type { ActionRequest, NotebookKey } from "amane-client/bindings.ts";
import { viewActor } from "amane-client/queries/session.ts";
import { useState } from "react";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { Input } from "../../kit/Input.tsx";
import { Modal } from "../../kit/Modal.tsx";
import { useSession } from "../../hooks.ts";
import { useGameUi } from "../game_ui.ts";
import { MentionInput } from "../mentions/MentionInput.tsx";

export function NotebookWrite({ open, onClose, notebookId }: { open: boolean; onClose: () => void; notebookId: NotebookKey }) {
  const session = useSession();
  const ui = useGameUi();
  const flash = useFlash();

  const [true_name, setTrueName] = useState("");
  const [death_message, setDeathMessage] = useState("");
  const [seconds, setSeconds] = useState(0);
  const [minutes, setMinutes] = useState(0);
  const [hours, setHours] = useState(0);
  const [days, setDays] = useState(0);

  function reset() {
    setTrueName("");
    setDeathMessage("");
    setSeconds(0);
    setMinutes(0);
    setHours(0);
    setDays(0);
  }

  async function submit() {
    if (!true_name.trim()) {
      flash.error("A true name is required.");
      return;
    }
    // delay is a duration in milliseconds (the backend adds it to the current engine time).
    const delay_ms = ((days * 24 + hours) * 60 + minutes) * 60 * 1000 + seconds * 1000;
    const request: ActionRequest = {
      actor: viewActor(ui.viewer),
      timestamp: Date.now(),
      payload: {
        WriteName: {
          true_name: true_name.trim(),
          death_message: death_message.trim() ? death_message.trim() : null,
          notebook_id: notebookId,
          delay: delay_ms,
        },
      },
    };
    const reply = await session.submit_action(request);
    if (flash.reply(reply, "Name written.")) reset();
  }

  return (
    <Modal open={open} onClose={onClose} title="Write Name" footer={<Button onClick={submit}>Write</Button>}>
      <div className="flex flex-col gap-3">
        <Input value={true_name} onChange={(e) => setTrueName(e.target.value)} placeholder="True Name" />
        <MentionInput value={death_message} onChange={setDeathMessage} placeholder="Death Message (optional)" onSubmit={submit} />

        <div>
          <p className="mb-1 text-xs text-ink-dim">Delay</p>
          <div className="grid grid-cols-4 gap-2">
            <label className="flex flex-col gap-1 text-xs text-ink-dim">
              Seconds
              <Input type="number" min="0" value={seconds} onChange={(e) => setSeconds(Number(e.target.value) || 0)} />
            </label>
            <label className="flex flex-col gap-1 text-xs text-ink-dim">
              Minutes
              <Input type="number" min="0" value={minutes} onChange={(e) => setMinutes(Number(e.target.value) || 0)} />
            </label>
            <label className="flex flex-col gap-1 text-xs text-ink-dim">
              Hours
              <Input type="number" min="0" value={hours} onChange={(e) => setHours(Number(e.target.value) || 0)} />
            </label>
            <label className="flex flex-col gap-1 text-xs text-ink-dim">
              Days
              <Input type="number" min="0" value={days} onChange={(e) => setDays(Number(e.target.value) || 0)} />
            </label>
          </div>
        </div>

        <FlashLine flash={flash} />
      </div>
    </Modal>
  );
}
