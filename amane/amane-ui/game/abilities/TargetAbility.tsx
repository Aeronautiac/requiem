// Every ability whose whole form is "pick one player": the registry describes each one in a line
// (its button, and how the picked player becomes the ability's arguments) instead of a file.
import type { AbilityBehaviour, ActorKey } from "amane-client/bindings.ts";
import { slotKeyFromString } from "amane-client/bindings.ts";
import { abilityRequest } from "amane-client/queries/abilities.ts";
import type { ComponentType } from "react";
import { useState } from "react";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { useGameUi } from "../game_ui.ts";
import { PlayerSelect } from "../PlayerSelect.tsx";
import type { AbilityUiProps } from "./registry.ts";

// `label` is the button, e.g. "Fire"; `build` turns the picked player into the ability's arguments;
// an irreversible ability is `danger` and gets the danger button.
export function targetAbility(
  label: string,
  build: (target: ActorKey) => AbilityBehaviour,
  danger = false,
): ComponentType<AbilityUiProps> {
  return function TargetAbility({ abilityId, orgId, onDone }: AbilityUiProps) {
    const session = useSession();
    const ui = useGameUi();
    const [target, setTarget] = useState("");
    const flash = useFlash();

    async function use() {
      if (!target) {
        flash.error("Pick a target.");
        return;
      }
      const reply = await session.submit_action(
        abilityRequest(ui.viewer, abilityId, orgId, build(slotKeyFromString(target)), Date.now()),
      );
      if (flash.reply(reply)) onDone();
    }

    return (
      <div className="flex flex-col gap-3">
        <PlayerSelect value={target} onChange={setTarget} placeholder="Target" />
        <Button variant={danger ? "danger" : "default"} onClick={() => void use()}>
          {label}
        </Button>
        <FlashLine flash={flash} />
      </div>
    );
  };
}
