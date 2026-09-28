import { useState } from "react";
import { abilityRequest } from "amane-client/queries/abilities.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { Input } from "../../kit/Input.tsx";
import { useGameUi } from "../game_ui.ts";
import type { AbilityUiProps } from "./registry.ts";

export function TapInAbility({ abilityId, orgId, onDone }: AbilityUiProps) {
  const session = useSession();
  const ui = useGameUi();

  // A raw number, not a picker: the whole ability is guessing which contact channel a number
  // belongs to, so there is nothing to choose from. A channel you could pick is one you are
  // already in.
  const [contactId, setContactId] = useState("");
  const flash = useFlash();

  async function tap() {
    const id = Number(contactId);
    if (contactId.trim() === "" || !Number.isInteger(id) || id < 0) {
      flash.error("Enter a contact number.");
      return;
    }
    const reply = await session.submit_action(
      abilityRequest(ui.viewer, abilityId, orgId, { TapIn: { contact_id: id } }, Date.now()),
    );
    if (flash.reply(reply)) onDone();
  }

  return (
    <div className="flex flex-col gap-3">
      <Input
        type="number"
        min="0"
        step="1"
        value={contactId}
        onChange={(e) => setContactId(e.target.value)}
        placeholder="Contact number"
      />
      <Button onClick={tap}>Tap in</Button>
      <FlashLine flash={flash} />
    </div>
  );
}
