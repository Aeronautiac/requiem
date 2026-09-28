import { useState } from "react";
import { slotKeyFromString } from "amane-client/bindings.ts";
import { abilityRequest } from "amane-client/queries/abilities.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { useGameUi } from "../game_ui.ts";
import { PlayerSelect } from "../PlayerSelect.tsx";
import type { AbilityUiProps } from "./registry.ts";

export function TrueNameRerollAbility({ abilityId, orgId, onDone }: AbilityUiProps) {
  const session = useSession();
  const ui = useGameUi();
  const [target, setTarget] = useState("");
  const flash = useFlash();

  async function reroll() {
    if (!target) {
      flash.error("Pick whose name to reroll.");
      return;
    }
    const reply = await session.submit_action(
      abilityRequest(
        ui.viewer,
        abilityId,
        orgId,
        // The server draws the new name and replaces this before the engine sees it, the same way
        // it replaces the timestamp — naming yourself is not a thing a client gets to do. The field
        // is carried anyway because the action has to be self-contained to replay.
        { TrueNameReroll: { target: slotKeyFromString(target), true_name: "" } },
        Date.now(),
      ),
    );
    if (flash.reply(reply)) onDone();
  }

  return (
    <div className="flex flex-col gap-3">
      <PlayerSelect value={target} onChange={setTarget} placeholder="Whose name to reroll" />
      <Button onClick={reroll}>Reroll name</Button>
      <FlashLine flash={flash} />
    </div>
  );
}
