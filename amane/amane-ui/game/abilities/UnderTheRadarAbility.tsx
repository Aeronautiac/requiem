import { abilityRequest } from "amane-client/queries/abilities.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { useGameUi } from "../game_ui.ts";
import type { AbilityUiProps } from "./registry.ts";

export function UnderTheRadarAbility({ abilityId, orgId, onDone }: AbilityUiProps) {
  const session = useSession();
  const ui = useGameUi();
  const flash = useFlash();

  async function goDark() {
    const reply = await session.submit_action(
      abilityRequest(ui.viewer, abilityId, orgId, { UnderTheRadar: {} }, Date.now()),
    );
    if (flash.reply(reply)) onDone();
  }

  return (
    <div className="flex flex-col gap-3">
      <Button variant="ghost" onClick={goDark}>
        Go under the radar
      </Button>
      <FlashLine flash={flash} />
    </div>
  );
}
