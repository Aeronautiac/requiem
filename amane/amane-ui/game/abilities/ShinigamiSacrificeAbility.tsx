import { useState } from "react";
import { slotKeyFromString } from "amane-client/bindings.ts";
import { abilityRequest } from "amane-client/queries/abilities.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { useGameUi, useView } from "../game_ui.ts";
import { PlayerSelect } from "../PlayerSelect.tsx";
import type { AbilityUiProps } from "./registry.ts";

export function ShinigamiSacrificeAbility({ abilityId, orgId, onDone }: AbilityUiProps) {
  const session = useSession();
  const ui = useGameUi();
  const view = useView();

  // Only an OG member can be spent, so offer the org's roster rather than every player. The
  // engine still checks it — this is UX. Undefined (no org) leaves PlayerSelect unfiltered.
  const members = orgId ? view.orgs.get(orgId)?.members : undefined;

  const [sacrifice, setSacrifice] = useState("");
  const [nameTarget, setNameTarget] = useState("");
  const flash = useFlash();

  async function sacrificeMember() {
    if (!sacrifice || !nameTarget) {
      flash.error("Pick who to spend and whose name to buy.");
      return;
    }
    const reply = await session.submit_action(
      abilityRequest(
        ui.viewer,
        abilityId,
        orgId,
        {
          ShinigamiSacrifice: {
            sacrifice: slotKeyFromString(sacrifice),
            name_target: slotKeyFromString(nameTarget),
          },
        },
        Date.now(),
      ),
    );
    if (flash.reply(reply)) onDone();
  }

  return (
    <div className="flex flex-col gap-3">
      <span className="text-xs uppercase tracking-wide text-ink-dim">Spend</span>
      <PlayerSelect value={sacrifice} onChange={setSacrifice} placeholder="Member to sacrifice" ids={members} />
      <span className="text-xs uppercase tracking-wide text-ink-dim">For the name of</span>
      <PlayerSelect value={nameTarget} onChange={setNameTarget} placeholder="Whose name to buy" />
      <Button variant="danger" onClick={sacrificeMember}>
        Make the trade
      </Button>
      <FlashLine flash={flash} />
    </div>
  );
}
