import { useState } from "react";
import { slotKeyFromString } from "amane-client/bindings.ts";
import { abilityRequest } from "amane-client/queries/abilities.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { Input } from "../../kit/Input.tsx";
import { useGameUi } from "../game_ui.ts";
import { PlayerSelect } from "../PlayerSelect.tsx";
import type { AbilityUiProps } from "./registry.ts";

export function TrueNameInviteAbility({ abilityId, orgId, onDone }: AbilityUiProps) {
  const session = useSession();
  const ui = useGameUi();
  const [target, setTarget] = useState("");
  const [trueName, setTrueName] = useState("");
  const flash = useFlash();

  async function invite() {
    if (!target) {
      flash.error("Pick who to invite.");
      return;
    }
    if (!trueName.trim()) {
      flash.error("Guess their true name.");
      return;
    }
    const reply = await session.submit_action(
      abilityRequest(
        ui.viewer,
        abilityId,
        orgId,
        { TrueNameInvite: { target: slotKeyFromString(target), true_name: trueName.trim() } },
        Date.now(),
      ),
    );
    if (flash.reply(reply)) onDone();
  }

  return (
    <div className="flex flex-col gap-3">
      <PlayerSelect value={target} onChange={setTarget} placeholder="Who to invite" />
      <Input value={trueName} onChange={(e) => setTrueName(e.target.value)} placeholder="Their true name" />
      <Button onClick={invite}>Send invite</Button>
      <FlashLine flash={flash} />
    </div>
  );
}
