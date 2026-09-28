// The generic abilities menu, in the bottom bar. With `orgId` it lists that org's shared abilities
// (dispatching UseOrgAbility instead of UseAbility) — the org panel reuses this component rather
// than duplicating a menu.
import { useState } from "react";
import { listAbilities, prettyAbility } from "amane-client/queries/abilities.ts";
import { abilityDescription, abilityWarning } from "amane-client/text.ts";
import { Button } from "../../kit/Button.tsx";
import { Modal } from "../../kit/Modal.tsx";
import { useView } from "../game_ui.ts";
import { AbilityCard } from "./AbilityCard.tsx";
import { ABILITY_UIS } from "./registry.ts";

export function AbilityMenu({ orgId }: { orgId?: string }) {
  const view = useView();
  const [open, setOpen] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null); // ability instance being configured

  const listed = listAbilities(view, orgId);
  const source = orgId ? view.orgs.get(orgId)?.abilities : view.abilities;
  const selectedAbility = selectedId ? source?.get(selectedId) : undefined;
  const SelectedForm = selectedAbility ? ABILITY_UIS[selectedAbility.name] : undefined;

  function close() {
    setOpen(false);
    setSelectedId(null);
  }

  const title =
    selectedId && SelectedForm && selectedAbility ? (
      <span className="flex items-center gap-2">
        <button type="button" className="text-ink-dim hover:text-ink" onClick={() => setSelectedId(null)} aria-label="Back to abilities">
          ←
        </button>
        {prettyAbility(selectedAbility.name)}
      </span>
    ) : orgId ? (
      "Org abilities"
    ) : (
      "Abilities"
    );

  return (
    <>
      <Button variant="ghost" size="sm" onClick={() => setOpen(true)}>
        {orgId ? "Org abilities" : "Abilities"}
      </Button>

      <Modal open={open} onClose={close} title={title}>
        {selectedId && SelectedForm && selectedAbility ? (
          <div className="flex flex-col gap-3">
            {abilityDescription(selectedAbility.name) && (
              <p className="text-sm text-ink-dim">{abilityDescription(selectedAbility.name)}</p>
            )}
            {abilityWarning(selectedAbility.name) && (
              <p className="text-sm text-danger-text">{abilityWarning(selectedAbility.name)}</p>
            )}
            <SelectedForm abilityId={selectedId} orgId={orgId} onDone={close} />
          </div>
        ) : (
          <div className="flex flex-col gap-2">
            {listed.map((ab) => (
              <AbilityCard
                key={ab.id}
                name={ab.name}
                successUsages={ab.successUsages}
                failureUsages={ab.failureUsages}
                resets={ab.resets}
                baseReset={ab.baseReset}
                unlimited={ab.unlimited}
                hasUi={ABILITY_UIS[ab.name] != null}
                requirements={ab.requirements}
                onUse={() => setSelectedId(ab.id)}
              />
            ))}
            {listed.length === 0 && <p className="py-2 text-sm text-ink-dim">No abilities.</p>}
          </div>
        )}
      </Modal>
    </>
  );
}
