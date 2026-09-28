// Pass (lend) a notebook to another player. Silent — it just dispatches LendNotebook; no logging or
// channel event. Mirrors the NotebookWrite modal, opened from the notebook channel.
import type { ActionRequest, NotebookKey } from "amane-client/bindings.ts";
import { slotKeyFromString } from "amane-client/bindings.ts";
import { viewActor } from "amane-client/queries/session.ts";
import { useState } from "react";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { Modal } from "../../kit/Modal.tsx";
import { useSession } from "../../hooks.ts";
import { useGameUi } from "../game_ui.ts";
import { PlayerSelect } from "../PlayerSelect.tsx";

export function NotebookPass({ open, onClose, notebookId }: { open: boolean; onClose: () => void; notebookId: NotebookKey }) {
  const session = useSession();
  const ui = useGameUi();
  const flash = useFlash();
  const [target, setTarget] = useState("");

  async function submit() {
    if (!target) {
      flash.error("Pick a player to pass to.");
      return;
    }
    const request: ActionRequest = {
      actor: viewActor(ui.viewer),
      timestamp: Date.now(),
      payload: { LendNotebook: { notebook_id: notebookId, target_id: slotKeyFromString(target) } },
    };
    const reply = await session.submit_action(request);
    if (flash.reply(reply, "Notebook passed.")) {
      setTarget("");
      onClose();
    }
  }

  return (
    <Modal open={open} onClose={onClose} title="Pass Notebook" footer={<Button onClick={submit}>Pass</Button>}>
      <div className="flex flex-col gap-3">
        <PlayerSelect value={target} onChange={setTarget} placeholder="Pass to" />
        <FlashLine flash={flash} />
      </div>
    </Modal>
  );
}
