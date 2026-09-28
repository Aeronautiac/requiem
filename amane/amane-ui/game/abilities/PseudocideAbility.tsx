import { useState } from "react";
import type { ActorKey, OrgMemberView, Role } from "amane-client/bindings.ts";
import { slotKeyFromString } from "amane-client/bindings.ts";
import { abilityRequest } from "amane-client/queries/abilities.ts";
import { orgDisplayName, roleLabel } from "amane-client/text.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { Input, Select } from "../../kit/Input.tsx";
import { useGameUi, useView } from "../game_ui.ts";
import { MentionInput } from "../mentions/MentionInput.tsx";
import { PlayerSelect } from "../PlayerSelect.tsx";
import { ROLES } from "amane-client/bindings.ts";
import type { AbilityUiProps } from "./registry.ts";

export function PseudocideAbility({ abilityId, orgId, onDone }: AbilityUiProps) {
  const session = useSession();
  const ui = useGameUi();
  const view = useView();

  const [target, setTarget] = useState("");
  const [trueName, setTrueName] = useState("");
  const [deathMessage, setDeathMessage] = useState("");
  const [role, setRole] = useState<Role>(ROLES[0]);
  const [notebookTransferred, setNotebookTransferred] = useState(false);
  const [abilityTransferred, setAbilityTransferred] = useState(false);
  // Fabricated affiliations to show on the fake death. Every org rides the data viewport, so any
  // of them can be named; each entry carries whatever leader/OG standing the faker wants seen.
  // Keyed by org actor key; absence means that org is not part of the lie.
  const [orgReveal, setOrgReveal] = useState<Record<string, OrgMemberView>>({});
  const flash = useFlash();

  function toggleOrg(key: string) {
    setOrgReveal((prev) => {
      if (prev[key]) {
        const next = { ...prev };
        delete next[key];
        return next;
      }
      return { ...prev, [key]: { leader: false, og: false } };
    });
  }

  function setOrgFlag(key: string, flag: "leader" | "og", value: boolean) {
    setOrgReveal((prev) => (prev[key] ? { ...prev, [key]: { ...prev[key], [flag]: value } } : prev));
  }

  async function fakeDeath() {
    if (!target) {
      flash.error("Pick whose death to fake.");
      return;
    }
    if (!trueName.trim()) {
      flash.error("A true name is required.");
      return;
    }
    const reply = await session.submit_action(
      abilityRequest(
        ui.viewer,
        abilityId,
        orgId,
        {
          Pseudocide: {
            target_id: slotKeyFromString(target),
            true_name: trueName,
            // Blank means "use the default death message" — send None, not an empty string.
            death_message: deathMessage.trim() ? deathMessage : null,
            role,
            orgs: Object.entries(orgReveal).map(
              ([key, v]) => [slotKeyFromString(key), v] as [ActorKey, OrgMemberView],
            ),
            notebook_transferred: notebookTransferred,
            ability_transferred: abilityTransferred,
          },
        },
        Date.now(),
      ),
    );
    if (flash.reply(reply)) onDone();
  }

  return (
    <div className="flex flex-col gap-3">
      <label className="flex flex-col gap-1 text-xs text-ink-dim">
        Target
        <PlayerSelect value={target} onChange={setTarget} placeholder="Whose death to fake" />
      </label>

      <label className="flex flex-col gap-1 text-xs text-ink-dim">
        True name revealed
        <Input value={trueName} onChange={(e) => setTrueName(e.target.value)} placeholder="True name" />
      </label>

      <div className="flex flex-col gap-1 text-xs text-ink-dim">
        <span>Death message</span>
        <MentionInput value={deathMessage} onChange={setDeathMessage} placeholder="Announced on death" />
      </div>

      <label className="flex flex-col gap-1 text-xs text-ink-dim">
        Role revealed
        <Select
          value={role}
          onChange={(e) => setRole(e.target.value as Role)}
          options={ROLES.map((r) => ({ value: r, label: roleLabel(r) }))}
        />
      </label>

      <div className="flex flex-col gap-1 text-xs text-ink-dim">
        Affiliations revealed
        <div className="flex flex-col gap-1 border border-edge bg-panel p-2">
          {[...view.orgs].map(([key, org]) => (
            <div key={key} className="flex items-center gap-2">
              <label className="flex flex-1 items-center gap-2 text-sm text-ink">
                <input type="checkbox" checked={!!orgReveal[key]} onChange={() => toggleOrg(key)} />
                {orgDisplayName(org.name)}
              </label>
              {orgReveal[key] && (
                <>
                  <label className="flex items-center gap-1 text-xs text-ink-dim">
                    <input
                      type="checkbox"
                      checked={orgReveal[key].leader}
                      onChange={(e) => setOrgFlag(key, "leader", e.target.checked)}
                    />
                    leader
                  </label>
                  <label className="flex items-center gap-1 text-xs text-ink-dim">
                    <input
                      type="checkbox"
                      checked={orgReveal[key].og}
                      onChange={(e) => setOrgFlag(key, "og", e.target.checked)}
                    />
                    og
                  </label>
                </>
              )}
            </div>
          ))}
          {view.orgs.size === 0 && <span className="text-ink-dim">No organizations to name.</span>}
        </div>
      </div>

      <label className="flex items-center gap-2 text-sm text-ink">
        <input
          type="checkbox"
          checked={notebookTransferred}
          onChange={(e) => setNotebookTransferred(e.target.checked)}
        />
        Notebook transferred
      </label>
      <label className="flex items-center gap-2 text-sm text-ink">
        <input
          type="checkbox"
          checked={abilityTransferred}
          onChange={(e) => setAbilityTransferred(e.target.checked)}
        />
        Abilities transferred
      </label>

      <Button variant="danger" onClick={fakeDeath}>
        Fake death
      </Button>
      <FlashLine flash={flash} />
    </div>
  );
}
