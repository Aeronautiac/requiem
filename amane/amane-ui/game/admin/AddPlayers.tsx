import { useState } from "react";
import type { ActionRequest, Role } from "amane-client/bindings.ts";
import { roleLabel } from "amane-client/text.ts";
import { ROLES } from "amane-client/bindings.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { Input, Select } from "../../kit/Input.tsx";
import { Modal } from "../../kit/Modal.tsx";

export function AddPlayers() {
  const session = useSession();
  const [open, setOpen] = useState(false);
  const [trueName, setTrueName] = useState("");
  const [role, setRole] = useState<Role>("Civilian");
  const flash = useFlash();

  async function add() {
    // No display name here: creating the SLOT and saying who is on it are separate facts with
    // separate lifetimes. The new player appears as `player-<slot>` and is named from the player
    // list, which is also where a rename happens later.
    // Blank is how you ask the server for a drawn name: it fills an empty true_name from the
    // reservoir and keeps anything you type.
    const request: ActionRequest = {
      actor: "Admin",
      timestamp: Date.now(),
      payload: { AddPlayer: { true_name: trueName, starting_role: role } },
    };
    const reply = await session.submit_action(request);
    if (flash.reply(reply)) {
      // The drawn name never comes back on the response, so a blank submission cannot be echoed
      // here — the player's own Notifications log is where the name lands.
      flash.ok(trueName ? `Added ${trueName}.` : "Added a player with a drawn name.");
      setTrueName("");
    }
  }

  return (
    <>
      <Button variant="ghost" size="sm" onClick={() => setOpen(true)}>
        Add Players
      </Button>

      <Modal open={open} onClose={() => setOpen(false)} title="Add Players" footer={<Button onClick={add}>Add</Button>}>
        <div className="flex flex-col gap-3">
          <Input value={trueName} onChange={(e) => setTrueName(e.target.value)} placeholder="True Name (blank to draw one)" />
          <Select value={role} onChange={(e) => setRole(e.target.value as Role)} options={ROLES.map((r) => ({ value: r, label: roleLabel(r) }))} />
          <FlashLine flash={flash} />
        </div>
      </Modal>
    </>
  );
}
