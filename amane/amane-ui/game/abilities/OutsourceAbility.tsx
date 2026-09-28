import { useState } from "react";
import { slotKeyFromString } from "amane-client/bindings.ts";
import { abilityRequest } from "amane-client/queries/abilities.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { useGameUi } from "../game_ui.ts";
import { PlayerSelect } from "../PlayerSelect.tsx";
import type { AbilityUiProps } from "./registry.ts";

export function OutsourceAbility({ abilityId, orgId, onDone }: AbilityUiProps) {
  const session = useSession();
  const ui = useGameUi();
  const [invitee, setInvitee] = useState("");
  const [defendant, setDefendant] = useState("");
  const flash = useFlash();

  async function run() {
    if (!invitee || !defendant) {
      flash.error("Pick who prosecutes and who is prosecuted.");
      return;
    }
    const reply = await session.submit_action(
      abilityRequest(
        ui.viewer,
        abilityId,
        orgId,
        { Outsource: { invitee: slotKeyFromString(invitee), defendant: slotKeyFromString(defendant) } },
        Date.now(),
      ),
    );
    if (flash.reply(reply)) onDone();
  }

  return (
    <div className="flex flex-col gap-3">
      <span className="text-xs uppercase tracking-wide text-ink-dim">Prosecutor</span>
      <PlayerSelect value={invitee} onChange={setInvitee} placeholder="Who to bring in and set prosecuting" />
      <span className="text-xs uppercase tracking-wide text-ink-dim">Defendant</span>
      <PlayerSelect value={defendant} onChange={setDefendant} placeholder="Who they prosecute" />
      <Button onClick={run}>Outsource prosecution</Button>
      <FlashLine flash={flash} />
    </div>
  );
}
