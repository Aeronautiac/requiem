import { useState } from "react";
import type { Role } from "amane-client/bindings.ts";
import { slotKeyFromString } from "amane-client/bindings.ts";
import { abilityRequest } from "amane-client/queries/abilities.ts";
import { roleLabel } from "amane-client/text.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { Select } from "../../kit/Input.tsx";
import { useGameUi } from "../game_ui.ts";
import { PlayerSelect } from "../PlayerSelect.tsx";
import { ROLES } from "amane-client/bindings.ts";
import type { AbilityUiProps } from "./registry.ts";

export function FalseAnonymousContactAbility({ abilityId, orgId, onDone }: AbilityUiProps) {
  const session = useSession();
  const ui = useGameUi();
  const [target, setTarget] = useState("");
  const [role, setRole] = useState<Role>(ROLES[0]);
  const flash = useFlash();

  async function contact() {
    if (!target) {
      flash.error("Pick a target.");
      return;
    }
    const reply = await session.submit_action(
      abilityRequest(
        ui.viewer,
        abilityId,
        orgId,
        { FalseAnonymousContact: { target: slotKeyFromString(target), role } },
        Date.now(),
      ),
    );
    if (flash.reply(reply)) onDone();
  }

  return (
    <div className="flex flex-col gap-3">
      <PlayerSelect value={target} onChange={setTarget} placeholder="Target" />
      <label className="flex flex-col gap-1 text-xs text-ink-dim">
        Role to pose as
        <Select value={role} onChange={(e) => setRole(e.target.value as Role)} options={ROLES.map((r) => ({ value: r, label: roleLabel(r) }))} />
      </label>
      <Button onClick={contact}>Contact as {roleLabel(role)}</Button>
      <FlashLine flash={flash} />
    </div>
  );
}
