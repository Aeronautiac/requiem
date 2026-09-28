import { abilityRequest } from "amane-client/queries/abilities.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { useGameUi } from "../game_ui.ts";
import type { AbilityUiProps } from "./registry.ts";

export function ShinigamiEyeDealAbility({ abilityId, orgId, onDone }: AbilityUiProps) {
  const session = useSession();
  const ui = useGameUi();
  const flash = useFlash();

  async function makeDeal() {
    const reply = await session.submit_action(
      abilityRequest(ui.viewer, abilityId, orgId, { ShinigamiEyeDeal: {} }, Date.now()),
    );
    if (flash.reply(reply)) onDone();
  }

  return (
    <div className="flex flex-col gap-3">
      <p className="text-sm text-ink-dim">
        Consumes this ability and grants you True Name Reveal. The world is told someone of your role has made
        the deal.
      </p>
      <Button variant="ghost" onClick={makeDeal}>
        Make the deal
      </Button>
      <FlashLine flash={flash} />
    </div>
  );
}
