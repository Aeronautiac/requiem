import { useState } from "react";
import { slotKeyFromString } from "amane-client/bindings.ts";
import { abilityRequest } from "amane-client/queries/abilities.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { useGameUi } from "../game_ui.ts";
import { PlayerSelect } from "../PlayerSelect.tsx";
import type { AbilityUiProps } from "./registry.ts";

export function ProsecuteAbility({ abilityId, orgId, onDone }: AbilityUiProps) {
  const session = useSession();
  const ui = useGameUi();
  const [target, setTarget] = useState("");
  const flash = useFlash();

  async function prosecute() {
    if (!target) {
      flash.error("Pick a defendant.");
      return;
    }
    const reply = await session.submit_action(
      abilityRequest(ui.viewer, abilityId, orgId, { Prosecute: { target: slotKeyFromString(target) } }, Date.now()),
    );
    if (flash.reply(reply)) onDone();
  }

  return (
    <div className="flex flex-col gap-3">
      <PlayerSelect value={target} onChange={setTarget} placeholder="Defendant" />
      <Button onClick={prosecute}>Prosecute</Button>
      <FlashLine flash={flash} />
    </div>
  );
}
