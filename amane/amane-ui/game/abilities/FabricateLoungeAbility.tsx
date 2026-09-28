import { useState } from "react";
import { slotKeyFromString } from "amane-client/bindings.ts";
import { abilityRequest } from "amane-client/queries/abilities.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { useGameUi } from "../game_ui.ts";
import { PlayerSelect } from "../PlayerSelect.tsx";
import type { AbilityUiProps } from "./registry.ts";

export function FabricateLoungeAbility({ abilityId, orgId, onDone }: AbilityUiProps) {
  const session = useSession();
  const ui = useGameUi();
  const [contactor, setContactor] = useState(""); // apparent initiator of the fake conversation
  const [contacted, setContacted] = useState(""); // apparent recipient
  const flash = useFlash();

  async function fabricate() {
    if (!contactor || !contacted) {
      flash.error("Pick both players.");
      return;
    }
    if (contactor === contacted) {
      flash.error("Pick two different players.");
      return;
    }
    const reply = await session.submit_action(
      abilityRequest(
        ui.viewer,
        abilityId,
        orgId,
        {
          FabricateLounge: {
            contactor_id: slotKeyFromString(contactor),
            contacted_id: slotKeyFromString(contacted),
          },
        },
        Date.now(),
      ),
    );
    if (flash.reply(reply)) onDone();
  }

  return (
    <div className="flex flex-col gap-3">
      <PlayerSelect value={contactor} onChange={setContactor} placeholder="First player" />
      <PlayerSelect value={contacted} onChange={setContacted} placeholder="Second player" />
      <Button onClick={fabricate}>Fabricate lounge</Button>
      <FlashLine flash={flash} />
    </div>
  );
}
