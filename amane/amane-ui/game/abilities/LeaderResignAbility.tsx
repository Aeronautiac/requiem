import { useState } from "react";
import { slotKeyFromString } from "amane-client/bindings.ts";
import { abilityRequest } from "amane-client/queries/abilities.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { useGameUi, useView } from "../game_ui.ts";
import { PlayerSelect } from "../PlayerSelect.tsx";
import type { AbilityUiProps } from "./registry.ts";

export function LeaderResignAbility({ abilityId, orgId, onDone }: AbilityUiProps) {
  const session = useSession();
  const ui = useGameUi();
  const view = useView();

  // Leadership stays inside the org, so only its own members can be named. Whether a successor is
  // required at all depends on the org's transfer policy, which the engine enforces.
  const members = orgId ? view.orgs.get(orgId)?.members : undefined;

  const [successor, setSuccessor] = useState("");
  const flash = useFlash();

  async function resign() {
    const reply = await session.submit_action(
      abilityRequest(
        ui.viewer,
        abilityId,
        orgId,
        { LeaderResign: { successor: successor ? slotKeyFromString(successor) : null } },
        Date.now(),
      ),
    );
    if (flash.reply(reply)) onDone();
  }

  return (
    <div className="flex flex-col gap-3">
      <PlayerSelect
        value={successor}
        onChange={setSuccessor}
        placeholder="Successor (if your org requires one)"
        ids={members}
      />
      <Button variant="ghost" onClick={resign}>
        Resign leadership
      </Button>
      <FlashLine flash={flash} />
    </div>
  );
}
